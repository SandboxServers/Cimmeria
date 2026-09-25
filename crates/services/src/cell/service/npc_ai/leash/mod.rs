//! Leashing: the walk home after a fight, the evade while walking, and the
//! reset on arrival (NA12, D-NA03 as corrected by D-NA10).
//!
//! - [`policy`] decides when a fighting NPC gives up, and holds the numbers.
//! - [`begin`] is the Fighting -> Leashing entry: drain player combat, clear
//!   threat, release cover, install the route home.
//! - This file is the per-tick Leashing handler: keep walking, replan a stale
//!   route, and on arrival heal, face the authored heading, clear cooldowns
//!   and go Idle. It snaps home only when there is no route or the walk times
//!   out.
//!
//! While Leashing the NPC evades: `combat::generate_threat` refuses it and
//! logs `npc_ai.leash event=damage_ignored`.

mod begin;
pub(in crate::cell::service) mod policy;

pub(super) use begin::{begin_leash, drop_threat_target, leash_out, LeashOutAt};

use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use policy::{horizontal_distance, LEASH_ARRIVE_RADIUS, LEASH_WALK_TIMEOUT, REAGGRO_SUPPRESSION};

/// How a leash ended. The label is the `arrival` field on the
/// `npc_ai.leash event=arrived` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arrival {
    /// Walked the route and reached spawn.
    Walked,
    /// No route home could be planned: snapped.
    SnapNoPath,
    /// The walk took longer than [`LEASH_WALK_TIMEOUT`]: snapped.
    SnapTimeout,
    /// Spawn is on another mesh island: the NPC walked the partial route to
    /// its end, then snapped (NA15).
    SnapPartialRoute,
    /// A follower, or an NPC with no spawn point: reset where it stands.
    InPlace,
}

impl Arrival {
    fn label(self) -> &'static str {
        match self {
            Self::Walked => "walked",
            Self::SnapNoPath => "snap_no_path",
            Self::SnapTimeout => "snap_timeout",
            Self::SnapPartialRoute => "snap_partial_route",
            Self::InPlace => "in_place",
        }
    }

    fn is_snap(self) -> bool {
        matches!(
            self,
            Self::SnapNoPath | Self::SnapTimeout | Self::SnapPartialRoute
        )
    }
}

/// Per-tick Leashing handler.
pub(super) async fn npc_ai_leash(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = Instant::now();
    let Some(npc) = space_mgr.get_entity(npc_id) else {
        return;
    };
    let pos = npc.position;
    // A follower is not walked or snapped home. A fighting escort NPC
    // (Follow does not clear `follow_target_id` on a threat preempt) that
    // was sent back to spawn would be stranded there: Follow does not
    // resume after a fight, so nothing would walk it back to the player
    // (GC1b-0 hardening).
    let home = npc
        .spawn_position
        .filter(|_| npc.follow_target_id.is_none());
    let route_end = npc.nav_path.back().copied();
    let started = npc.leash.walk_started_at;
    let home_route_partial = npc.leash.home_route_partial;

    let Some(spawn) = home else {
        arrive(npc_id, Arrival::InPlace, tx, space_mgr).await;
        return;
    };

    // Leashing reached without `begin_leash` (content `set_npc_ai_state`,
    // the GM console) has no walk clock yet: start it now.
    let started = match started {
        Some(t) => t,
        None => {
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.leash.walk_started_at = Some(now);
            }
            now
        }
    };

    if route_end.is_none() && horizontal_distance(&pos, &spawn) <= LEASH_ARRIVE_RADIUS {
        arrive(npc_id, Arrival::Walked, tx, space_mgr).await;
        return;
    }
    if now.saturating_duration_since(started) >= LEASH_WALK_TIMEOUT {
        arrive(npc_id, Arrival::SnapTimeout, tx, space_mgr).await;
        return;
    }
    // The end of a partial route home: spawn is on another mesh island, and
    // replanning from here would only find the same island edge again until
    // the walk timeout. Snap now (NA15).
    if route_end.is_none() && home_route_partial {
        arrive(npc_id, Arrival::SnapPartialRoute, tx, space_mgr).await;
        return;
    }
    // Only a route that ends at home (or the partial route planned toward
    // it) is a walk home. Anything else (none at all, or a chase route left
    // behind by a Leashing entry that skipped the transition's stop) is
    // replanned from where the NPC stands.
    let heading_home = home_route_partial && route_end.is_some()
        || route_end
            .is_some_and(|end| horizontal_distance(&end, &spawn) <= LEASH_ARRIVE_RADIUS * 2.0);
    if heading_home {
        super::note_outcome("leash_walking");
        return;
    }
    if begin::plan_home_path(space_mgr, npc_id, pos, spawn) > 0 {
        super::note_outcome("leash_replan");
        tracing::debug!(
            target: "npc_ai.leash",
            event = "replan",
            npc_id,
            had_route = route_end.is_some(),
            npc_to_spawn = horizontal_distance(&pos, &spawn),
            "NPC leashing without a route home: planned one"
        );
        return;
    }
    arrive(npc_id, Arrival::SnapNoPath, tx, space_mgr).await;
}

