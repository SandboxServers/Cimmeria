//! Bandolier persistence handlers — the active-slot UPDATE that drives the
//! visible weapon swap on the model, and the per-shot ammo writeback.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use super::super::super::ConnectedClientState;
use super::super::methods::inventory::update_bandolier_ammo;

/// `CellToBaseMsg::ActiveSlotUpdate` — persist the player's bandolier-slot
/// selection, then re-query the appearance so the BEING_APPEARANCE broadcast
/// swaps the visible weapon on the model. Skips the appearance refresh if
/// the UPDATE didn't land (DB row missing or write failure).
pub(super) async fn active_slot_update(
    entity_id: u32,
    player_id: i32,
    slot_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if let Some(pool) = db_pool {
        // The schema column is `bandolier_slot` (see sgw_player.sql);
        // an earlier draft used `active_bandolier_slot` which never
        // existed and would hard-fail at runtime.
        let updated = match sqlx::query(
            "UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2",
        )
        .bind(slot_id)
        .bind(player_id)
        .execute(pool.as_ref())
        .await
        {
            Ok(res) if res.rows_affected() == 0 => {
                tracing::warn!(player_id, slot_id, "ActiveSlotUpdate: no rows updated");
                false
            }
            Ok(_) => true,
            Err(e) => {
                tracing::warn!(player_id, slot_id, error = %e, "ActiveSlotUpdate: DB write failed");
                false
            }
        };
        // Refresh the player's appearance after the slot is durable.
        // The appearance query at `player_load/core.rs` filters
        // bandolier visual components by the persisted `bandolier_slot`,
        // so this re-query (and the resulting `BEING_APPEARANCE`
        // broadcast) is what actually swaps the visible weapon on
        // the model. Without it, F1-F4 changes the active slot but
        // the player keeps holding whatever weapon was visible at
        // login.
        //
        // Skip when the UPDATE didn't land — the appearance is still
        // consistent with what the DB says, and a no-op refresh would
        // just spam witnesses with the same packet they already have.
        if updated {
            super::super::methods::inventory::refresh_player_appearance(
                entity_id,
                player_id,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}

/// `CellToBaseMsg::BandolierAmmoUpdate` — persist a per-shot ammo writeback
/// from the cell. Validates bounds (slot in 0..5, ammo / type non-negative,
/// expected_item_id positive) at the service boundary so out-of-range
/// values can't become durable corruption.
pub(super) async fn bandolier_ammo_update(
    player_id: i32,
    slot_id: i32,
    expected_item_id: i32,
    current_ammo: i32,
    cur_ammo_type: i32,
    db_pool: &Option<Arc<PgPool>>,
) {
    // `player_id` from the cell is the DB character_id (matches the
    // `ActiveSlotUpdate` convention right above).
    //
    // Validate bounds before persisting — these payloads cross a
    // service boundary, and any out-of-range value would become
    // durable corruption. Bandolier holds 5 slots (0-4); ammo and
    // ammo_type IDs are non-negative.
    if !(0..5).contains(&slot_id) || current_ammo < 0 || cur_ammo_type < 0 || expected_item_id <= 0
    {
        tracing::warn!(
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
            "BandolierAmmoUpdate: dropping out-of-range payload"
        );
        return;
    }
    if let Some(pool) = db_pool {
        if let Err(e) = update_bandolier_ammo(
            pool.as_ref(),
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
        )
        .await
        {
            tracing::warn!(
                player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type, error = %e,
                "BandolierAmmoUpdate: DB write failed"
            );
        }
    }
}
