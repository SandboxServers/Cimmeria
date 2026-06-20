//! Terminal / quiescent states: despawn (remove from space), submit
//! (surrender + hold), and error (diagnostic hold).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::record_decision_outcome;

/// NPC despawn behavior: remove the entity from the space. Used by
/// scripted cleanup (e.g., "the boss died, his bodyguards retreat
/// off-screen"). The destroy fires AoI-left events to all witnesses.
///
/// One-shot: the entity is gone by the time this returns, so any
/// subsequent tick filters skip it naturally.
pub(super) async fn npc_ai_despawn(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    record_decision_outcome("despawn");
    // Clear the movement-type cache first so the wire state is clean
    // before the destroy. The broadcast itself is dedup'd on None and
    // emits nothing — this is purely a state-clean step.
    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
    tracing::info!(npc_id, "NPC AI: despawn → removing entity from space");
    space_mgr.destroy_entity(npc_id);
}

/// NPC submit behavior: the NPC surrenders. Clears combat state and
/// holds position. The AI tick will keep admitting Submit on every
/// pass (since the snapshot filter permits it), so the handler stays
/// cheap — broadcast None once, no further work. Content authors
/// destroy or transition the NPC when they're done with it.
pub(super) async fn npc_ai_submit(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use crate::cell::combat;
    record_decision_outcome("submit_init");
    // Cache check: only do the heavy work on first entry. After that,
    // last_movement_type is None and we early-out.
    let needs_init = space_mgr
        .get_entity(npc_id)
        .is_some_and(|e| e.last_movement_type.is_some() || !e.threat_list.is_empty());
    if !needs_init {
        return;
    }
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.threat_list.clear();
        npc.nav_path.clear();
        npc.velocity = [0.0; 3];
        npc.state_field &= !combat::BSF_IN_COMBAT;
    }
    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
    tracing::info!(npc_id, "NPC AI: submit → combat state cleared, holding");
}

/// NPC error behavior: diagnostic fallback. Halts AI work (no
/// pathfind, no broadcast cadence). Logged once per entry so a stuck
/// NPC doesn't fill the log stream. Used by the `enterErrorAIState`
/// slash command and by the AI tick when it catches an unrecoverable
/// inconsistency (future).
pub(super) async fn npc_ai_error(
    npc_id: u32,
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    record_decision_outcome("error_hold");
    // No-op per tick — Error is a quiescent diagnostic state. The
    // entry log is emitted by whatever transitioned the NPC into
    // Error (typically the content action or the slash command).
    tracing::debug!(npc_id, "NPC AI: error state — holding");
}
