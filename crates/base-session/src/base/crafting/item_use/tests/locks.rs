//! Lock order against the other inventory paths. A second connection holds
//! another path's locks in that path's order, the item use runs until it
//! blocks, then the second connection takes the lock that would close a
//! cycle. With the shared order both commit; with it broken Postgres aborts
//! one of them as a deadlock victim.

use std::time::Duration;

use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};

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
    panic!("the item use never blocked behind the other path's locks");
}

async fn advisory(
    conn: &mut sqlx::PgConnection,
    player_id: i32,
    key: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(player_id)
        .bind(key)
        .execute(conn)
        .await
        .map(|_| ())
}

async fn lock_row(
    conn: &mut sqlx::PgConnection,
    sql: &'static str,
    id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(sql).bind(id).execute(conn).await.map(|_| ())
}

const LOCK_ITEM: &str = "SELECT 1 FROM sgw_inventory WHERE item_id = $1 FOR UPDATE";
const LOCK_PLAYER: &str = "SELECT 1 FROM sgw_player WHERE player_id = $1 FOR UPDATE";

/// The lock the other path takes last, closing the cycle if the use holds
/// what it waits for.
enum Closing {
    Advisory { player_id: i32, key: i32 },
    Row { sql: &'static str, id: i32 },
}

/// Run the use in a task while `other` holds its first locks, then let
/// `other` take `closing` and commit. Returns the other side's result and
/// whether the use committed.
async fn race(
    pool: &PgPool,
    slot: Slot,
    port: u16,
    mut other: sqlx::Transaction<'static, sqlx::Postgres>,
    closing: Closing,
) -> (Result<(), sqlx::Error>, bool) {
    let used = {
        let pool = pool.clone();
        tokio::spawn(async move {
            let session = OneSession::new(slot.entity(), port);
            use_item(Some(&pool), &session, slot, slot.item)
                .await
                .is_some()
        })
    };
    wait_for_lock_waiter(pool).await;
    let taken = match closing {
        Closing::Advisory { player_id, key } => advisory(&mut other, player_id, key).await,
        Closing::Row { sql, id } => lock_row(&mut other, sql, id).await,
    };
    let closed = match taken {
        Ok(()) => other.commit().await,
        Err(e) => Err(e),
    };
    let used = tokio::time::timeout(Duration::from_secs(10), used)
        .await
        .expect("the use finished")
        .expect("use task");
    (closed, used)
}

/// A vendor purchase takes the player-wide key 0, its cost rows, the player
/// row, then the main bag's key while it reserves slots. The use must wait
/// on key 0 rather than take the main-bag key and then wait for the row the
/// purchase holds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_db_a_use_waits_for_a_vendor_purchase_instead_of_deadlocking() {
    let pool = require_db_or_skip!();
    let slot = Slot::new(13);
    player(&pool, slot, &[], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_CRAFTING,
        1,
    )
    .await;

    let mut vendor = pool.begin().await.expect("begin vendor");
    advisory(&mut vendor, slot.player_id, 0)
        .await
        .expect("key 0");
    lock_row(&mut vendor, LOCK_ITEM, slot.item)
        .await
        .expect("cost row");
    lock_row(&mut vendor, LOCK_PLAYER, slot.player_id)
        .await
        .expect("player row");
    let closing = Closing::Advisory {
        player_id: slot.player_id,
        key: INV_MAIN,
    };
    let (closed, used) = race(&pool, slot, 55805, vendor, closing).await;

    let (_, levels) = crafting(&pool, slot.player_id).await;
    cleanup(&pool, slot).await;
    closed.expect("the purchase was not aborted as a deadlock victim");
    assert!(used, "the use committed after the purchase");
    assert_eq!(levels[2], (3, 2));
}

/// A writer holding the main bag's key and the player row, then reaching for
/// the item row: the order trade used before it took the shared inventory
/// order (a real trade now takes key 0 first, so it and a use queue on key
/// 0). The use must still wait on the main-bag key rather than lock the item
/// row and then wait for the player row the writer holds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_db_a_use_waits_for_a_trade_instead_of_deadlocking() {
    let pool = require_db_or_skip!();
    let slot = Slot::new(14);
    player(&pool, slot, &[], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_CRAFTING,
        1,
    )
    .await;

    let mut trade = pool.begin().await.expect("begin trade");
    advisory(&mut trade, slot.player_id, INV_MAIN)
        .await
        .expect("main-bag key");
    lock_row(&mut trade, LOCK_PLAYER, slot.player_id)
        .await
        .expect("player row");
    let closing = Closing::Row {
        sql: LOCK_ITEM,
        id: slot.item,
    };
    let (closed, used) = race(&pool, slot, 55806, trade, closing).await;

    let (_, levels) = crafting(&pool, slot.player_id).await;
    cleanup(&pool, slot).await;
    closed.expect("the trade was not aborted as a deadlock victim");
    assert!(used, "the use committed after the trade");
    assert_eq!(levels[2], (3, 2));
}
