//! Giving up a fight: the Fighting -> Leashing entry, the route home, and the
//! player-side combat drain every threat clear owes.

use std::time::Instant;

use cimmeria_common::Vector3;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::policy::{horizontal_distance, LEASH_ARRIVE_RADIUS};

/// Send `onStateFieldUpdate` to each player whose `BSF_InCombat` a drain just
/// cleared. `send_entity_method` routes a player method to that player's own
/// client. No appearance refresh: `exit_player_combat` armed the out-of-combat
/// holster timer instead of flipping the holster, the same as the death path.
async fn send_combat_exits(
    exits: Vec<(u32, u32)>,
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    for (player_id, new_state) in exits {
        tracing::debug!(
            target: "npc_ai.leash",
            event = "player_combat_exit",
            npc_id,
            player_id,
            new_state,
            "NPC gave up: player's last threatening mob drained, BSF_InCombat cleared"
        );
        crate::cell::abilities::send_entity_method(
            player_id,
            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
            new_state.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }
}

/// Drop `npc_id` from every player that still counts it as a threat, and tell
/// the players who just left combat. Call it **before** clearing the NPC's
/// threat list: the drain reads that list (audit S7).
pub(in crate::cell::service::npc_ai) async fn drain_player_combat(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let exits = crate::cell::combat::drain_npc_from_player_combat(space_mgr, npc_id);
    send_combat_exits(exits, npc_id, tx, space_mgr).await;
}

/// Remove one target from the NPC's threat list and take the NPC off that
/// player's combat set. Used when a target dies, disconnects or is lost; the
/// fight goes on against whoever is left.
pub(in crate::cell::service::npc_ai) async fn drop_threat_target(
    npc_id: u32,
    target_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.threat_list.remove(&target_id);
    }
    let exits: Vec<(u32, u32)> =
        crate::cell::combat::exit_player_combat(space_mgr, target_id, npc_id)
            .map(|state| (target_id, state))
            .into_iter()
            .collect();
    send_combat_exits(exits, npc_id, tx, space_mgr).await;
}

/// Plan the route from `from` to `spawn` and install it. Returns the number of
/// waypoints installed, `0` when no route exists (no navmesh, no start or end
/// polygon, or a one-point path), in which case the NPC's route is left empty
/// and the leash tick snaps it home.
pub(in crate::cell::service::npc_ai) fn plan_home_path(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    from: Vector3,
    spawn: Vector3,
) -> usize {
    if horizontal_distance(&from, &spawn) <= LEASH_ARRIVE_RADIUS {
        return 0;
    }
    // Through NA02's request logger, so the walk home shows up as
    // `npc_ai.path event=request state=leash` beside every other route.
    let routed = super::super::path_request::request_path(
        space_mgr,
        super::super::path_request::PathRequest {
            npc_id,
            state: "leash",
            from,
            to: spawn,
            target_id: None,
            partial_outcome: "leash_partial",
        },
        Instant::now(),
    );
    let Some(path) = routed.waypoints else {
        return 0;
    };
    if path.len() < 2 {
        return 0;
    }
    let waypoints: Vec<Vector3> = path.into_iter().skip(1).collect();
    let n = waypoints.len();
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        super::super::replace_nav_path_on(npc, waypoints);
    }
    n
}

/// Where the NPC, its spawn and its target stood when the leash fired. Only
/// for the decision row.
pub(in crate::cell::service::npc_ai) struct LeashOutAt {
    pub spawn: Vector3,
    pub npc_pos: Vector3,
    pub target_pos: Vector3,
    pub leash_distance: f32,
}

/// The fight handler's leash: the NPC went past its leash radius
/// ([`super::policy::leash_trigger`]). Logs the `decision_outcome=leashed`
/// row, then [`begin_leash`].
pub(in crate::cell::service::npc_ai) async fn leash_out(
    npc_id: u32,
    target_id: u32,
    trigger: super::policy::LeashTrigger,
    at: LeashOutAt,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    super::super::note_outcome("leashed");
    // Distances are named by their endpoints (T7). `npc_to_spawn` is the
    // horizontal distance the leash measures; `target_to_spawn` is kept for
    // comparison with rows from before NA12, when it was the metric.
    tracing::info!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = "leashed",
        npc_id,
        target_id,
        trigger = trigger.label(),
        target_to_spawn = at.spawn.distance_to(&at.target_pos),
        npc_to_spawn = horizontal_distance(&at.npc_pos, &at.spawn),
        npc_dy_from_spawn = at.npc_pos.y - at.spawn.y,
        leash_distance = at.leash_distance,
        "NPC AI: NPC past its leash radius, walking home"
    );
    begin_leash(
        npc_id,
        super::super::AiTransitionReason::LeashOut,
        trigger.label(),
        Some((target_id, at.target_pos)),
        tx,
        space_mgr,
    )
    .await;
}

/// Enter `Leashing`: the NPC stops fighting, drains every player's combat
/// state for it, releases cover and starts walking home.
///
/// Order matters. The drain reads the threat list, so it runs first; the
/// transition row is written next so its `threat_count` shows what was
/// dropped; the transition stops the NPC (NA10), so the route home is
/// installed after it.
///
/// A follower (`follow_target_id` set) and an NPC with no spawn point get no
/// route: the leash tick resets them where they stand, as before.
pub(in crate::cell::service::npc_ai) async fn begin_leash(
    npc_id: u32,
    reason: super::super::AiTransitionReason,
    trigger: &'static str,
    target: Option<(u32, Vector3)>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use super::super::detectors::threat::ThreatClear;
    use super::super::AiTransitionReason as R;
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    drain_player_combat(npc_id, tx, space_mgr).await;

    let world = super::super::world_label(space_mgr, npc_id);
    let (npc_pos, home, leash_distance) = {
        let Some(npc) = space_mgr.get_entity_mut(npc_id) else {
            return;
        };
        super::super::set_ai_state_on(npc, &world, AiState::Leashing, reason);
        npc.threat_list.clear();
        npc.ai_retry_at = None;
        npc.leash.target_lost_since = None;
        npc.leash.walk_started_at = Some(Instant::now());
        (
            npc.position,
            npc.spawn_position
                .filter(|_| npc.follow_target_id.is_none()),
            super::policy::leash_radius(npc.leash.distance_override),
        )
    };
    let now = Instant::now();
    // NA02's S7 detector: after the drain above this must find nobody.
    let clear = match reason {
        R::LeashOut => ThreatClear::LeashOut,
        R::TargetLost => ThreatClear::TargetLost,
        _ => ThreatClear::ThreatEmpty,
    };
    super::super::detectors::threat::check_cleared(space_mgr, npc_id, clear, now);

    // Leash is a combat-end transition: give the cover slot back.
    space_mgr
        .cover
        .release_for_entity(cimmeria_common::EntityId(npc_id as i32));
    // Cache only: no movement-type message exists server-to-client (NA10).
    // The client sees the walk home from position and velocity.
    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Leash),
        tx,
        space_mgr,
    )
    .await;

    if let Some(spawn) = home {
        plan_home_path(space_mgr, npc_id, npc_pos, spawn);
    }
    // NA02's `enter` row (and its leash-loop counter), written after the
    // route is installed: `nav_path_len = 0` with a spawn means no route,
    // and the next leash tick snaps the NPC home (`event=snap_fallback`).
    super::super::detectors::leash::on_enter(
        space_mgr,
        npc_id,
        super::super::detectors::leash::LeashEntry {
            target,
            leash_distance,
            reason: reason.label(),
            trigger,
        },
        now,
    );
}
