//! Crafting-station tick (CR-05): tells the base which stations are in
//! reach of each player, so it can keep `onUpdateCraftingOptions` current.
//!
//! **Why a 1 Hz tick, not the movement handler.** The station set changes
//! for four reasons: the player moves, a station spawns or despawns, the
//! player changes world, or the player's cell entity is re-created. Only
//! the first comes through the movement handler; a sweep catches all four
//! at one seam. It costs one spatial-grid query of radius
//! `MAX_INTERACT_DISTANCE` per player per second. The report feeds the
//! client's crafting-window label only: the station gate itself is
//! recomputed at request time by the crafting forward, so a second of lag
//! here can never allow or refuse a request.
//!
//! A report goes out only when a player's set differs from the last one
//! sent ([`CraftingStationState::record`]); a player's first tick after its
//! entity is created always reports.
//!
//! [`CraftingStationState::record`]: cimmeria_entity::cell_entity::CraftingStationState::record

use tokio::sync::mpsc;

use crate::cell::interactions::crafting_stations::stations_in_range;
use crate::cell::messages::{CellToBaseMsg, CraftingStations};
use crate::cell::space_manager::SpaceManager;

/// Run one station sweep over every loaded player.
pub(in crate::cell::service) async fn crafting_station_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut reports = Vec::new();
    for entity_id in space_mgr.all_player_entity_ids() {
        // A player with no `player_id` has not finished world entry; it is
        // left unrecorded so it reports once it has one.
        let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
            continue;
        };
        let stations = stations_in_range(space_mgr, entity_id);
        let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
            continue;
        };
        if entity.crafting_stations.record(stations) {
            reports.push(CraftingStations {
                entity_id,
                player_id,
                stations,
            });
        }
    }

    for report in reports {
        tracing::debug!(
            target: "crafting",
            event = "stations_changed",
            entity_id = report.entity_id,
            player_id = report.player_id,
            stations = ?report.stations,
            "crafting stations in range changed"
        );
        let entity_id = report.entity_id;
        if let Err(e) = tx.send(CellToBaseMsg::CraftingStations(report)).await {
            tracing::warn!(
                target: "crafting",
                event = "stations_send_failed",
                entity_id,
                error = %e,
                "crafting station report could not be queued (base channel closed)"
            );
        }
    }
}

#[cfg(test)]
#[path = "crafting_stations_tests.rs"]
mod tests;
