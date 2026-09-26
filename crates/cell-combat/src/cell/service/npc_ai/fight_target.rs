//! Fight target selection: pick the top-threat target that is still worth
//! fighting, prune the ones that are not, and send the NPC home when nobody
//! is left (NA12, audit S6).
//!
//! Split out of `fight.rs` along the target-selection seam named in the
//! restoration ledger's dispatch rules.

use std::time::Instant;

use cimmeria_common::Vector3;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::leash::policy as leash_policy;

/// What the rest of the fight tick needs about the NPC and its target.
pub(super) struct Engagement {
    pub target_id: u32,
    pub target_pos: Vector3,
    pub spawn_pos: Option<Vector3>,
    pub npc_pos: Vector3,
    pub is_stationary: bool,
    pub use_cover: bool,
    /// The NPC's resolved leash radius (template override or default).
    pub leash_distance: f32,
}

/// Why a threat target was dropped. The label is the `trigger` on the
/// `npc_ai.leash event=begin` row when the drop empties the threat list.
#[derive(Debug, Clone, Copy)]
enum Dropped {
    /// The target entity no longer exists (disconnected, despawned).
    Gone,
    /// The target is at or below zero health.
    Dead,
    /// The target stayed beyond the NPC's AoI for the grace period.
    OutOfPerception,
}

impl Dropped {
    fn label(self) -> &'static str {
        match self {
            Self::Gone => "target_gone",
            Self::Dead => "target_dead",
            Self::OutOfPerception => "target_out_of_aoi",
        }
    }
}

/// The highest-threat live target, or `None` when the fight is over.
///
/// Dead, vanished and lost targets are removed from the threat list **and**
/// the NPC is removed from their combat set, so their `BSF_InCombat` clears
/// and regen resumes (audit S7). When the list runs dry the NPC starts its
/// walk home (`Leashing`) instead of going Idle where it stands, which used
/// to park it passive and frozen after every player death (audit S6).
pub(super) async fn select_target(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<Engagement> {
    let now = Instant::now();
    let mut last_drop: Option<Dropped> = None;
    loop {
        let npc = space_mgr.get_entity(npc_id)?;
        let top = npc
            .threat_list
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&eid, _)| eid);
        let npc_pos = npc.position;
        let aoi_radius = npc.aoi_radius;
        let lost_since = npc.leash.target_lost_since;
        let engagement = |target_id, target_pos| Engagement {
            target_id,
            target_pos,
            spawn_pos: npc.spawn_position,
            npc_pos,
            is_stationary: npc.is_stationary,
            use_cover: npc.use_cover,
            leash_distance: leash_policy::leash_radius(npc.leash.distance_override),
        };

        let Some(target_id) = top else {
            let (reason, trigger) = match last_drop {
                Some(d) => (super::AiTransitionReason::TargetLost, d.label()),
                None => (super::AiTransitionReason::ThreatEmpty, "threat_empty"),
            };
            super::note_outcome(reason.label());
            super::leash::begin_leash(npc_id, reason, trigger, None, tx, space_mgr).await;
            return None;
        };

        let dropped = match space_mgr.get_entity(target_id) {
            None => Dropped::Gone,
            // `BSF_DEAD` as well as HEALTH (NA24, UAT-1 A): a player corpse
            // healed during the Defeat Window reads as alive by HEALTH alone,
            // and the guard kept it as its target through the respawn.
            Some(t)
                if crate::cell::combat::is_dead_state(t.state_field)
                    || t.stats
                        .get(cimmeria_entity::stats::HEALTH)
                        .is_none_or(|s| s.cur <= 0) =>
            {
                Dropped::Dead
            }
            Some(t) => {
                let target_pos = t.position;
                let out = leash_policy::target_out_of_perception(
                    npc_pos.distance_to(&target_pos),
                    aoi_radius,
                );
                match (out, lost_since) {
                    (true, Some(since)) if leash_policy::target_lost_for_grace(since, now) => {
                        Dropped::OutOfPerception
                    }
                    (true, _) => {
                        let e = engagement(target_id, target_pos);
                        if lost_since.is_none() {
                            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                                npc.leash.target_lost_since = Some(now);
                            }
                        }
                        return Some(e);
                    }
                    (false, _) => {
                        let e = engagement(target_id, target_pos);
                        if lost_since.is_some() {
                            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                                npc.leash.target_lost_since = None;
                            }
                        }
                        return Some(e);
                    }
                }
            }
        };

        tracing::debug!(
            target: "npc_ai",
            event = "target_dropped",
            npc_id,
            target_id,
            why = dropped.label(),
            "NPC AI: dropping threat target"
        );
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.leash.target_lost_since = None;
        }
        super::leash::drop_threat_target(npc_id, target_id, tx, space_mgr).await;
        last_drop = Some(dropped);
    }
}

/// Take a player who just died off every NPC's threat list, at the moment of
/// death (NA24, UAT-1 A).
///
/// Before this the drop waited for each NPC's next fight pass, up to two
/// seconds later, and was decided by reading the target's HEALTH. A dead
/// player who used a medkit in that window read as alive, the pass kept it,
/// the attack was refused (the target is dead), the 500 ms retry sweep kept
/// the NPC `Fighting`, and after the respawn the NPC chased the living player
/// across the floor. Purging here leaves nothing for a later pass — natural
/// tick or retry sweep — to re-acquire; only a fresh aggro scan can engage the
/// respawned player again, and that scan refuses dead candidates.
///
/// Each drop goes through [`super::leash::drop_threat_target`], so the corpse
/// leaves the NPC's combat set exactly as a pruned target does. An NPC whose
/// list runs dry starts its walk home now, with the same `target_dead`
/// trigger [`select_target`] would have written.
pub(in crate::cell) async fn purge_dead_player_from_threat(
    player_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;

    let holders: Vec<u32> = space_mgr
        .all_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .is_some_and(|e| !e.is_player && e.threat_list.contains_key(&player_id))
        })
        .collect();
    for npc_id in holders {
        tracing::debug!(
            target: "npc_ai",
            event = "target_dropped",
            npc_id,
            target_id = player_id,
            why = Dropped::Dead.label(),
            "NPC AI: dropping threat target (target died)"
        );
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.leash.target_lost_since = None;
        }
        super::leash::drop_threat_target(npc_id, player_id, tx, space_mgr).await;
        let give_up = space_mgr
            .get_entity(npc_id)
            .is_some_and(|n| n.threat_list.is_empty() && n.ai_state() == AiState::Fighting);
        if give_up {
            super::leash::begin_leash(
                npc_id,
                super::AiTransitionReason::TargetLost,
                Dropped::Dead.label(),
                None,
                tx,
                space_mgr,
            )
            .await;
        }
    }
}
