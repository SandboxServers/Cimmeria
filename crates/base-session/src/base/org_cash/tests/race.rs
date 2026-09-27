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

/// The race is forced: the test holds the organization row lock while both
/// withdrawals of 60 from a treasury of 100 park behind it, then releases
/// it. The organization lock serializes them and each decides on the
/// balance it reads under that lock, so exactly one goes through (100 to
/// 40) and the other is refused `insufficient_org_cash` with nothing moved.
/// Fails if the treasury is written from a balance read before the lock
/// (both land, and 60 naquadah is created), or if its guards are removed
/// (the second is caught only by the `CHECK`, as `query_failed`).
#[tokio::test]
async fn two_withdrawals_that_exceed_the_treasury_let_exactly_one_through() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 5, 2, 0).await;
    let team = fx.org(OrgType::Team, 0, &[1]).await;
    fx.set_member_bits(team, OrgPermission::WITHDRAW_CASH, true)
        .await;
    fx.set_org_cash(team, 100).await;
    fx.online(0);
    fx.online(1);

    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("SELECT 1 FROM sgw_organizations WHERE org_id = $1 FOR UPDATE")
        .bind(team)
        .execute(&mut *gate)
        .await
        .unwrap();

    let capture = LogCapture::install();
    let release = async {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let waiting = held(&pool, gate_pid).await;
            if waiting >= 2 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "expected both withdrawals parked behind the gate (saw {waiting})"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
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
