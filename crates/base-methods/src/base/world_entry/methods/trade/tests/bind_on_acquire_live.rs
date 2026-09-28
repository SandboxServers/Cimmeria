//! Live-DB regression guard for issue #914 (BIND_ON_ACQUIRE): a bind-on-
//! acquire design granted through the real grant path
//! (`inventory::handle_grant_item`) must land `bound = true` in
//! `sgw_inventory`, so `lock_items`'s existing bound guard
//! (`commit::live_db_commit_rolls_back_on_bound_item`, which proves the
//! guard itself works on a hand-set `bound = true` row) actually has
//! something to catch on a *granted* item.
//!
//! Before the fix, every grant path inserted `bound = false`
//! unconditionally regardless of `resources.items.flags`, so a
//! bind-on-acquire mission or vendor reward could be freely traded away.
//! Reverting the fix makes this test fail: the grant lands unbound, the
//! trade validation gauntlet finds nothing to reject, and the item changes
//! hands.
//!
//! Sentinels: `fixtures(1800)` (accounts/players/entities), the synthetic
//! item type `0x7000_0FA0`.

use std::sync::Arc;

use cimmeria_entity::inventory::INV_MAIN;

use super::{
    cleanup, fixtures, insert_account_and_player, insert_item, make_state, owner_of,
    tradeable_type_ids,
};
use crate::base::world_entry::methods::handle_grant_item;
use crate::base::world_entry::methods::trade::handle_execute_trade;
use crate::test_support::require_db_or_skip;

const BIND_TYPE_ID: i32 = 0x7000_0FA0;
/// `resources.items.flags` bit for BIND_ON_ACQUIRE (`ItemFlags::BIND_ON_ACQUIRE`
/// in `cimmeria_cell_catalog::crafting::constants`; duplicated as a plain
/// literal here so this test file needs no dependency on that crate).
const BIND_ON_ACQUIRE: i32 = 4;

/// `handle_grant_item` enqueues a `cell_event_outbox` row for `entity_a`
/// (via `persist_grant`); with `cell_tx: &None` it can never be dispatched
/// in-process, so it is left permanently undelivered unless this test
/// deletes it. `trade::tests::cleanup` never touches the outbox — it never
/// had a reason to before this test called a grant path directly. Left
/// undelivered, the row poisons the outbox's own live-DB tests, which
/// share this slot's database and scan/drain undelivered rows table-wide
/// (issue #914 CI failure).
async fn cleanup_outbox(pool: &sqlx::PgPool, entity_id: u32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(entity_id as i32)
        .execute(pool)
        .await;
}

async fn insert_bind_on_acquire_type(pool: &sqlx::PgPool) {
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

/// A grant of a BIND_ON_ACQUIRE design lands `bound = true`, and offering
/// it in a trade rolls the whole swap back exactly like the hand-set
/// `bound = true` fixture in `commit::live_db_commit_rolls_back_on_bound_item`.
#[tokio::test]
async fn live_db_a_freshly_granted_bind_on_acquire_item_is_refused_by_trade() {
    let pool = require_db_or_skip!();
    let (_weapon_type_id, tradeable_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(1800);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
    cleanup_outbox(&pool, f.entity_a).await;
    insert_bind_on_acquire_type(&pool).await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
    let item_b = insert_item(&pool, f.player_b, tradeable_type_id, INV_MAIN, 0, false).await;

    let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
    let db = Some(Arc::new(pool.clone()));
    handle_grant_item(
        f.entity_a,
        f.player_a,
        BIND_TYPE_ID,
        INV_MAIN,
        1,
        false,
        &db,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
    let granted_item_id: i32 = sqlx::query_scalar(
        "SELECT item_id FROM sgw_inventory WHERE character_id = $1 AND type_id = $2",
    )
    .bind(f.player_a)
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
        "a BIND_ON_ACQUIRE grant must land bound (SS-914); the trade \
         refusal below only proves something once this does"
    );

    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        vec![granted_item_id],
        0,
        vec![item_b],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        owner_of(&pool, granted_item_id).await,
        Some(f.player_a),
        "a freshly-granted bind-on-acquire item must not change hands"
    );
    assert_eq!(owner_of(&pool, item_b).await, Some(f.player_b));

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
    cleanup_outbox(&pool, f.entity_a).await;
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(BIND_TYPE_ID)
        .execute(&pool)
        .await;
}
