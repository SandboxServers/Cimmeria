//! The Fighting handler's cover step: ask the cover system where to stand,
//! walk there, hold the slot with Cover Stance, and report why when the
//! answer is "nowhere".
//!
//! Split out of `fight.rs` along the cover seam when NA02 added the
//! `no_cover` reasons and the `cover.selection` sample. NA22 made cover a
//! firing position: the step runs whether or not the target is in range
//! (audit C3), an NPC standing at its slot holds still and fires from it,
//! and the walk to a slot is `chase::walk_to_cover_slot`, not the target
//! chase (NA15's stop distance and unreachable hold are for targets).

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, Vector3};
use tokio::sync::mpsc;

use crate::cell::cover::{
    grant_cover_stance, horizontal, maintain_cover_for_npc_traced, release_npc_cover,
    revoke_cover_stance, CoverDecision, CoverQuery, CoverWeights, NoCoverReason, PickTrace,
    ReleaseReason, COVER_ARRIVE_RADIUS, MAX_COVER_DISTANCE, SEEK_RETRY,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `cover.selection` rows at most once per NPC per this window.
const SELECTION_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
/// `no_cover` rows at most once per NPC per this window.
const NO_COVER_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
/// Inside this horizontal distance of its slot an NPC has arrived even
/// with route left: the last waypoint is the slot, projected on the mesh.
const COVER_SNAP_RADIUS: f32 = 0.5;

/// The inputs the cover step needs from the fight handler.
pub(super) struct CoverStep {
    pub npc_id: u32,
    pub target_id: u32,
    pub npc_pos: Vector3,
    /// The NPC's `resources.worlds.world_id`: cover is indexed per world.
    pub world_id: Option<i32>,
    pub target_pos: Vector3,
    pub in_range: bool,
    /// The chosen ability's `max_range`: a slot must reach the target.
    pub attack_range: f32,
    pub use_cover: bool,
    pub is_stationary: bool,
    /// Every known ability is a melee swing: cover is not for this NPC.
    pub melee_only: bool,
}

/// Where the fight tick goes after the cover step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum CoverRoute {
    /// No cover this tick: chase or fight the target as usual.
    Target,
    /// Walking to the held slot; the route is installed. Nothing else to do
    /// this tick.
    ToSlot,
    /// Standing at the held slot, stopped, with Cover Stance: fire from
    /// here and do not chase.
    InSlot,
}

