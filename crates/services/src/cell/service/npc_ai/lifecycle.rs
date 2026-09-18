//! Terminal / quiescent states: despawn (remove from space), submit
//! (surrender + hold), and error (diagnostic hold).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};

use super::record_decision_outcome;

/// NPC despawn behavior: remove the entity from the space. Used by
/// scripted cleanup (e.g., "the boss died, his bodyguards retreat
/// off-screen"). Fans `LeftAoI` to every witness immediately.
///
/// C08b (2026-09-18): switched from the bare `SpaceManager::destroy_entity`
/// to `despawn_npc` — the bare call left the entity in every observer's
/// `witnesses` set until the next AoI tick happened to visit them (this
/// function's own doc comment used to claim immediate fanout, which was
/// false; see `content::executor::world::destroy_tagged_entity`'s doc
/// comment for the full failure-shape writeup, issue #582).
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
    match space_mgr.despawn_npc(npc_id, tx).await {
        DespawnOutcome::Despawned { witnesses_notified } => {
            tracing::info!(
                npc_id,
                witnesses_notified,
                "NPC AI: despawn → removed entity from space"
            );
        }
        DespawnOutcome::RefusedPlayer => {
            // Scripted AI cleanup should never target a player entity;
            // WARN loudly rather than silently no-opping.
            tracing::warn!(
                npc_id,
                "NPC AI: despawn target resolved to a player entity -- refused"
            );
        }
        DespawnOutcome::NotFound => {
            tracing::debug!(npc_id, "NPC AI: despawn target already gone");
        }
    }
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
