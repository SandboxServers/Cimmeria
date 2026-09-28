//! Live-DB guards: vendor buyback and sell take the shared inventory lock
//! order (`crate::base::crafting::inventory_locks`: advisory keys, then
//! rows, then `sgw_player`), so each serializes with a trade, a crafting
//! completion or the other on the same player instead of deadlocking.
//!
//! Each test holds the other side's locks on a second connection, starts
//! the real handler, requires that it is seen waiting on a lock (else the
//! test is vacuous), then finishes the other side's lock sequence and
//! commits. With the old order that second half deadlocks and Postgres
//! aborts one side.
//!
//! Sentinels: accounts and players `0x7000_C590..=0x7000_C5A3`, entities
//! `0x7000_C5A8..=0x7000_C5A9`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_entity::inventory::{INV_BUYBACK, INV_CRAFTING, INV_MAIN};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::{handle_buyback_vendor_items, handle_sell_vendor_items};
use crate::base::crafting::inventory_locks::take_inventory_locks;
use crate::base::ConnectedClientState;
use crate::test_support::{require_db_or_skip, TestTransport};

/// The seeded vendor whose sell list prices design 21 at 1000.
const VENDOR_TEMPLATE_ID: i32 = 25;
/// Allowed in the main bag, and on vendor 25's sell list.
const TYPE_ID: i32 = 21;
const UNIT_PRICE: i32 = 1_000;

async fn setup(pool: &PgPool, account_id: i32, player_id: i32, entity_id: u32, naquadah: i32) {
    cleanup(pool, account_id, player_id, entity_id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("vendor-lock-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("vendor-lock-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

/// Also deletes the entity's `cell_event_outbox` rows: a sale enqueues
/// one, and an undelivered row left behind is drained by the next test
/// that drains the outbox (the drainer is not scoped to an entity).
async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32, entity_id: u32) {
    delete_outbox_rows(pool, entity_id).await;
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn delete_outbox_rows(pool: &PgPool, entity_id: u32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(entity_id as i32)
        .execute(pool)
        .await;
}

async fn outbox_rows(pool: &PgPool, entity_id: u32) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM cell_event_outbox WHERE entity_id = $1")
        .bind(entity_id as i32)
        .fetch_one(pool)
        .await
        .expect("count outbox rows")
}

async fn insert_item(pool: &PgPool, player_id: i32, container_id: i32, flags: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, flags) \
         VALUES ($1, $2, 1, 0, $3, false, 100, 0, $4) RETURNING item_id",
    )
    .bind(player_id)
    .bind(TYPE_ID)
    .bind(container_id)
    .bind(flags)
    .fetch_one(pool)
    .await
    .expect("insert item")
}

async fn container_of(pool: &PgPool, item_id: i32) -> i32 {
    sqlx::query_scalar("SELECT container_id FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(pool)
        .await
        .expect("read container")
}

async fn naquadah_of(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("read naquadah")
}

type State = (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
);

fn state(entity_id: u32) -> State {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr: SocketAddr = "127.0.0.1:41711".parse().unwrap();
    (
        transport,
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
    )
}

async fn backend_pid(conn: &mut sqlx::PgConnection) -> i32 {
    sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(conn)
        .await
        .unwrap()
}

/// Wait until some session other than `holder_pid` (and this pool's
/// probe) is waiting on a lock in this database; fail if none ever does.
async fn wait_until_blocked(pool: &PgPool, holder_pid: i32) {
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock' \
               AND pid <> pg_backend_pid() AND pid <> $1",
        )
        .bind(holder_pid)
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the vendor handler never waited on the held locks");
}

