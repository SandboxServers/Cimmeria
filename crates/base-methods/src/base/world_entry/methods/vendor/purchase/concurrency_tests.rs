//! A vendor purchase racing a crafting completion for the same player.
//!
//! Crafting takes the player-wide advisory lock, then per-bag advisory
//! locks, then inventory rows. The purchase locks its cost rows, then the
//! player row, then the main bag's advisory lock. Unless the purchase
//! takes the player-wide lock before any row, the two orders form a cycle
//! and Postgres aborts one of them.

use std::time::Duration;

use super::tests::{
    cleanup, count_in_container, insert_account_and_player, insert_item, make_state, stack_sum,
    ITEM_COST_DESIGN_ID, ITEM_COST_PREREQ_DESIGN_ID, ITEM_COST_STORE_INDEX,
    SEEDED_BUY_VENDOR_TEMPLATE_ID,
};
use super::*;
use crate::test_support::require_db_or_skip;

/// In the crafting campaign's sentinel block, clear of the crafting
/// transaction tests (`0x7000_CF40..=0x7000_CF9F`).
const ACCOUNT_ID: i32 = 0x7000_CFC0;
const PLAYER_ID: i32 = 0x7000_CFC1;
const ENTITY_ID: i32 = 0x7000_CFC2;

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
    panic!("the purchase never blocked behind the crafting locks");
}

/// A completion holds the player-wide and main-bag advisory locks and is
/// about to lock the purchase's cost row. The purchase must wait for the
/// whole completion instead of taking the cost row and the player row
/// first: then the completion gets its row, commits, and the purchase
/// goes through after it. Both succeed; nothing deadlocks.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_purchase_waits_for_a_crafting_completion_instead_of_deadlocking() {
    let pool = require_db_or_skip!();
    cleanup(&pool, ENTITY_ID, ACCOUNT_ID, PLAYER_ID).await;
    insert_account_and_player(&pool, ACCOUNT_ID, PLAYER_ID, 5_000).await;
    let prereq = insert_item(&pool, PLAYER_ID, ITEM_COST_PREREQ_DESIGN_ID, INV_MAIN, 5, 1).await;

    // The crafting side, in its own lock order.
    let mut craft = pool.begin().await.expect("begin craft");
    for key in [0, INV_MAIN, 15] {
        sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(PLAYER_ID)
            .bind(key)
            .execute(&mut *craft)
            .await
            .expect("craft advisory lock");
    }

    let purchase = {
        let pool = pool.clone();
        tokio::spawn(async move {
            let (transport, e2a, conn) = make_state(ENTITY_ID as u32);
            handle_purchase_vendor_items(
                ENTITY_ID as u32,
                PLAYER_ID,
                99,
                SEEDED_BUY_VENDOR_TEMPLATE_ID,
                vec![(ITEM_COST_STORE_INDEX, 1)],
                &Some(Arc::new(pool)),
                &None,
                &transport,
                &conn,
                &e2a,
            )
            .await;
        })
    };
    wait_for_lock_waiter(&pool).await;

    // The completion now locks the cost row, as its consumption does.
    let row: Result<Option<i32>, sqlx::Error> =
        sqlx::query_scalar("SELECT item_id FROM sgw_inventory WHERE item_id = $1 FOR UPDATE")
            .bind(prereq)
            .fetch_optional(&mut *craft)
            .await;
    let committed = match row {
        Ok(_) => craft.commit().await,
        Err(e) => Err(e),
    };
    tokio::time::timeout(Duration::from_secs(10), purchase)
        .await
        .expect("purchase finished")
        .expect("purchase task");

    let granted = count_in_container(&pool, PLAYER_ID, INV_MAIN, ITEM_COST_DESIGN_ID).await;
    let prereq_left = stack_sum(&pool, PLAYER_ID, INV_MAIN, ITEM_COST_PREREQ_DESIGN_ID).await;
    cleanup(&pool, ENTITY_ID, ACCOUNT_ID, PLAYER_ID).await;

    committed.expect("the completion was not aborted as a deadlock victim");
    assert_eq!(granted, 1, "the purchase went through after the completion");
    assert_eq!(prereq_left, 0, "the purchase consumed its cost");
}
