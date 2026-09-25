//! The Fighting handler's cover step: ask the cover system where to stand,
//! and report why when the answer is "nowhere".
//!
//! Split out of `fight.rs` along the cover seam when NA02 added the
//! `no_cover` reasons and the `cover.selection` sample (the file was at the
//! size cap). The decision itself is unchanged: `maintain_cover_for_npc`
//! runs only for a mobile NPC that uses cover, and its answer overrides the
//! chase destination.

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, Vector3};
use tokio::sync::mpsc;

use crate::cell::cover::{
    maintain_cover_for_npc_traced, CoverDecision, CoverWeights, NoCoverReason, PickTrace,
    MAX_COVER_DISTANCE,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `cover.selection` rows at most once per NPC per this window.
const SELECTION_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);

/// The inputs the cover step needs from the fight handler.
pub(super) struct CoverStep {
    pub npc_id: u32,
    pub target_id: u32,
    pub npc_pos: Vector3,
    /// The NPC's `resources.worlds.world_id`: cover is indexed per world.
    pub world_id: Option<i32>,
    pub target_pos: Vector3,
    pub in_range: bool,
    pub use_cover: bool,
    pub is_stationary: bool,
}

/// Run the cover step. Returns the position the chase should path toward:
/// a cover slot, or the target itself.
pub(super) async fn route_via_cover(
    step: CoverStep,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) -> Vector3 {
    let CoverStep {
        npc_id,
        target_id,
        npc_pos,
        world_id,
        target_pos,
        in_range,
        use_cover,
        is_stationary,
    } = step;
    // The two short-circuits the caller used to take silently.
    if !use_cover || is_stationary {
        let reason = if use_cover {
            NoCoverReason::Stationary
        } else {
            NoCoverReason::UseCoverFalse
        };
        report_no_cover(space_mgr, npc_id, target_id, reason, None);
        return target_pos;
    }

    let (decision, trace) = maintain_cover_for_npc_traced(
        EntityId(npc_id as i32),
        npc_pos,
        world_id,
        target_pos,
        in_range,
        true,
        &space_mgr.cover,
        &CoverWeights::default(),
    );
    if let Some(pick) = &trace.pick {
        log_selection(space_mgr, npc_id, pick, Instant::now());
    }
    match decision {
        CoverDecision::StayInCover { pos, slot } => {
            super::note_outcome("stay_in_cover");
            tracing::debug!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "stay_in_cover",
                npc_id,
                target_id,
                chunk_id = slot.chunk_id,
                node_id = slot.node_id,
                "NPC AI: holding cover slot"
            );
            pos
        }
        CoverDecision::MoveToCover { pos, slot } => {
            super::note_outcome("move_to_cover");
            tracing::info!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "move_to_cover",
                npc_id,
                target_id,
                chunk_id = slot.chunk_id,
                node_id = slot.node_id,
                "NPC AI: picked cover slot"
            );
            pos
        }
        CoverDecision::Released { prior_slot } => {
            super::note_outcome("cover_released_flanked");
            tracing::info!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "cover_released_flanked",
                npc_id,
                target_id,
                chunk_id = prior_slot.chunk_id,
                node_id = prior_slot.node_id,
                "NPC AI: released flanked cover slot, re-evaluating next tick"
            );
            // Stop walking toward the abandoned slot (and zero the
            // velocity) until the re-pick lands next AI tick (NA10).
            super::stop_npc_movement(space_mgr, npc_id, super::StopReason::CoverReleased);
            // Fire the OnNpcFlanked content trigger so chain
            // authors can hook narrative reactions (the AI itself
            // already repositions; this is just the affordance).
            let npc_template = space_mgr
                .get_entity(npc_id)
                .and_then(|e| e.npc_name.clone())
                .unwrap_or_default();
            crate::cell::content::fire_npc_flanked(
                npc_id,
                target_id,
                &npc_template,
                engine,
                tx,
                space_mgr,
            )
            .await;
            // Player-perspective twin: mission-scoped chains (flank
            // objectives, C06) need the flanking player as the action
            // target. No-ops when the threat isn't a player.
            crate::cell::content::fire_player_flanked_npc(
                npc_id,
                target_id,
                &npc_template,
                engine,
                tx,
                space_mgr,
            )
            .await;
            target_pos
        }
        CoverDecision::NoCover => {
            let reason = trace.no_cover.unwrap_or(NoCoverReason::NoCandidateInRadius);
            report_no_cover(space_mgr, npc_id, target_id, reason, trace.pick.as_ref());
            target_pos
        }
    }
}

/// The `decision_outcome=no_cover` row that replaces the silent
/// `NoCover => {}` arm (audit C7).
///
/// DEBUG and a log field only: it does not claim the tick's terminal
/// outcome (the chase / attack branch that follows does), so it goes
/// through neither `note_outcome` nor the decisions counter.
fn report_no_cover(
    space_mgr: &SpaceManager,
    npc_id: u32,
    target_id: u32,
    reason: NoCoverReason,
    pick: Option<&PickTrace>,
) {
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
        search_radius = MAX_COVER_DISTANCE,
        cover_nodes_loaded = space_mgr.cover.node_count(),
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
