//! Live-DB regression guard for issue #914 (BIND_ON_ACQUIRE): a bind-on-
//! acquire design granted through the real grant path
//! (`inventory::handle_grant_item`, the choke point behind loot, the
//! content engine's `grant_item` and `gmGiveItem`) must land `bound = true`
//! in `sgw_inventory`, so the send path's existing bound check
//! (`send/escrow.rs::check_source`, proved by
//! `attach_live::live_db_send_rejects_bound_item`) actually has something
//! to catch.
//!
//! Before the fix, every grant path inserted `bound = false`
//! unconditionally regardless of `resources.items.flags`, so a
//! bind-on-acquire mission or vendor reward could be freely mailed to an
//! alt. Reverting the fix makes this test fail: the grant lands unbound,
//! the send succeeds, and the item leaves escrow into the recipient's mail.
//!
//! Sentinels: accounts/players/entities `0x7300_2E00..=0x7300_2E52`, the
//! synthetic item type `0x7300_2F00`.

use std::sync::Arc;
use std::time::Instant;

use super::attach_live::{assert_untouched, attached, reply, setup};
use super::packets::Client;
use super::*;
use crate::base::world_entry::methods::handle_grant_item;
use crate::cell::mail::codes::MailResult;
use crate::test_support::LogCapture;
use cimmeria_entity::inventory::INV_MAIN;

const BASE: i32 = 0x7300_2E00;
const BIND_TYPE_ID: i32 = 0x7300_2F00;
/// `resources.items.flags` bit for BIND_ON_ACQUIRE (`ItemFlags::BIND_ON_ACQUIRE`
/// in `cimmeria_cell_catalog::crafting::constants`; duplicated as a plain
/// literal here so this test file needs no dependency on that crate).
const BIND_ON_ACQUIRE: i32 = 4;

/// `handle_grant_item` enqueues a `cell_event_outbox` row for `entity_id`
/// (via `persist_grant`); with `cell_tx: &None` it can never be dispatched
/// in-process, so it is left permanently undelivered unless this test
/// deletes it. `mail::tests::cleanup` only touches `sgw_gate_mail` and
/// `account` — it never had a reason to know about the outbox before this
/// test called a grant path directly. Left undelivered, the row poisons
/// the outbox's own live-DB tests, which share this slot's database and
/// scan/drain undelivered rows table-wide (issue #914 CI failure).
async fn cleanup_outbox(pool: &PgPool, entity_id: u32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(entity_id as i32)
        .execute(pool)
        .await;
}

async fn insert_bind_on_acquire_type(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(BIND_TYPE_ID)
        .execute(pool)
        .await;
    sqlx::query(
        "INSERT INTO resources.items \
            (item_id, description, name, quality_id, tech_comp, tier, \
             max_stack_size, container_sets, flags) \
         VALUES ($1, '', 'SS-914 bind-on-acquire fixture', 'ITEM_QUALITY_Normal', 0, 1, \
                 1, '{1}', $2)",
    )
    .bind(BIND_TYPE_ID)
    .bind(BIND_ON_ACQUIRE)
    .execute(pool)
    .await
    .expect("insert bind-on-acquire item type");
}

/// A grant of a BIND_ON_ACQUIRE design lands `bound = true`, and the item
/// is then untradeable by mail exactly like a hand-set `bound = true` row
/// (CAT-G-01 / D-SS08).
#[tokio::test]
async fn live_db_a_freshly_granted_bind_on_acquire_item_is_rejected_by_mail_send() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt, entity) = (BASE, BASE + 1, BASE + 2, (BASE + 0x50) as u32);
    cleanup_outbox(&pool, entity).await;
    setup(
        &pool,
        acct,
        (sender, "Ss914BndSend"),
        (rcpt, "Ss914BndRcpt"),
        500,
    )
    .await;
    insert_bind_on_acquire_type(&pool).await;

    let (transport, e2a, conn) = make_state(entity);
    let db_pool = Some(Arc::new(pool.clone()));
    handle_grant_item(
        entity,
        sender,
        BIND_TYPE_ID,
        INV_MAIN,
        1,
        false,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
    let granted_item_id: i32 = sqlx::query_scalar(
        "SELECT item_id FROM sgw_inventory WHERE character_id = $1 AND type_id = $2",
    )
    .bind(sender)
    .bind(BIND_TYPE_ID)
    .fetch_one(&pool)
    .await
    .expect("the grant must have landed a row");
    let bound: bool = sqlx::query_scalar("SELECT bound FROM sgw_inventory WHERE item_id = $1")
        .bind(granted_item_id)
        .fetch_one(&pool)
        .await
        .expect("read the granted row's bound column");
    assert!(
        bound,
        "a BIND_ON_ACQUIRE grant must land bound (SS-914); the mail-send \
         refusal below only proves something once this does"
    );

    let c = Client::new(entity, sender, 54_744, "Ss914BndSend");
    c.op(
        MailOp::Send(attached("Ss914BndRcpt", 0, false, granted_item_id, 1)),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::ItemNotAvailable.code()), "{r:?}");
    assert!(capture
        .find_event(
            tracing::Level::WARN,
            "sendMailMessage refused",
            "item_bound",
        )
        .is_some());
    assert_untouched(
        &pool,
        sender,
        rcpt,
        500,
        &[TestItem {
            item_id: granted_item_id,
            owner: sender,
            container_id: INV_MAIN,
            slot_id: 0,
            stack_size: 1,
            bound: true,
        }],
    )
    .await;

    cleanup(&pool, acct).await;
    cleanup_outbox(&pool, entity).await;
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(BIND_TYPE_ID)
        .execute(&pool)
        .await;
}
