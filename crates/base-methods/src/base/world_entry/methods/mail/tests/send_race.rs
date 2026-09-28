//! Type 5 (concurrency): two senders race for a recipient's last open slot.

use std::time::{Duration, Instant};

use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::mail::codes::MailResult;

const BASE: i32 = 0x7300_1200;

/// D-SS03 / D-SS06: a recipient holds 99 open messages and two senders send
/// at once. Exactly one gets through: the count runs under the recipient's
/// `FOR UPDATE` row lock, so the second send counts 100 after the first
/// commits. Without the lock both count 99 and the box ends at 101.
///
/// The race is forced, not hoped for: the test holds `SHARE` on
/// `sgw_gate_mail` (which lets the count run and blocks every `INSERT`)
/// until both senders are parked behind that gate. Without the row lock both
/// sends are then parked at their `INSERT` having counted 99; with it, the
/// second is parked at `FOR UPDATE` behind the first. Only sessions held by
/// the gate, directly or through one waiter it holds, are counted
/// (`pg_blocking_pids`), so an unrelated lock waiter elsewhere in the
/// database cannot open the gate early.
#[tokio::test]
async fn live_db_concurrent_sends_respect_mailbox_cap() {
    let pool = require_db_or_skip!();
    let (acct, a, b, rcpt) = (BASE, BASE + 1, BASE + 2, BASE + 3);
    cleanup(&pool, acct).await;
    insert_players(
        &pool,
        acct,
        &[
            (a, "SsmOneRaceA"),
            (b, "SsmOneRaceB"),
            (rcpt, "SsmOneRaceR"),
        ],
    )
    .await;
    fill_mailbox(&pool, rcpt, 99, 0).await;

    let ca = Client::new(0x7300_1281, a, 54_721, "SsmOneRaceA");
    let cb = Client::new(0x7300_1282, b, 54_722, "SsmOneRaceB");
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
            .fetch_one(&pool)
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
        ca.op(MailOp::Send(plain_send(&["SsmOneRaceR"])), Some(&pool), now),
        cb.op(MailOp::Send(plain_send(&["SsmOneRaceR"])), Some(&pool), now),
        release,
    );

    assert_eq!(
        mail_count(&pool, rcpt).await,
        100,
        "exactly one of the two racing sends may fill the last slot"
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
                .expect("each sender is answered")
        })
        .collect();
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![MailResult::Sent.code(), MailResult::NoRecipients.code()]
    );

    cleanup(&pool, acct).await;
}
