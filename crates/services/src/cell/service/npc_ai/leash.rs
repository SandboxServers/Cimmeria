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

    // The Fighting → Leashing transition site in `npc_ai_fight` already
    // records Leash, so this is a no-op in the normal path. Cache only:
    // nothing reaches the client (NA10, see `broadcast_movement_type`).
    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Leash),
        tx,
        space_mgr,
    )
    .await;

    // Snap back to spawn position -- but NOT for a follower. A
    // fighting escort NPC (follow_target_id still set — Follow
    // doesn't auto-clear on threat preemption, see
    // `Action::SetFollowTarget` doc) that got yanked back to
    // spawn_position here would be stranded: Follow doesn't
    // auto-resume post-fight either, so nothing would walk it back
    // to the player, and it would sit at spawn until a content
    // chain re-fires SetFollowTarget. Leaving it at its
    // leash-time position keeps it near the player it was
    // escorting instead of teleporting it away (GC1b-0 hardening).
    //
    // The snap goes through the grid-updating position writer. It used to
    // write `npc.position` directly, which left the AoI spatial grid
    // indexing the NPC at its chase position. It also left the chase path
    // and velocity in place, so the movement tick walked the NPC from
    // spawn back out along the stale route (NA10, audit S4). The authored
    // spawn facing is restored the way the respawn tick restores it.
    let (snap_to, spawn_facing, from) = match space_mgr.get_entity(npc_id) {
        Some(npc) => (
            npc.spawn_position
                .filter(|_| npc.follow_target_id.is_none()),
            npc.spawn_direction,
            npc.position,
        ),
        None => return,
    };
    let snapped = snap_to.is_some();
    match snap_to {
        Some(spawn_pos) => super::snap_npc_to(space_mgr, npc_id, spawn_pos, spawn_facing),
        // A follower stays where it is, but it still stops.
        None => {
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                super::stop_movement_on(npc);
            }
        }
    }

    let world = super::world_label(space_mgr, npc_id);
    let (stat_update, state_field) = {
        let npc = match space_mgr.get_entity_mut(npc_id) {
            Some(e) => e,
            None => return,
        };

        // Restore health to max
        if let Some(health) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            health.set_current(health.max);
        }

        // Today's leash is an instant snap, so "arrived" is immediate.
        super::set_ai_state_on(
            npc,
            &world,
            AiState::Idle,
            super::AiTransitionReason::LeashArrived,
        );
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
    super::detectors::leash::on_complete(space_mgr, npc_id, from, snapped);
    super::detectors::threat::check_cleared(
        space_mgr,
        npc_id,
        super::detectors::threat::ThreatClear::LeashComplete,
        std::time::Instant::now(),
    );

    crate::cell::abilities::send_entity_method(npc_id, 20, stat_update, tx, space_mgr).await;

    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&state_field.to_le_bytes());
    crate::cell::abilities::send_entity_method(npc_id, 19, state_args, tx, space_mgr).await;

    // Leash complete: clear the cached movement type. The client never saw
    // it; what shows the NPC standing is the zero velocity the snap wrote.
    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
}