/// The reset at the end of a leash: put the NPC home (snap only on the
/// fallback arms), heal it, restore the authored facing, clear threat and
/// cooldowns, go Idle, and open the re-aggro suppression window.
async fn arrive(
    npc_id: u32,
    how: Arrival,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;

    let now = Instant::now();
    let (from, home, spawn_facing, walk_secs) = match space_mgr.get_entity(npc_id) {
        Some(npc) => (
            npc.position,
            npc.spawn_position,
            npc.spawn_direction,
            npc.leash
                .walk_started_at
                .map(|t| now.saturating_duration_since(t).as_secs_f32()),
        ),
        None => return,
    };

    match (how.is_snap(), home) {
        // Through the grid-updating writer, which also stops the NPC (NA10).
        (true, Some(spawn)) => super::snap_npc_to(space_mgr, npc_id, spawn, spawn_facing),
        _ => {
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                super::stop_movement_on(npc);
                if how == Arrival::Walked {
                    if let Some(dir) = spawn_facing {
                        npc.direction = dir;
                    }
                }
            }
        }
    }

    // A Leashing NPC normally has an empty threat list (`begin_leash`
    // drained and cleared it, and the evade refuses new threat), but one put
    // into Leashing by content or the GM console may not. Draining again is
    // idempotent.
    drain_player_combat_then_clear(npc_id, tx, space_mgr).await;

    let world = super::world_label(space_mgr, npc_id);
    let reason = if how.is_snap() {
        super::AiTransitionReason::LeashSnapFallback
    } else {
        super::AiTransitionReason::LeashArrived
    };
    let (stat_update, state_field) = {
        let Some(npc) = space_mgr.get_entity_mut(npc_id) else {
            return;
        };
        if let Some(health) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            health.set_current(health.max);
        }
        super::set_ai_state_on(npc, &world, AiState::Idle, reason);
        npc.abilities.clear_all_cooldowns();
        npc.ai_retry_at = None;
        npc.leash.walk_started_at = None;
        npc.leash.target_lost_since = None;
        npc.leash.home_route_partial = false;
        npc.leash.reaggro_suppressed_until = Some(now + REAGGRO_SUPPRESSION);

        // No state-flag unsetting: a leashing NPC is alive, so BSF_DEAD and
        // BSF_MOVEMENT_LOCK were never set on it.
        let stat_update = npc.stats.serialize_dirty();
        npc.stats.clear_dirty();
        (stat_update, npc.state_field)
    };

    // Back at spawn: an NPC authored in cover takes its slot again (NA22).
    crate::cell::cover::hold_spawn_cover(space_mgr, npc_id, "leash_home");

    let outcome = if how.is_snap() {
        "leash_snap_fallback"
    } else {
        "leash_arrived"
    };
    super::note_outcome(outcome);
    // NA02's rows: `arrived` for a walk or an in-place reset,
    // `snap_fallback` (with `snap_dist`, the size of the pop the client saw)
    // for a snap. Then the S7 detector, which must find nobody after the
    // drain above.
    super::detectors::leash::on_complete(
        space_mgr,
        npc_id,
        super::detectors::leash::LeashEnd {
            from,
            arrival: how.label(),
            snapped: how.is_snap() && home.is_some(),
            walk_secs,
        },
    );
    super::detectors::threat::check_cleared(
        space_mgr,
        npc_id,
        super::detectors::threat::ThreatClear::LeashComplete,
        now,
    );

    crate::cell::abilities::send_entity_method(
        npc_id,
        crate::mercury::method_idx::ON_STAT_UPDATE,
        stat_update,
        tx,
        space_mgr,
    )
    .await;
    crate::cell::abilities::send_entity_method(
        npc_id,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        state_field.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;

    // Leash over: clear the cached movement type. Nothing reaches the client
    // (NA10); what shows the NPC standing is the zero velocity above.
    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
}

/// [`begin::drain_player_combat`], then clear the NPC's threat list.
async fn drain_player_combat_then_clear(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    begin::drain_player_combat(npc_id, tx, space_mgr).await;
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.threat_list.clear();
    }
}
