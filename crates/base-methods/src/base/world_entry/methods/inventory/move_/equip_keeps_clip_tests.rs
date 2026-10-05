//! Live-DB guard for OD-CS13's other half: only acquiring a gun empties it.
//! Equipping (bag to bandolier) and unequipping (bandolier to bag) keep the
//! row's rounds, and the bandolier sync tells the cell the same count.
//!
//! Sentinels: `0x7000_C830` (account), `0x7000_C831` (player),
//! `0x7000_C832` (entity).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::tests::{cleanup, insert_account_and_player};
use super::*;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{require_db_or_skip, TestTransport};

const ACCOUNT_ID: i32 = 0x7000_C830;
const PLAYER_ID: i32 = 0x7000_C831;
const ENTITY_ID: u32 = 0x7000_C832;
const PISTOL: i32 = 55;
const BANDOLIER: i32 = 3;
const MAIN_BAG: i32 = 1;

async fn insert_pistol(pool: &PgPool, ammo: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, ammo) \
         VALUES ($1, $2, 1, 0, $3, false, 100, 0, $4) RETURNING item_id",
    )
    .bind(PLAYER_ID)
    .bind(PISTOL)
    .bind(MAIN_BAG)
    .bind(ammo)
    .fetch_one(pool)
    .await
    .expect("insert pistol")
}

async fn row(pool: &PgPool, item_id: i32) -> (i32, i32) {
    sqlx::query_as("SELECT container_id, ammo FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(pool)
        .await
        .expect("pistol row")
}

/// Move `item_id` to `(container, slot)`; return the pistol's
/// `current_ammo` from any `SyncBandolierItems` the move sent the cell.
async fn move_to(pool: &PgPool, item_id: i32, container: i32, slot: i32) -> Option<i32> {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let e2a: Arc<Mutex<HashMap<u32, SocketAddr>>> = Arc::new(Mutex::new(HashMap::from([(
        ENTITY_ID,
        "127.0.0.1:65535".parse().unwrap(),
    )])));
    let conn = Arc::new(Mutex::new(HashMap::new()));
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(32);
    handle_move_inventory_item(
        ENTITY_ID,
        PLAYER_ID,
        item_id,
        container,
        slot,
        1,
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;
    let mut synced = None;
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::SyncBandolierItems {
            bandolier_items, ..
        } = msg
        {
            synced = bandolier_items
                .iter()
                .find(|(_, item)| item.instance_id == item_id)
                .map(|(_, item)| item.current_ammo);
        }
    }
    synced
}

/// A partly loaded (9 of 15) and a full (15 of 15) pistol keep their rounds
/// through equip and unequip, in the row and on the cell.
#[tokio::test]
async fn live_db_equip_and_unequip_keep_the_clip() {
    let pool = require_db_or_skip!();
    for ammo in [9, 15] {
        cleanup(&pool, ACCOUNT_ID, PLAYER_ID).await;
        insert_account_and_player(&pool, ACCOUNT_ID, PLAYER_ID).await;
        let pistol = insert_pistol(&pool, ammo).await;

        let synced = move_to(&pool, pistol, BANDOLIER, 0).await;
        assert_eq!(
            row(&pool, pistol).await,
            (BANDOLIER, ammo),
            "equip keeps the row's {ammo} rounds",
        );
        assert_eq!(
            synced,
            Some(ammo),
            "the bandolier sync tells the cell the equipped pistol holds {ammo}",
        );

        move_to(&pool, pistol, MAIN_BAG, 0).await;
        assert_eq!(
            row(&pool, pistol).await,
            (MAIN_BAG, ammo),
            "unequip keeps the row's {ammo} rounds",
        );
    }
    cleanup(&pool, ACCOUNT_ID, PLAYER_ID).await;
}
