//! Type 5: a vault withdrawal racing a delete of the same character (the
//! lock order agreed for BV-07: per-player advisory locks, the actor's
//! `sgw_player` row `FOR KEY SHARE`, then `lock_org`, then items).
//!
//! A character delete takes the character's row `FOR UPDATE`, then its
//! organizations (`sgw_player_before_delete_lock_orgs`). A withdrawal that
//! took the organization first would hold it while its `sgw_inventory`
//! insert waits on that row through the foreign key: a deadlock (40P01).
//! Taking the player row first, the two serialize and one wins cleanly.

use std::time::{Duration, Instant};

use cimmeria_base_session::base::organization::character_delete::delete_character;
use cimmeria_entity::cell_entity::VaultScope;

use super::moves::{at_banker, mv};
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

/// The race is forced: the test holds the organization row lock (the lock
/// both sides want) while first the withdrawal and then the delete park,
/// in that order, then releases it. With the agreed order the withdrawal
/// already holds `KEY SHARE` on the character, so the delete waits on it,
/// the withdrawal commits, and the delete then removes the character with
/// the item it took. No side fails with a deadlock. With `lock_org` first,
/// the withdrawal gets the organization and waits on the delete's row lock
/// while the delete waits on the organization: Postgres aborts one side.
#[tokio::test]
async fn live_db_a_withdrawal_racing_a_delete_of_the_same_character_does_not_deadlock() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 17, 2).await;
    let team = fx.org(0, 0, &[1]).await;
    let item = fx.item(0);
    fx.put(team, item, 0, BANKABLE, 4).await;
    let client = Client::in_world(fx.entity(0), 40889);
    let vault = at_banker(VaultScope::Team, team);

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
    let withdraw = mv(&fx, &client, 0, item, 1, 0, -1, vault);
    let delete = async {
        // Only once the withdrawal is parked, so it holds what it takes
        // before the organization.
        wait_held(&pool, gate_pid, 1).await;
        delete_character(&pool, fx.player(0), fx.account_id).await
    };
    let release = async {
        wait_held(&pool, gate_pid, 2).await;
        gate.commit().await.unwrap();
    };
    let ((), deleted, ()) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(withdraw, delete, release)
    })
    .await
    .expect("the withdrawal and the delete hung past 20 s");

    let deleted = deleted.expect("the delete must not fail (a deadlock aborts it with 40P01)");
    assert!(deleted.deleted, "the character is deleted");
    let deadlocks: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| format!("{c:?}").contains("deadlock"))
        .collect();
    assert!(deadlocks.is_empty(), "no deadlock anywhere: {deadlocks:#?}");
    // The withdrawal committed first: the item left the vault, then went
    // with the character.
    assert!(fx.vault(team).await.is_empty(), "the withdrawal committed");
    assert!(fx.duplicated_ids().await.is_empty());
    fx.teardown().await;
}
