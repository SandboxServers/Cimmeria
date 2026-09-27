//! A respec racing an alloy's completion: the knowledge check runs inside
//! the completion's transaction, under a share lock on the player row, so
//! the two serialize.

use std::time::Duration;

use sqlx::PgPool;

use super::fixture::*;
use crate::base::crafting::test_packets::feedback_text;
use crate::mercury::method_idx;
use crate::test_support::require_db_or_skip;

/// Whether some backend of this database is blocked on a lock, polled for
/// up to two seconds.
async fn saw_lock_waiter(pool: &PgPool) -> bool {
    for _ in 0..80 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock'",
        )
        .fetch_one(pool)
        .await
        .expect("read pg_stat_activity");
        if waiting > 0 {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    false
}

/// The respec has written the player row but not committed when the bar
/// ends. The completion waits for it, then sees the discipline gone and
/// refuses: nothing consumed, nothing granted. A check made before the
/// transaction would read the committed (old) state, pass, and alloy.
#[tokio::test]
async fn a_respec_committing_during_the_completion_refuses_the_alloy() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 21).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let normals = f.singles(NORMAL, 0, 10).await;
    f.alloy(ALLOY, component, &normals).await;
    let before = f.inventory().await;
    f.transport.clear();

    let mut respec = pool.begin().await.expect("begin respec");
    sqlx::query("UPDATE sgw_player SET discipline_ids = '{}' WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&mut *respec)
        .await
        .expect("forget the discipline, uncommitted");

    let respec_side = async {
        let blocked = saw_lock_waiter(&pool).await;
        respec.commit().await.expect("commit respec");
        blocked
    };
    let (ran, blocked) = tokio::join!(f.finish_inductions(), respec_side);

    let after = f.inventory().await;
    let lines: Vec<String> = f
        .calls()
        .iter()
        .filter(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
        .map(feedback_text)
        .collect();
    f.cleanup().await;

    assert_eq!(ran, 1);
    assert!(blocked, "the completion waited for the respec's row lock");
    assert_eq!(after, before, "nothing consumed, nothing granted");
    assert_eq!(
        lines,
        vec!["You must learn the blueprint's discipline first.".to_string()]
    );
}
