//! The one path every crafting verb takes to the base.

use tokio::sync::mpsc;

use crate::cell::interactions::crafting_stations::{station_mask, stations_in_range};
use crate::cell::messages::{CellToBaseMsg, CraftRequest, CraftVerb};
use crate::cell::space_manager::SpaceManager;

/// Forward `verb` for `entity_id` as a `CellToBaseMsg::Crafting`.
///
/// `allowed` is the station gate (CR-05): the verbs whose station is in
/// reach right now, computed here rather than read from the 1 Hz station
/// tick, so a player who walked away a moment ago is not let through. The
/// base adds the tools and "craft anywhere" to it.
///
/// An entity that is not a loaded character (no `player_id`) cannot have
/// sent a crafting method through a legitimate client, so the request is
/// dropped with a WARN.
pub(super) async fn forward(
    entity_id: u32,
    verb: CraftVerb,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
        tracing::warn!(
            target: "crafting",
            event = "no_player",
            entity_id,
            method = verb.method_name(),
            "crafting request from an entity with no player_id; dropped"
        );
        return;
    };
    let request = CraftRequest {
        entity_id,
        player_id,
        verb,
        allowed: station_mask(&stations_in_range(space_mgr, entity_id)),
    };
    if let Err(e) = tx.send(CellToBaseMsg::Crafting(request)).await {
        tracing::warn!(
            target: "crafting",
            event = "forward_failed",
            entity_id,
            player_id,
            error = %e,
            "crafting request could not be queued (base channel closed)"
        );
    }
}
