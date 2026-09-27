//! Type 5 (concurrency): two sends of the same item at once move it once.

use std::time::{Duration, Instant};

use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::mail::codes::MailResult;

const BASE: i32 = 0x7300_1450;
const ITEMS: i32 = 0x7300_1650;

/// Two sessions of one sender send `quantity` of the same `stack_size`
/// stack at the same time. Returns the two result codes, sorted.
///
/// The race is forced, as in `concurrent_sends_respect_mailbox_cap`: the
/// test holds `SHARE` on `sgw_gate_mail`, which blocks every mail insert,
/// until both sends are parked behind it (directly, or behind a send the
/// gate holds). With the item locked (advisory lock, `FOR UPDATE`), the
/// second send waits behind the first and reads the item after the first
/// commits. Without the locks, both read the full stack before either
/// writes.
async fn race_same_item(
    pool: &PgPool,
    offset: i32,
    stack_size: i32,
    quantity: i32,
) -> (i32, i32, TestItem, Vec<u8>) {
    let (acct, sender, rcpt) = (BASE + offset, BASE + offset + 1, BASE + offset + 2);
    let (s_name, r_name) = (
        format!("SsmTwoRaceS{offset}"),
        format!("SsmTwoRaceR{offset}"),
    );
    cleanup(pool, acct).await;
    insert_players(pool, acct, &[(sender, &s_name), (rcpt, &r_name)]).await;
    set_naquadah(pool, sender, 1_000).await;
    let type_id = any_type_id(pool).await;
    let stack = TestItem::main(ITEMS + offset, sender, 0, stack_size);
    insert_item(pool, stack, type_id).await;

    let send = || {
        let mut s = plain_send(&[r_name.as_str()]);
        s.item_id = stack.item_id;
        s.item_quantity = quantity;
        s
    };
    let port = 54_761 + 2 * offset as u16;
    let ca = Client::new(0x7300_1491, sender, port, &s_name);
    let cb = Client::new(0x7300_1492, sender, port + 1, &s_name);
    let now = Instant::now();
    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE sgw_gate_mail IN SHARE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let release = async {
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
            if waiting >= 2 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "both sends should be parked behind the gate (saw {waiting})"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        gate.commit().await.unwrap();
    };
    tokio::join!(
        ca.op(MailOp::Send(send()), Some(pool), now),
        cb.op(MailOp::Send(send()), Some(pool), now),
        release,
    );

    let mut codes: Vec<u8> = [ca.take(), cb.take()]
        .iter()
        .map(|received| {
            received
                .iter()
                .find_map(|r| match r {
                    Received::SendMailResult { result, .. } => Some(*result),
                    _ => None,
                })
                .expect("each send is answered")
        })
        .collect();
    codes.sort_unstable();
    (sender, rcpt, stack, codes)
}

/// CAT-G-01 / D-SS06, split path: two sends of two from a three-stack at
/// once. One is delivered (two in escrow, one left in the bag); the other
/// reads the stack after the first commits and is refused
/// `ItemNotAvailable`, with nothing debited. Without the locks and the
/// `stack_size > $1` guard both escrow two and the stack ends at -1.
#[tokio::test]
async fn concurrent_sends_move_item_once() {
    let pool = require_db_or_skip!();
    let (sender, rcpt, stack, codes) = race_same_item(&pool, 0, 3, 2).await;
    assert_eq!(
        codes,
        vec![MailResult::Sent.code(), MailResult::ItemNotAvailable.code()]
    );
    assert_eq!(inventory_row(&pool, stack.item_id).await, Some((sender, 1)));
    let escrow = escrow_for(&pool, rcpt).await;
    assert_eq!(
        escrow.len(),
        1,
        "the item went into escrow once: {escrow:?}"
    );
    assert_eq!(escrow[0].stack_size, 2);
    assert_eq!(mail_count(&pool, rcpt).await, 1);
    assert_eq!(naquadah(&pool, sender).await, 975, "postage charged once");

    cleanup(&pool, BASE).await;
}

/// The same race on the whole-row path: two sends of a whole three-stack.
/// The second finds no row once the first has moved it (`item_not_owned`),
/// so the row is in escrow once and the bag holds nothing.
#[tokio::test]
async fn concurrent_whole_stack_sends_move_item_once() {
    let pool = require_db_or_skip!();
    let (sender, rcpt, stack, codes) = race_same_item(&pool, 10, 3, 3).await;
    assert_eq!(
        codes,
        vec![MailResult::Sent.code(), MailResult::ItemNotAvailable.code()]
    );
    assert_eq!(inventory_row(&pool, stack.item_id).await, None);
    let escrow = escrow_for(&pool, rcpt).await;
    assert_eq!(
        escrow.len(),
        1,
        "the item went into escrow once: {escrow:?}"
    );
    assert_eq!(
        (escrow[0].item_id, escrow[0].stack_size),
        (stack.item_id, 3)
    );
    assert_eq!(mail_count(&pool, rcpt).await, 1);
    assert_eq!(naquadah(&pool, sender).await, 975, "postage charged once");

    cleanup(&pool, BASE + 10).await;
}
