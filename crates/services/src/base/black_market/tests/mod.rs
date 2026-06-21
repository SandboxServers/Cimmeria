//! Live-DB integration tests for the Black Market state machine + sweep, plus
//! the non-DB state-helper / validate / types edge tests.
//!
//! Live-DB tests skip cleanly when `DATABASE_URL` is unset (via
//! `require_db_or_skip!`). Against the bundled local Postgres they exercise
//! createAuction / placeBid / cancelAuction, the expiry sweep (sold / unsold /
//! multi-auction / phantom-bidder), and the reusable persistence helpers
//! (`adjust_player_cash`, `escrow_item`, `send_mail_to_player`, `return_item`).
//!
//! Sentinels fit in i32 and cleanup deletes by exact sentinel — never by range.
//! Shared fixtures live here in `mod.rs`; the per-area tests are split into
//! sibling files to keep each under the 500-line soft cap.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::ConnectedClientState;
use crate::test_support::TestTransport;

mod create_bid_cancel;
mod helpers;
mod state_helpers;
mod sweep;

/// Sentinel base for Black Market live-DB tests. Distinct from prior live-DB
/// sentinels (vendor sell +0x800 was the highest documented).
pub(super) const TEST_BASE: i32 = 0x7000_0900;

/// design_id 21 exists in `resources.items` (used by the vendor sell tests as a
/// known-good type), so `escrow_item` / `return_item`'s INSERT…SELECT against
/// `resources.items` resolves a real row.
pub(super) const ITEM_DEF_ID: i32 = 21;

pub(super) async fn cleanup(pool: &PgPool, account_ids: &[i32], player_ids: &[i32]) {
    for &pid in player_ids {
        let _ = sqlx::query("DELETE FROM sgw_auction_bid WHERE bidder_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1 OR current_bidder = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_gate_mail WHERE character_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
    }
    for &aid in account_ids {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(aid)
            .execute(pool)
            .await;
    }
}

pub(super) async fn insert_account_and_player(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
    naquadah: i32,
) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("bm-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("bmp-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

pub(super) async fn insert_item(pool: &PgPool, player_id: i32, type_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, 0, 0, false, 100, 0) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

pub(super) async fn naquadah_of(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar("SELECT naquadah::bigint FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

pub(super) async fn inventory_count(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

pub(super) fn make_state(
    entity_id: u32,
) -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (transport, entity_to_addr, connected)
}
