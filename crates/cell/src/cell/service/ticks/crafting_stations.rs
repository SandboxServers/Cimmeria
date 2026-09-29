//! Crafting-station tick: tells the base which stations are in
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
use crate::cell::messages::{CellToBaseMsg, CraftingStations, PluginMsg, StationChangeCause};
use crate::cell::space_manager::SpaceManager;

/// Why a player's station set moved from `previous` to `current`: the
/// first report since the entity was created is a world change (or login);
/// a station that was in reach and no longer exists despawned; anything
/// else is movement.
fn change_cause(
    space_mgr: &SpaceManager,
    previous: Option<[Option<u32>; 4]>,
    current: &[Option<u32>; 4],
) -> StationChangeCause {
    let Some(previous) = previous else {
        return StationChangeCause::WorldChange;
    };
    let despawned = previous
        .iter()
        .flatten()
        .any(|id| !current.contains(&Some(*id)) && space_mgr.get_entity(*id).is_none());
    if despawned {
        StationChangeCause::StationDespawned
    } else {
        StationChangeCause::Moved
    }
}

/// Run one station sweep over every loaded player. Logs nothing per tick;
/// the base logs `options_changed` when a report changes what the client
/// is told.
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
        let previous = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.crafting_stations.last_reported);
        let cause = change_cause(space_mgr, previous, &stations);
        let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
            continue;
        };
        if entity.crafting_stations.record(stations) {
            reports.push(CraftingStations {
                entity_id,
                player_id,
                stations,
                cause,
            });
        }
    }

    for report in reports {
        let (entity_id, player_id) = (report.entity_id, report.player_id);
        if let Err(e) = tx.send(CellToBaseMsg::Plugin(PluginMsg::new(report))).await {
            let account_id = space_mgr.player_identity(entity_id).account_id;
            tracing::warn!(
                target: "crafting",
                event = "forward_failed",
                kind = "crafting_stations",
                account_id,
                player_id,
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
