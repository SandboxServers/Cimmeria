//! Type 5 (concurrency): attachment ops racing on one mail pay out once
//! (SS-M3, audit § 6 CAT-G-04). Sentinels: accounts, players and entities
//! `0x7300_1C00` up, items `0x7300_1C80` up.

use std::future::Future;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use sqlx::{Postgres, Transaction};

use super::packets::Client;
use super::*;

const BASE: i32 = 0x7300_1C00;
const ITEMS: i32 = 0x7300_1C80;

/// Hold `SHARE` on both mail tables, so every write to a mail row or an
/// escrow row blocks, until every op is either waiting behind the gate
/// (directly, or behind one the gate holds) or finished; then release it.
///
/// With the locks (the owner's advisory lock, the mail row `FOR UPDATE`)
/// the first op parks on its write and the rest queue behind it, so each
/// re-reads the mail after the one ahead commits. Without them every op
/// reads the same untouched mail before any can write. An op that refuses
/// without writing (a take on an unpaid COD) finishes instead of parking,
/// which is why finished ops count.
async fn open_gate(pool: &PgPool) -> (Transaction<'static, Postgres>, i32) {
    let mut gate = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE sgw_gate_mail, sgw_gate_mail_item IN SHARE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    (gate, pid)
}

async fn release_when_parked(
    pool: &PgPool,
    gate: Transaction<'static, Postgres>,
    gate_pid: i32,
    ops: i64,
    done: &AtomicI64,
) {
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
        let finished = done.load(Ordering::SeqCst);
        if waiting + finished >= ops {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "{ops} ops should be parked or finished (saw {waiting} parked, {finished} finished)"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    gate.commit().await.unwrap();
}

/// The shared test pool is too small for four ops, the gate and the poll
/// at once; the ops get a pool of their own on the same database.
async fn wide_pool() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .connect(&std::env::var("DATABASE_URL").expect("live-DB test"))
        .await
        .expect("connect the race pool")
}

/// Run `op`, then count it finished.
async fn counted(op: impl Future<Output = ()>, done: &AtomicI64) {
    op.await;
    done.fetch_add(1, Ordering::SeqCst);
}

fn take_item(mail_id: i32) -> MailOp {
    MailOp::TakeItem {
        mail_id,
        container_id: -1,
        slot_id: -1,
    }
}

/// CAT-G-04: two take-cash and two take-item requests for one mail (500
/// gift cash and an item), from two sessions of its owner, all in flight at
/// once. The owner is credited 500 once, the item lands in the bag once,
/// the escrow row and the cash are gone.
///
/// Revert that proves it: remove the advisory lock, the mail row
/// `FOR UPDATE` and the `cash > 0` predicate together; both cash takes read
/// 500 and both credit it (2,000). The layers back each other up, so
/// removing any one alone still passes: the conditional `UPDATE` re-checks
/// `cash` after its row wait, and the row lock alone makes the second take
/// re-read 0. The item half is also held by the escrowed instance keeping
/// its id (a second restore would hit the `sgw_inventory` key) and by the
/// escrow `DELETE`'s row count.
#[tokio::test]
async fn concurrent_take_cash_and_item_pays_out_once() {
    let pool = require_db_or_skip!();
    cleanup(&pool, BASE).await;
    let (owner, sender) = (BASE + 1, BASE + 2);
    insert_players(
        &pool,
        BASE,
        &[(owner, "SsmThreeRaceO"), (sender, "SsmThreeRaceS")],
    )
    .await;
    set_naquadah(&pool, owner, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeRaceS")
        .cash(500)
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let ca = Client::new(BASE as u32 + 0x10, owner, 55_160, "SsmThreeRaceO");
    let cb = Client::new(BASE as u32 + 0x11, owner, 55_161, "SsmThreeRaceO");
    let now = Instant::now();
    let ops = wide_pool().await;
    let (gate, gate_pid) = open_gate(&ops).await;
    let done = AtomicI64::new(0);
    tokio::join!(
        counted(ca.op(MailOp::TakeCash { mail_id }, Some(&ops), now), &done),
        counted(cb.op(MailOp::TakeCash { mail_id }, Some(&ops), now), &done),
        counted(ca.op(take_item(mail_id), Some(&ops), now), &done),
        counted(cb.op(take_item(mail_id), Some(&ops), now), &done),
        release_when_parked(&ops, gate, gate_pid, 4, &done),
    );
    ca.take();
    cb.take();

    assert_eq!(naquadah(&pool, owner).await, 1_500, "cash paid out once");
    assert_eq!(
        inventory_rows(&pool, item_id).await.len(),
        1,
        "item moved once"
    );
    assert!(!has_escrow(&pool, mail_id).await);
    assert_eq!(mail_state(&pool, mail_id).await, Some((owner, 0, 0, false)));

    cleanup(&pool, BASE).await;
}

/// CAT-G-04 / D-SS09: an unpaid COD blocks both takes, whatever the
/// interleaving. Pay, take-cash and take-item race on one COD mail (price
/// 300). The price is debited once and mailed to the sender once; the
/// payer is never paid the price as cash; the item moves at most once (it
/// moves only if the take runs after the payment).
#[tokio::test]
async fn concurrent_pay_cod_and_takes_never_pay_out_the_price() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x08;
    cleanup(&pool, base).await;
    let (payer, sender) = (base + 1, base + 2);
    insert_players(
        &pool,
        base,
        &[(payer, "SsmThreeRaceCodP"), (sender, "SsmThreeRaceCodS")],
    )
    .await;
    set_naquadah(&pool, payer, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 0x08;
    let mail_id = AttachedMail::from(payer, sender, "SsmThreeRaceCodS")
        .cod(300)
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let ca = Client::new(base as u32 + 0x10, payer, 55_162, "SsmThreeRaceCodP");
    let cb = Client::new(base as u32 + 0x11, payer, 55_163, "SsmThreeRaceCodP");
    let now = Instant::now();
    let ops = wide_pool().await;
    let (gate, gate_pid) = open_gate(&ops).await;
    let done = AtomicI64::new(0);
    tokio::join!(
        counted(ca.op(MailOp::PayCod { mail_id }, Some(&ops), now), &done),
        counted(cb.op(MailOp::PayCod { mail_id }, Some(&ops), now), &done),
        counted(ca.op(MailOp::TakeCash { mail_id }, Some(&ops), now), &done),
        counted(cb.op(take_item(mail_id), Some(&ops), now), &done),
        release_when_parked(&ops, gate, gate_pid, 4, &done),
    );
    ca.take();
    cb.take();

    assert_eq!(
        naquadah(&pool, payer).await,
        700,
        "debited once, never paid"
    );
    let payments: Vec<i64> =
        sqlx::query_scalar("SELECT cash FROM sgw_gate_mail WHERE character_id = $1")
            .bind(sender)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(payments, vec![300], "one payment mail");
    assert_eq!(mail_state(&pool, mail_id).await, Some((payer, 0, 0, false)));
    let moved = inventory_rows(&pool, item_id).await.len();
    assert!(moved <= 1);
    assert_eq!(has_escrow(&pool, mail_id).await, moved == 0);

    cleanup(&pool, base).await;
}
