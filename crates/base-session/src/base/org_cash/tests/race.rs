//! Type 5: two withdrawals that together exceed the treasury.

use std::time::{Duration, Instant};

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Sessions held behind `gate_pid`, directly or through one waiter it holds.
async fn held(pool: &PgPool, gate_pid: i32) -> i64 {
    sqlx::query_scalar(
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
    .unwrap()
}

/// Wait until `n` sessions are parked behind `gate_pid`.
async fn wait_held(pool: &PgPool, gate_pid: i32, n: i64) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let waiting = held(pool, gate_pid).await;
        if waiting >= n {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "expected {n} sessions parked behind the gate (saw {waiting})"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Hold `org_id`'s row the way `lock_org` does, returning the gate and its
/// backend pid.
async fn gate(pool: &PgPool, org_id: i32) -> (sqlx::Transaction<'static, sqlx::Postgres>, i32) {
    let mut gate = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("SELECT 1 FROM sgw_organizations WHERE org_id = $1 FOR UPDATE")
        .bind(org_id)
        .execute(&mut *gate)
        .await
        .unwrap();
    (gate, pid)
}

/// The race is forced: the test holds the organization row lock while both
/// withdrawals of 60 from a treasury of 100 park behind it, then releases
/// it. The organization lock serializes them and each decides on the
/// balance it reads under that lock, so exactly one goes through (100 to
/// 40) and the other is refused `insufficient_org_cash` with nothing moved.
/// Fails if the treasury is written from a balance read before the lock
/// (both land, and 60 naquadah is created), or if its guards are removed
/// (the second is caught only by the `CHECK`, as `query_failed`).
#[tokio::test]
async fn live_db_two_withdrawals_that_exceed_the_treasury_let_exactly_one_through() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 5, 2, 0).await;
    let team = fx.org(OrgType::Team, 0, &[1]).await;
    fx.set_member_bits(team, OrgPermission::WITHDRAW_CASH, true)
        .await;
    fx.set_org_cash(team, 100).await;
    fx.online(0);
    fx.online(1);

    let (gate, gate_pid) = gate(&pool, team).await;
    let capture = LogCapture::install();
    let release = async {
        wait_held(&pool, gate_pid, 2).await;
        gate.commit().await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(
            fx.transfer(0, team, CashDir::Withdraw(60)),
            fx.transfer(1, team, CashDir::Withdraw(60)),
            release
        )
    })
    .await
    .expect("the withdrawals hung past 20 s");

    let (w0, w1, cash) = (
        fx.wallet(0).await,
        fx.wallet(1).await,
        fx.org_cash(team).await,
    );
    assert_eq!(cash, 40, "exactly one withdrawal: {w0} + {w1}");
    assert_eq!(w0 + w1, 60, "the wallets gained what the treasury lost");
    assert_eq!(bank_rows(&capture, "org_cash_transfer").len(), 1);
    let refused = bank_rows(&capture, "org_cash_rejected");
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert!(
        refused[0].has_field("reason", "insufficient_org_cash")
            && refused[0].has_field("org_cash_before", "40"),
        "{:#?}",
        refused[0]
    );
    assert_eq!(fx.cash_log(team).await.len(), 1);
    fx.teardown().await;
}

/// A member loses the right to withdraw while their withdrawal waits on the
/// organization lock: first the rank's `WithdrawCash` bit is cleared, then
/// (a second run) the member is removed, each by the transaction that holds
/// the lock, which then commits. The parked withdrawal reads membership and
/// bits only after it gets the lock, so it sees the change: `no_permission`,
/// then `not_a_member`, and nothing moves. Fails if the membership read
/// moves ahead of `lock_org`, or off the transaction onto the pool.
#[tokio::test]
async fn live_db_a_demote_or_kick_that_lands_while_a_withdrawal_waits_is_seen() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 7, 2, 0).await;
    let team = fx.org(OrgType::Team, 0, &[1]).await;
    fx.set_member_bits(team, OrgPermission::WITHDRAW_CASH, true)
        .await;
    fx.set_org_cash(team, 100).await;
    fx.online(1);
    let rank = i16::from(OrgRank::for_type(OrgType::Team)[0].as_u8());
    let bit = OrgPermission::WITHDRAW_CASH.bits() as i32;

    for reason in ["no_permission", "not_a_member"] {
        let (mut gate, gate_pid) = gate(&pool, team).await;
        let capture = LogCapture::install();
        let release = async {
            wait_held(&pool, gate_pid, 1).await;
            if reason == "no_permission" {
                sqlx::query(
                    "UPDATE sgw_organization_ranks SET permissions = permissions & ~$3 \
                     WHERE org_id = $1 AND rank = $2",
                )
                .bind(team)
                .bind(rank)
                .bind(bit)
                .execute(&mut *gate)
                .await
                .unwrap();
            } else {
                sqlx::query(
                    "DELETE FROM sgw_organization_members WHERE org_id = $1 AND player_id = $2",
                )
                .bind(team)
                .bind(fx.player(1))
                .execute(&mut *gate)
                .await
                .unwrap();
            }
            gate.commit().await.unwrap();
        };
        tokio::time::timeout(Duration::from_secs(20), async {
            tokio::join!(fx.transfer(1, team, CashDir::Withdraw(60)), release)
        })
        .await
        .expect("the withdrawal hung past 20 s");

        assert_eq!(
            (fx.wallet(1).await, fx.org_cash(team).await),
            (0, 100),
            "{reason}: nothing moved"
        );
        let refused = bank_rows(&capture, "org_cash_rejected");
        assert_eq!(refused.len(), 1, "{reason}: {refused:#?}");
        assert!(refused[0].has_field("reason", reason), "{:#?}", refused[0]);
        assert!(bank_rows(&capture, "org_cash_transfer").is_empty());
        // The second run starts with the bit back, so only the kick stops it.
        fx.set_member_bits(team, OrgPermission::WITHDRAW_CASH, true)
            .await;
    }
    assert!(fx.cash_log(team).await.is_empty());
    fx.teardown().await;
}