/// Run the cover step. See [`CoverRoute`].
pub(super) async fn route_via_cover(
    step: CoverStep,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) -> CoverRoute {
    let CoverStep {
        npc_id,
        target_id,
        npc_pos,
        world_id,
        target_pos,
        in_range,
        attack_range,
        use_cover,
        is_stationary,
        melee_only,
    } = step;
    // An NPC that does not use cover, cannot move, or only swings never
    // asks. Not logged: it would be one row per fight tick for a fact the
    // `spawner.npc_behaviour` spawn row already records (`use_cover`,
    // `is_stationary`).
    if !use_cover || is_stationary || melee_only {
        return CoverRoute::Target;
    }

    let now = Instant::now();
    let (decision, trace) = maintain_cover_for_npc_traced(
        CoverQuery {
            npc_id: EntityId(npc_id as i32),
            npc_pos,
            world_id,
            threat_pos: target_pos,
            in_range,
            attack_range,
            use_cover: true,
            now,
        },
        &space_mgr.cover,
        &CoverWeights::default(),
    );
    if let Some(pick) = &trace.pick {
        log_selection(space_mgr, npc_id, pick, now);
    }
    match decision {
        CoverDecision::StayInCover { pos, slot } | CoverDecision::MoveToCover { pos, slot } => {
            let picked = matches!(decision, CoverDecision::MoveToCover { .. });
            let arrived = has_arrived(space_mgr, npc_id, npc_pos, pos);
            let outcome = if picked {
                "move_to_cover"
            } else {
                "stay_in_cover"
            };
            if picked {
                tracing::info!(
                    target: "npc_ai",
                    event = "decision",
                    decision_outcome = "move_to_cover",
                    npc_id,
                    target_id,
                    chunk_id = slot.chunk_id,
                    node_id = slot.node_id,
                    in_range,
                    arrived,
                    "NPC AI: picked cover slot"
                );
            } else {
                tracing::debug!(
                    target: "npc_ai",
                    event = "decision",
                    decision_outcome = "stay_in_cover",
                    npc_id,
                    target_id,
                    chunk_id = slot.chunk_id,
                    node_id = slot.node_id,
                    arrived,
                    "NPC AI: holding cover slot"
                );
            }
            if arrived {
                // Stand still at the slot (zero velocity reaches every
                // witness on the next AoI tick) and take the stance. The
                // attack branch that follows sets the tick's outcome.
                super::stop_npc_movement(space_mgr, npc_id, super::StopReason::InCover);
                grant_cover_stance(space_mgr, npc_id);
                return CoverRoute::InSlot;
            }
            super::note_outcome(outcome);
            if super::chase::walk_to_cover_slot(space_mgr, npc_id, target_id, npc_pos, pos, now) {
                return CoverRoute::ToSlot;
            }
            // The slot cannot be reached: give it back, and do not pick
            // again for a while (the scorer would choose the same slot).
            release_npc_cover(space_mgr, npc_id, "unreachable");
            lock_defer(space_mgr, npc_id, now);
            report_release(
                space_mgr,
                npc_id,
                target_id,
                slot,
                ReleaseReason::Unreachable,
            );
            CoverRoute::Target
        }
        CoverDecision::Released { prior_slot, reason } => {
            revoke_cover_stance(space_mgr, npc_id);
            report_release(space_mgr, npc_id, target_id, prior_slot, reason);
            // Stop walking toward the abandoned slot (and zero the
            // velocity) until the re-pick lands next AI tick (NA10).
            super::stop_npc_movement(space_mgr, npc_id, super::StopReason::CoverReleased);
            if reason == ReleaseReason::Flanked {
                fire_flank_triggers(npc_id, target_id, tx, space_mgr, engine).await;
            }
            CoverRoute::Target
        }
        CoverDecision::NoCover => {
            let reason = trace.no_cover.unwrap_or(NoCoverReason::NoCandidateInRadius);
            report_no_cover(
                space_mgr,
                npc_id,
                target_id,
                reason,
                trace.pick.as_ref(),
                now,
            );
            CoverRoute::Target
        }
    }
}

/// Whether the NPC stands at its slot: within [`COVER_ARRIVE_RADIUS`] with
/// no route left to walk, or within [`COVER_SNAP_RADIUS`] regardless.
fn has_arrived(space_mgr: &SpaceManager, npc_id: u32, npc_pos: Vector3, slot: Vector3) -> bool {
    let d = horizontal(&npc_pos, &slot);
    let route_left = space_mgr
        .get_entity(npc_id)
        .is_some_and(|e| !e.nav_path.is_empty());
    d <= COVER_SNAP_RADIUS || (d <= COVER_ARRIVE_RADIUS && !route_left)
}

fn lock_defer(space_mgr: &SpaceManager, npc_id: u32, now: Instant) {
    let mut r = match space_mgr.cover.reservations.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    r.defer_seek(EntityId(npc_id as i32), now + SEEK_RETRY);
}

/// The release row: `decision_outcome = cover_released_<reason>`.
fn report_release(
    space_mgr: &SpaceManager,
    npc_id: u32,
    target_id: u32,
    slot: crate::cell::cover::CoverSlotKey,
    reason: ReleaseReason,
) {
    super::note_outcome(reason.outcome());
    tracing::info!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = reason.outcome(),
        npc_id,
        target_id,
        chunk_id = slot.chunk_id,
        node_id = slot.node_id,
        world = %super::world_label(space_mgr, npc_id),
        "NPC AI: released cover slot, re-evaluating"
    );
}

