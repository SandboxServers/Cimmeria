//! Type 5 (concurrency): the expiry sweep racing an owner's take on one
//! mail (SS-M4). Sentinels: accounts, players and entities `0x7300_2100`
//! up, items `0x7300_2180` up.
//!
//! Both races are ordered: the take starts first and parks on the gate at
//! its first write, then the sweep starts and parks, then the gate opens.
//! With the shared lock order (`claim::lock_mail`) the sweep waits for the
//! take's commit and re-reads the mail; without it, the sweep decides on
//! the mail as it was before the take.

use std::sync::atomic::AtomicI64;
use std::time::{Duration, Instant};

use sqlx::{Postgres, Transaction};

use super::super::expiry::sweep_mailbox;
use super::packets::Client;
use super::take_race::{counted, open_gate, release_when_parked, wide_pool};
use super::*;
use crate::cell::messages::MailOp;

const BASE: i32 = 0x7300_2100;
const ITEMS: i32 = 0x7300_2180;
const NOW: i32 = 1_000_000;

/// Wait until at least `n` sessions are parked behind the gate (directly
/// or behind one it holds).
async fn wait_parked(pool: &PgPool, gate_pid: i32, n: i64) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let waiting: i64 = sqlx::query_scalar(
            "WITH held AS (\
                 SELECT pid FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid))\
             ) \
             SELECT COUNT(*) FROM pg_stat_activity a \
             WHERE a.pid IN (SELECT pid FROM held) \
                OR EXISTS (SELECT 1 FROM held h WHERE h.pid = ANY(pg_blocking_pids(a.pid)))",
        )
        .bind(gate_pid)
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting >= n {
            return;
        }
        assert!(Instant::now() < deadline, "{n} sessions should park");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Run `take` first, then the sweep once `take` has parked, then open the
/// gate once both have.
async fn take_then_sweep(
    ops: &PgPool,
    gate: Transaction<'static, Postgres>,
    gate_pid: i32,
    owner: i32,
    take: impl std::future::Future<Output = ()>,
) {
    let done = AtomicI64::new(0);
    let sweep = async {
        wait_parked(ops, gate_pid, 1).await;
        counted(
            async {
                sweep_mailbox(ops, owner, NOW, None).await;
            },
            &done,
        )
        .await;
    };
    tokio::join!(
        counted(take, &done),
        sweep,
        release_when_parked(ops, gate, gate_pid, 2, &done),
    );
}

/// An expired mail holding 500 gift cash and an item. Its owner takes the
/// cash while the sweep returns the mail: the 500 is paid out once, either
/// to the owner or back to the sender with the mail, never both; the item
/// exists once.
///
/// Revert that proves it: drop the advisory lock and `FOR UPDATE` from
/// `claim::lock_mail`. The sweep no longer waits for the parked take, so
/// the take-first ordering assert fails (observed: the sweep's return
/// committed first). A stale read is also exposed: the return `UPDATE`
/// re-checks the owner, not the cash, so a sweep that read `cash = 500`
/// before the take committed would write it back onto the returned mail.
#[tokio::test]
async fn live_db_sweep_racing_take_cash_pays_out_once() {
    let pool = require_db_or_skip!();
    cleanup(&pool, BASE).await;
    let (owner, sender) = (BASE + 1, BASE + 2);
    insert_players(
        &pool,
        BASE,
        &[(owner, "SsmFourRaceO"), (sender, "SsmFourRaceS")],
    )
    .await;
    set_naquadah(&pool, owner, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourRaceS")
        .cash(500)
        .item(ITEMS, type_id, 1)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW), false, false).await;

    let c = Client::new(BASE as u32 + 0x10, owner, 55_210, "SsmFourRaceO");
    let ops = wide_pool().await;
    let (gate, gate_pid) = open_gate(&ops).await;
    let take = c.op(MailOp::TakeCash { mail_id }, Some(&ops), Instant::now());
    take_then_sweep(&ops, gate, gate_pid, owner, take).await;
    c.take();

    let credited = i64::from(naquadah(&pool, owner).await - 1_000);
    let row = expiry_row(&pool, mail_id).await.expect("the mail survives");
    assert_eq!(
        credited + row.cash,
        500,
        "paid out once: {credited} + {row:?}"
    );
    assert_eq!(credited, 500, "the take committed first");
    assert_eq!(row.character_id, sender, "then the sweep returned it");
    assert!(has_escrow(&pool, mail_id).await, "with its item");
    assert!(inventory_rows(&pool, ITEMS).await.is_empty());

    cleanup(&pool, BASE).await;
}

