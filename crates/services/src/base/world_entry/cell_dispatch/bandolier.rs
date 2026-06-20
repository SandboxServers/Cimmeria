//! Bandolier persistence handlers — the active-slot UPDATE that drives the
//! visible weapon swap on the model, and the per-shot ammo writeback.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::ConnectedClientState;
use super::super::methods::inventory::update_bandolier_ammo;

/// `CellToBaseMsg::ActiveSlotUpdate` — persist the player's bandolier-slot
/// selection, then re-query the appearance so the BEING_APPEARANCE broadcast
/// swaps the visible weapon on the model. Skips the appearance refresh if
/// the UPDATE didn't land (DB row missing or write failure).
#[tracing::instrument(
    name = "bandolier.active_slot_update",
    level = "info",
    skip_all,
    fields(entity_id, player_id, slot_id)
)]
pub(super) async fn active_slot_update(
    entity_id: u32,
    player_id: i32,
    slot_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
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
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}

/// `CellToBaseMsg::RefreshAppearance` — update the cached holster state
/// on the connected client and rebroadcast `BeingAppearance` to AoI.
///
/// Used by the combat enter/exit path (Phase 2 of the holster work,
/// ) to draw or holster the weapon without an inventory change.
/// The cached `weapon_holstered` flag is what every appearance-emit
/// site (here, the equipment-move path, world entry, post-cinematic
/// resend) reads to filter `weapon_visual` out of `ComponentList`.
///
/// **Always rebroadcasts**, even when the cached flag matches the
/// incoming `holstered`. The earlier short-circuit was wrong:
/// re-aggro inside the 10-second OOC grace window leaves the weapon
/// drawn (we deliberately defer the holster), so the cached flag is
/// still `false` when the next combat-enter fires its draw kick. A
/// same-state short-circuit would suppress the `BeingAppearance` that
/// triggers the client's unholster animation, leaving the second
/// combat enter's draw silent. The wire cost is one packet per
/// combat transition — well within budget.
#[tracing::instrument(
    name = "bandolier.refresh_appearance",
    level = "info",
    skip_all,
    fields(entity_id, player_id, holstered)
)]
pub(super) async fn refresh_appearance(
    entity_id: u32,
    player_id: i32,
    holstered: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Update the cached flag so any subsequent refresh fired by an
    // unrelated path (item equip, slot swap, world re-entry) picks up
    // the live holster state. The flag write itself is unconditional;
    // the rebroadcast below is too — see fn doc for why we don't
    // short-circuit on no-change.
    {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => {
                tracing::debug!(
                    entity_id,
                    player_id,
                    holstered,
                    "RefreshAppearance: entity has no transport addr (disconnected?), skipping"
                );
                return;
            }
        };
        let mut clients = connected.lock().unwrap();
        match clients.get_mut(&addr) {
            Some(c) => {
                c.weapon_holstered = holstered;
            }
            None => {
                tracing::debug!(
                    entity_id,
                    player_id,
                    holstered,
                    "RefreshAppearance: client state missing for addr, skipping"
                );
                return;
            }
        }
    }

    tracing::info!(
        entity_id,
        player_id,
        holstered,
        "RefreshAppearance: cached holster flag updated; broadcasting BeingAppearance"
    );

    super::super::methods::inventory::refresh_player_appearance(
        entity_id,
        player_id,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::BandolierAmmoUpdate` — persist a per-shot ammo writeback
/// from the cell. Validates bounds (slot in 0..5, ammo / type non-negative,
/// expected_instance_id positive) at the service boundary so out-of-range
/// values can't become durable corruption.
#[tracing::instrument(
    name = "bandolier.ammo_update",
    level = "debug",
    skip_all,
    fields(player_id, slot_id, expected_instance_id, current_ammo, cur_ammo_type)
)]
pub(super) async fn bandolier_ammo_update(
    player_id: i32,
    slot_id: i32,
    expected_instance_id: i32,
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
    // ammo_type IDs are non-negative. `expected_instance_id` is the
    // `sgw_inventory.item_id` PK and is always positive for a real row.
    if !(0..5).contains(&slot_id)
        || current_ammo < 0
        || cur_ammo_type < 0
        || expected_instance_id <= 0
    {
        tracing::warn!(
            player_id,
            slot_id,
            expected_instance_id,
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
            expected_instance_id,
            current_ammo,
            cur_ammo_type,
        )
        .await
        {
            tracing::warn!(
                player_id, slot_id, expected_instance_id, current_ammo, cur_ammo_type, error = %e,
                "BandolierAmmoUpdate: DB write failed"
            );
        }
    }
}
