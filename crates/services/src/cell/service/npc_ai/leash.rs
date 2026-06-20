//! Leashing state: reset the NPC to Idle and restore health (snap home).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// NPC leashing behavior: reset to Idle and restore health.
///
/// In a full implementation this would pathfind the NPC back to spawn.
/// For now we snap back instantly and restore health.
pub(super) async fn npc_ai_leash(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    // The Fighting → Leashing transition site in `npc_ai_fight`
    // already broadcasts Leash, so this is a no-op in the normal
    // path — but for completeness (and for the future when leash
    // becomes a multi-tick walk-back rather than a snap) call it
    // here too. Dedup'd by `last_movement_type`.
    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Leash),
        tx,
        space_mgr,
    )
    .await;

    let (stat_update, state_field) = {
        let npc = match space_mgr.get_entity_mut(npc_id) {
            Some(e) => e,
            None => return,
        };

        // Snap back to spawn position
        if let Some(spawn_pos) = npc.spawn_position {
            npc.position = spawn_pos;
        }

        // Restore health to max
        if let Some(health) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            health.set_current(health.max);
        }

        npc.ai_state = AiState::Idle;
        npc.threat_list.clear();
        npc.abilities.clear_all_cooldowns();

        // No state-flag unsetting here: leash only fires when the NPC is
        // alive (the AI state machine routes dead NPCs to AiState::Dead, not
        // Leashing). BSF_DEAD/BSF_MOVEMENT_LOCK were never set in the first
        // place on a leashing NPC, so unsetting them would be defensive
        // paranoia against an unreachable code path.

        tracing::info!(
            npc_id,
            "NPC AI: leash complete, reset to Idle with full health"
        );

        // Collect data before dropping the mutable borrow
        let stat_update = npc.stats.serialize_dirty();
        npc.stats.clear_dirty();
        let state_field = npc.state_field;
        (stat_update, state_field)
    };

    crate::cell::abilities::send_entity_method(npc_id, 20, stat_update, tx, space_mgr).await;

    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&state_field.to_le_bytes());
    crate::cell::abilities::send_entity_method(npc_id, 19, state_args, tx, space_mgr).await;

    // Leash complete — clear the cached movement-type so the next
    // Fighting transition re-broadcasts CombatAdvance. None emits no
    // wire byte (client keeps its idle pose); only the dedup cache
    // resets. See `broadcast_movement_type` doc.
    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
}