/// An expired mail holding an item. Its owner takes the item while the
/// sweep runs: the item lands once, in the owner's bag, the escrow row is
/// gone, and the now-empty mail is deleted rather than returned empty.
/// (The item cannot be duplicated even without the locks: the instance id
/// is unique and the take deletes the escrow row with `rows_affected == 1`;
/// this pins the path the sweep then takes.)
#[tokio::test]
async fn live_db_sweep_racing_take_item_moves_it_once() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x08;
    cleanup(&pool, base).await;
    let (owner, sender) = (base + 1, base + 2);
    insert_players(
        &pool,
        base,
        &[(owner, "SsmFourRaceIO"), (sender, "SsmFourRaceIS")],
    )
    .await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 1;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourRaceIS")
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW), false, false).await;

    let c = Client::new(base as u32 + 0x10, owner, 55_211, "SsmFourRaceIO");
    let ops = wide_pool().await;
    let (gate, gate_pid) = open_gate(&ops).await;
    let take = c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&ops),
        Instant::now(),
    );
    take_then_sweep(&ops, gate, gate_pid, owner, take).await;
    c.take();

    let rows = inventory_rows(&pool, item_id).await;
    assert_eq!(rows.len(), 1, "moved once: {rows:?}");
    assert_eq!(rows[0].0, owner);
    assert!(!has_escrow(&pool, mail_id).await);
    assert_eq!(expiry_row(&pool, mail_id).await, None, "empty, so deleted");
    assert_eq!(mail_count(&pool, sender).await, 0, "nothing returned");

    cleanup(&pool, base).await;
}

/// The expiry sweep racing the payment of the COD it would return: the
/// payer pays first and parks, then the sweep runs. The paid COD is never
/// returned to the seller (who would then hold the item and the price):
/// the payer is debited once, the seller gets one payment mail, and the
/// mail stays the payer's with its item and a fresh expiry.
#[tokio::test]
async fn live_db_sweep_racing_pay_cod_never_returns_a_paid_cod() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x10;
    cleanup(&pool, base).await;
    let (payer, seller) = (base + 1, base + 2);
    insert_players(
        &pool,
        base,
        &[(payer, "SsmFourRacePP"), (seller, "SsmFourRacePS")],
    )
    .await;
    set_naquadah(&pool, payer, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 2;
    let mail_id = AttachedMail::from(payer, seller, "SsmFourRacePS")
        .cod(300)
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW), false, false).await;

    let c = Client::new(base as u32 + 0x10, payer, 55_212, "SsmFourRacePP");
    let ops = wide_pool().await;
    let (gate, gate_pid) = open_gate(&ops).await;
    let pay = c.op(MailOp::PayCod { mail_id }, Some(&ops), Instant::now());
    take_then_sweep(&ops, gate, gate_pid, payer, pay).await;
    c.take();

    assert_eq!(naquadah(&pool, payer).await, 700, "debited once");
    let seller_mail: Vec<(i64, Option<i32>)> =
        sqlx::query_as("SELECT cash, sender_id FROM sgw_gate_mail WHERE character_id = $1")
            .bind(seller)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        seller_mail,
        vec![(300, None)],
        "one payment mail, nothing returned"
    );
    let row = expiry_row(&pool, mail_id).await.unwrap();
    assert_eq!(row.character_id, payer, "{row:?}");
    assert!(!row.returned && !row.quarantined, "{row:?}");
    assert!(row.expires_at.is_some_and(|at| at > NOW), "{row:?}");
    assert!(has_escrow(&pool, mail_id).await);

    cleanup(&pool, base).await;
}
