//! `npc_ai.aggro_scan`: why the Idle auto-aggro scan passed over a witness
//! (`event=candidate_rejected`) or found nobody (`event=no_candidates`).
//!
//! The reasons are the gates of `npc_ai::aggro_gates` (NA13): `not_player`,
//! `dead`, `same_faction`, `not_hostile`, `gm_ignored`,
//! `out_of_vertical_band`, `out_of_radius`, `no_los`, plus
//! `post_reset_suppressed` for the NA12 window. `aggro_radius` is the NPC's
//! radius in world units (NA02 logged `unbounded` before the gate existed).

use std::time::{Duration, Instant};

use super::NpcIdent;
use crate::cell::space_manager::SpaceManager;

/// One `(npc, player)` pair logs at most once per this window.
const REJECT_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
/// One NPC's `no_candidates` row at most once per this window.
const NO_CANDIDATES_SAMPLE_INTERVAL: Duration = Duration::from_secs(30);

/// Why a witness is not an aggro candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell) enum ScanReject {
    NotPlayer,
    Dead,
    SameFaction,
    /// The NPC's effective aggression toward the player is not HOSTILE.
    NotHostile,
    /// A GM with `.aggro off` set.
    GmIgnored,
    OutOfVerticalBand,
    OutOfRadius,
    NoLos,
    /// The NPC is inside its post-reset window (NA12); every witness is
    /// passed over until it closes.
    PostResetSuppressed,
}

impl ScanReject {
    /// Stable snake_case label. Treat as API.
    pub(in crate::cell) fn label(self) -> &'static str {
        match self {
            Self::NotPlayer => "not_player",
            Self::Dead => "dead",
            Self::SameFaction => "same_faction",
            Self::NotHostile => "not_hostile",
            Self::GmIgnored => "gm_ignored",
            Self::OutOfVerticalBand => "out_of_vertical_band",
            Self::OutOfRadius => "out_of_radius",
            Self::NoLos => "no_los",
            Self::PostResetSuppressed => "post_reset_suppressed",
        }
    }
}

/// Log the scan's rejects (sampled per pair) and, when nothing qualified,
/// the `no_candidates` row (sampled per NPC).
pub(in crate::cell) fn report_scan(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    witness_count: usize,
    rejects: &[(u32, ScanReject)],
    found_candidate: bool,
    now: Instant,
) {
    if rejects.is_empty() && found_candidate {
        return;
    }
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    let Some((npc_pos, radius)) = space_mgr
        .get_entity(npc_id)
        .map(|e| (e.position, crate::cell::combat::aggro_radius(e)))
    else {
        return;
    };
    for &(player_id, reason) in rejects {
        let Some(suppressed) = space_mgr.npc_detectors.pair_log.admit(
            npc_id,
            player_id,
            reason.label(),
            now,
            REJECT_SAMPLE_INTERVAL,
        ) else {
            continue;
        };
        let target_pos = space_mgr.get_entity(player_id).map(|p| p.position);
        tracing::debug!(
            target: "npc_ai.aggro_scan",
            event = "candidate_rejected",
            npc_id,
            tag = %ident.tag,
            template_id = ident.template_id,
            world = %ident.world,
            space_id = ident.space_id,
            reason = reason.label(),
            player_id,
            npc_to_target = target_pos.map(|p| p.distance_to(&npc_pos)),
            dy = target_pos.map(|p| p.y - npc_pos.y),
            aggro_radius = radius,
            suppressed,
            "npc_ai.aggro_scan: witness rejected as an aggro candidate"
        );
    }
    if found_candidate {
        return;
    }
    let Some(suppressed) = space_mgr.npc_detectors.admit_sample(
        npc_id,
        "aggro_scan_no_candidates",
        now,
        NO_CANDIDATES_SAMPLE_INTERVAL,
    ) else {
        return;
    };
    tracing::debug!(
        target: "npc_ai.aggro_scan",
        event = "no_candidates",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        witness_count,
        rejected = rejects.len(),
        suppressed,
        "npc_ai.aggro_scan: aggressive Idle NPC found no candidate"
    );
}