/// A trade on the player holds its inventory keys (0, 1, 15) and is about
/// to lock the player row. A buyback starts meanwhile. It must wait at
/// key 0; the trade then locks and writes the player row and commits,
/// and the buyback runs after it.
///
/// Revert-verifier: with buyback's old order (buyback rows, the player
/// row, then the main-bag key inside the slot pick) the buyback holds
/// the player row and waits for key 1, the trade waits for the player
/// row, and Postgres aborts one of them: the trade's player-row lock
/// fails, or the item stays in the buyback bag.
#[tokio::test]
async fn live_db_buyback_waits_for_a_trade_on_the_same_player() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C590, 0x7000_C591, 0x7000_C5A8_u32);
    setup(&pool, account_id, player_id, entity_id, 5_000).await;
    let sold = insert_item(&pool, player_id, INV_BUYBACK, UNIT_PRICE).await;

    let mut trade = pool.begin().await.expect("begin trade");
    let trade_pid = backend_pid(&mut trade).await;
    take_inventory_locks(&mut trade, player_id, &[INV_MAIN, INV_CRAFTING])
        .await
        .expect("trade advisory locks");

    let buyback = tokio::spawn({
        let pool = pool.clone();
        async move {
            let (transport, connected, e2a) = state(entity_id);
            handle_buyback_vendor_items(
                entity_id,
                player_id,
                99,
                VENDOR_TEMPLATE_ID,
                vec![(sold, 1)],
                &Some(Arc::new(pool)),
                &None,
                &transport,
                &connected,
                &e2a,
            )
            .await;
        }
    });
    wait_until_blocked(&pool, trade_pid).await;

    sqlx::query("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(player_id)
        .execute(&mut *trade)
        .await
        .expect("the trade locks the player row without a deadlock");
    sqlx::query("UPDATE sgw_player SET naquadah = naquadah + 1 WHERE player_id = $1")
        .bind(player_id)
        .execute(&mut *trade)
        .await
        .expect("the trade writes the player row");
    trade.commit().await.expect("the trade commits");

    tokio::time::timeout(Duration::from_secs(20), buyback)
        .await
        .expect("the buyback finishes once the trade commits")
        .expect("buyback task");
    assert_eq!(container_of(&pool, sold).await, INV_MAIN, "bought back");
    assert_eq!(naquadah_of(&pool, player_id).await, 5_000 + 1 - UNIT_PRICE);

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// A buyback on the player holds its keys (0, 1, 16) and a buyback row,
/// and is about to lock the main-bag rows for its slot pick, then the
/// player row. A sale of a main-bag item starts meanwhile. It must wait
/// at key 0; the buyback then locks the sold item's row (among the
/// main-bag rows) and the player row and commits, and the sale runs
/// after it.
///
/// Revert-verifier: with sell's old order (no key 0: the sold row first,
/// then the buyback key) the sale holds the sold row and waits for key
/// 16, the buyback waits for the sold row, and Postgres aborts one of
/// them.
#[tokio::test]
async fn live_db_sell_waits_for_a_buyback_on_the_same_player() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C592, 0x7000_C593, 0x7000_C5A9_u32);
    setup(&pool, account_id, player_id, entity_id, 5_000).await;
    let for_sale = insert_item(&pool, player_id, INV_MAIN, 0).await;
    let in_buyback = insert_item(&pool, player_id, INV_BUYBACK, UNIT_PRICE).await;

    let mut buyback = pool.begin().await.expect("begin buyback");
    let buyback_pid = backend_pid(&mut buyback).await;
    take_inventory_locks(&mut buyback, player_id, &[INV_MAIN, INV_BUYBACK])
        .await
        .expect("buyback advisory locks");
    sqlx::query("SELECT 1 FROM sgw_inventory WHERE item_id = $1 FOR UPDATE")
        .bind(in_buyback)
        .execute(&mut *buyback)
        .await
        .expect("buyback locks its buyback row");

    let sale = tokio::spawn({
        let pool = pool.clone();
        async move {
            let (transport, connected, e2a) = state(entity_id);
            handle_sell_vendor_items(
                entity_id,
                player_id,
                99,
                VENDOR_TEMPLATE_ID,
                vec![(for_sale, 1)],
                &Some(Arc::new(pool)),
                &None,
                &transport,
                &connected,
                &e2a,
            )
            .await;
        }
    });
    wait_until_blocked(&pool, buyback_pid).await;

    sqlx::query(
        "SELECT slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 FOR UPDATE",
    )
    .bind(player_id)
    .bind(INV_MAIN)
    .execute(&mut *buyback)
    .await
    .expect("the buyback locks the main-bag rows without a deadlock");
    sqlx::query("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(player_id)
        .execute(&mut *buyback)
        .await
        .expect("the buyback locks the player row");
    buyback.commit().await.expect("the buyback commits");

    let finished = tokio::time::timeout(Duration::from_secs(20), sale).await;
    // The sale enqueued one outbox row for the removed item. Count it,
    // then delete it before any assertion can fail and strand it.
    let enqueued = outbox_rows(&pool, entity_id).await;
    delete_outbox_rows(&pool, entity_id).await;
    finished
        .expect("the sale finishes once the buyback commits")
        .expect("sale task");
    assert_eq!(enqueued, 1, "the sale enqueues one outbox row");
    assert_eq!(container_of(&pool, for_sale).await, INV_BUYBACK, "sold");
    assert_eq!(naquadah_of(&pool, player_id).await, 5_000 + UNIT_PRICE);

    cleanup(&pool, account_id, player_id, entity_id).await;
}