/// The `OnNpcFlanked` content trigger and its player-perspective twin, so
/// chain authors can hook narrative reactions (the AI itself already
/// repositions; this is just the affordance).
async fn fire_flank_triggers(
    npc_id: u32,
    target_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    let npc_template = space_mgr
        .get_entity(npc_id)
        .and_then(|e| e.npc_name.clone())
        .unwrap_or_default();
    crate::cell::content::fire_npc_flanked(npc_id, target_id, &npc_template, engine, tx, space_mgr)
        .await;
    // Mission-scoped chains (flank objectives, C06) need the flanking
    // player as the action target. No-ops when the threat isn't a player.
    crate::cell::content::fire_player_flanked_npc(
        npc_id,
        target_id,
        &npc_template,
        engine,
        tx,
        space_mgr,
    )
    .await;
}

/// The `decision_outcome=no_cover` row that replaces the silent
/// `NoCover => {}` arm (audit C7).
///
/// DEBUG and a log field only: it does not claim the tick's terminal
/// outcome (the chase / attack branch that follows does), so it goes
/// through neither `note_outcome` nor the decisions counter. Sampled per
/// NPC ([`NO_COVER_SAMPLE_INTERVAL`]): a fight without cover repeats the
/// same answer every tick.
fn report_no_cover(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    target_id: u32,
    reason: NoCoverReason,
    pick: Option<&PickTrace>,
    now: Instant,
) {
    let Some(suppressed) =
        space_mgr
            .npc_detectors
            .admit_sample(npc_id, "no_cover", now, NO_COVER_SAMPLE_INTERVAL)
    else {
        return;
    };
    tracing::debug!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = "no_cover",
        npc_id,
        target_id,
        tag = space_mgr
            .get_entity(npc_id)
            .and_then(|e| e.tag.as_deref())
            .unwrap_or(""),
        world = %super::world_label(space_mgr, npc_id),
        reason = reason.label(),
        candidates_scanned = pick.map(|p| p.scanned),
        reserved_skipped = pick.map(|p| p.reserved_skipped),
        out_of_reach = pick.map(|p| p.out_of_reach),
        search_radius = MAX_COVER_DISTANCE,
        cover_nodes_loaded = space_mgr.cover.node_count(),
        suppressed,
        "NPC AI: no cover this tick ({})",
        reason.label()
    );
}

/// `cover.selection`: what the scorer picked and the best of what it
/// passed over. Sampled per NPC.
fn log_selection(space_mgr: &mut SpaceManager, npc_id: u32, pick: &PickTrace, now: Instant) {
    let Some(suppressed) = space_mgr.npc_detectors.admit_sample(
        npc_id,
        "cover_selection",
        now,
        SELECTION_SAMPLE_INTERVAL,
    ) else {
        return;
    };
    if let Some(best) = pick.best {
        tracing::debug!(
            target: "cover.selection",
            event = "picked",
            npc_id,
            chunk_id = best.chunk_id,
            node_id = best.node_id,
            score = best.score,
            move_dist = best.move_dist,
            threat_dist = best.threat_dist,
            scanned = pick.scanned,
            out_of_reach = pick.out_of_reach,
            suppressed,
            "cover.selection: best-scoring free node"
        );
    }
    for (rank, c) in pick.runners_up.iter().enumerate() {
        tracing::debug!(
            target: "cover.selection",
            event = "rejected",
            npc_id,
            rank = rank + 1,
            chunk_id = c.chunk_id,
            node_id = c.node_id,
            score = c.score,
            move_dist = c.move_dist,
            threat_dist = c.threat_dist,
            reason = if c.reserved { "reserved" } else { "lower_score" },
            "cover.selection: candidate passed over"
        );
    }
}
