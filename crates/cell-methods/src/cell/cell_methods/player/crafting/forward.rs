//! The one path every crafting verb takes to the base.

use tokio::sync::mpsc;

use crate::cell::messages::{CellToBaseMsg, CraftRequest, CraftVerb};
use crate::cell::space_manager::SpaceManager;

/// The `ECraftTypeFlags` mask sent while no station gate exists: nothing
/// granted. CR-05 replaces it with the gate's verdict.
const NO_STATION_GATE: u8 = 0;

/// Forward `verb` for `entity_id` as a `CellToBaseMsg::Crafting`.
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
    // TODO(CR-05): the station gate computes `allowed` from the stations in
    // range, and "craft anywhere" (D-CR17).
    let request = CraftRequest {
        entity_id,
        player_id,
        verb,
        allowed: NO_STATION_GATE,
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
