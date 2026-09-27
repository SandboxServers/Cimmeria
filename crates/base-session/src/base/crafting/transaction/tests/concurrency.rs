//! A completion racing the vendor stack's lock order on the same player.
//!
//! Sell, repair and the rest of the vendor stack lock an inventory row,
//! then the `sgw_player` row. A completion that locked the player row
//! before its inventory rows would close a cycle with them.

use std::time::Duration;

use super::*;
use crate::test_support::require_db_or_skip;

/// Wait until some backend of this database is blocked on a lock.
async fn wait_for_lock_waiter(pool: &PgPool) {
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock'",
        )
        .fetch_one(pool)
        .await
        .expect("read pg_stat_activity");
        if waiting > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the completion never blocked behind the component row");
}

/// Another transaction holds the component row and then asks for the
/// player row, as a vendor sale does. The completion waits for the row
/// without holding the player row, so the other transaction gets it and
/// commits, and then the completion consumes. Neither is aborted.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_completion_never_holds_the_player_row_while_waiting_for_an_item() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 19).await;
    let component = f.stack(COMPONENT, INV_MAIN, 0, 2).await;

    let mut vendor = pool.begin().await.expect("begin vendor-order tx");
    sqlx::query("SELECT item_id FROM sgw_inventory WHERE item_id = $1 FOR UPDATE")
        .bind(component)
        .execute(&mut *vendor)
        .await
        .expect("lock the component row");

    let craft = {
        let pool = pool.clone();
        let ids = f.ids();
        let plan = CraftTransaction {
            named_items: vec![NamedItem::new(component, COMPONENT)],
            consume: vec![(COMPONENT, 1)],
            ..CraftTransaction::default()
        };
        tokio::spawn(async move { run_craft_transaction(&pool, &ids, &plan).await.map(|_| ()) })
    };
    wait_for_lock_waiter(&pool).await;

    let player: Result<Option<i32>, sqlx::Error> =
        sqlx::query_scalar("SELECT player_id FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(f.player_id)
            .fetch_optional(&mut *vendor)
            .await;
    let vendor_done = match player {
        Ok(_) => vendor.commit().await,
        Err(e) => Err(e),
    };
    let crafted = tokio::time::timeout(Duration::from_secs(10), craft)
        .await
        .expect("completion finished")
        .expect("completion task");
    let left = f.row(component).await;
    f.cleanup().await;

    vendor_done.expect("the vendor-order transaction was not aborted");
    crafted.expect("the completion was not aborted as a deadlock victim");
    assert_eq!(left, Some((1, INV_MAIN)), "the completion consumed one");
}
