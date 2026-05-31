//! Cover-node scoring for the NPC AI.
//!
//! Implements the six-weight scoring formula that the SGW dev console
//! `Event_NetOut_ChangeCoverWeight` (at SGW.exe `0x00c87430`) was designed
//! to tune. The six weights:
//!
//! | Weight | Term meaning |
//! |---|---|
//! | `aDistanceWeight` | shorter is better — encourages picking nearby cover |
//! | `aDefCoverWeight` | how well the cover faces the current threat |
//! | `aOffCoverWeight` | how well the cover faces the *direction the NPC will shoot* |
//! | `aMoveWeight` | penalty for picking a node far from the NPC's current position |
//! | `aCrossPathWeight` | penalty for paths that cross the threat's line of fire |
//! | `aCoverWeight` | quality of the cover itself (Best/Better/Good/None) |
//!
//! Plus two Cimmeria additions on top of the six base terms:
//!
//! - **Flank check** (`is_flanked`): half-plane test — is the threat outside
//!   the cover's defensive arc (`orient ± π/2`)? Cheap, gates re-picking.
//! - **Squad-affinity penalty**: discourages multiple NPCs from piling into
//!   the same chunk when alternatives exist.
//!
//! All sub-scores are normalised to `[0.0, 1.0]` so the final score has a
//! predictable range. Higher = better. The caller picks the highest-scoring
//! unoccupied node.

use cimmeria_common::Vector3;

use super::reservation::CoverReservations;
use super::spatial::CoverIndex;
use super::types::CoverNode;

/// Tunable weights. Defaults match the V5 RE recommendation. Wired
/// initially as compile-time constants; can be swapped to a runtime
/// `RwLock<CoverWeights>` if/when the `Event_NetOut_ChangeCoverWeight`
/// slash command is implemented.
#[derive(Debug, Clone, Copy)]
pub struct CoverWeights {
    pub distance: f32,
    pub def_cover: f32,
    pub off_cover: f32,
    pub move_: f32,
    pub cross_path: f32,
    pub cover: f32,
    /// Squad-affinity penalty multiplier — applied per allied NPC already
    /// holding cover in the same chunk.
    pub squad_affinity_penalty_per_ally: f32,
}

impl Default for CoverWeights {
    /// Defaults from the V5 RE notes. Tunable per encounter.
    fn default() -> Self {
        Self {
            distance: 1.0,
            def_cover: 1.0,
            off_cover: 0.5,
            move_: 0.3,
            cross_path: 0.2,
            cover: 1.0,
            squad_affinity_penalty_per_ally: 0.2,
        }
    }
}

/// Maximum distance (BW meters) the scorer considers viable. Nodes beyond
/// this are excluded before scoring. Matches the typical `nearby` query
/// radius used by the NPC AI cover-search.
pub const MAX_COVER_DISTANCE: f32 = 30.0;

/// Inputs to a scoring pass — passed by reference to avoid copies.
#[derive(Debug, Clone, Copy)]
pub struct ScoringContext {
    /// NPC's current world position.
    pub npc_pos: Vector3,
    /// Top-threat target's current world position.
    pub threat_pos: Vector3,
}

impl ScoringContext {
    pub fn new(npc_pos: Vector3, threat_pos: Vector3) -> Self {
        Self { npc_pos, threat_pos }
    }
}

/// Half-plane flank test. Returns `true` if `threat_pos` is OUTSIDE the
/// cover's defensive arc (more than ±π/2 off `orient`), i.e. the cover is
/// no longer protecting against this threat.
///
/// The cover's `orient` faces outward from the wall. The defended
/// half-plane is therefore the half-space "in front of" the cover. If
/// `(threat - cover_pos)` falls into the *back* half-plane, the threat
/// is behind the cover — the NPC is flanked.
pub fn is_flanked(cover_pos: Vector3, cover_orient: f32, threat_pos: Vector3) -> bool {
    let to_threat = Vector3::new(threat_pos.x - cover_pos.x, 0.0, threat_pos.z - cover_pos.z);
    // Cover faces (cos(orient), 0, sin(orient)) in BW x/z plane.
    let face_x = cover_orient.cos();
    let face_z = cover_orient.sin();
    // Dot product: positive = in defended arc, negative = flanked.
    let dot = to_threat.x * face_x + to_threat.z * face_z;
    dot <= 0.0
}

/// Score a single candidate cover node. Returns a final score in roughly
/// `[-N, 1+N]` (where N is the number of squad allies in the same chunk,
/// times the squad-affinity penalty). Higher is better.
///
/// `allied_in_chunk` is the count of squad-allied NPCs currently holding
/// cover in this node's chunk; the caller computes it once per chunk.
pub fn score_node(
    node: &CoverNode,
    ctx: &ScoringContext,
    weights: &CoverWeights,
    allied_in_chunk: usize,
) -> f32 {
    // Distance from NPC to cover (the "I have to walk here" cost).
    let move_dist = node.pos.distance_to(&ctx.npc_pos);
    // Distance from threat to cover (the "how far is the threat from this cover" — used in distance term).
    let threat_dist = node.pos.distance_to(&ctx.threat_pos);

    // 1. Distance term — prefer cover that puts the NPC at a tactical
    //    engagement distance from the threat. Saturates at MAX_COVER_DISTANCE
    //    (beyond which the threat is out of weapon range anyway).
    //    Cover right next to the threat (dist→0) scores 0; cover at the
    //    edge of engagement (dist→MAX) scores 1.
    let dist_score = (threat_dist / MAX_COVER_DISTANCE).min(1.0);

    // 2. Defensive-cover term — how much the cover protects against the threat.
    //    `orient` is the direction the NPC faces while occupying the slot
    //    (out from the wall, toward where threats come from). If `orient`
    //    aligns with the (cover→threat) vector, the wall is between the
    //    NPC and the threat — good cover. Map cos∈[-1,1] → [0,1].
    let to_threat_from_node = Vector3::new(
        ctx.threat_pos.x - node.pos.x,
        0.0,
        ctx.threat_pos.z - node.pos.z,
    );
    let to_threat_len = (to_threat_from_node.x * to_threat_from_node.x
        + to_threat_from_node.z * to_threat_from_node.z)
        .sqrt()
        .max(1e-3);
    let face_x = node.orient.cos();
    let face_z = node.orient.sin();
    let cos_def =
        (to_threat_from_node.x * face_x + to_threat_from_node.z * face_z) / to_threat_len;
    let def_cover_score = ((cos_def + 1.0) / 2.0).clamp(0.0, 1.0);

    // 3. Offensive-cover term — same geometric shape as def_cover (the
    //    NPC needs to be able to shoot AT the threat from cover, which
    //    requires the cover orient to point toward the threat — same
    //    condition as defensive). Weighted separately in `CoverWeights`
    //    so designers can balance "I prefer hiding cover over firing
    //    cover" (def_cover = 1.0, off_cover = 0.5 by default).
    let off_cover_score = def_cover_score;

    // 4. Move term — discourage long walks. Same normalisation as
    //    distance, but applied to the NPC→cover distance.
    let move_score = (1.0 - (move_dist / MAX_COVER_DISTANCE)).max(0.0);

    // 5. Cross-path term — penalise covers where the NPC's path to cover
    //    crosses through the threat's line of fire. Approximation: how
    //    much of (npc→cover) goes "across" (npc→threat). 1.0 = perpendicular
    //    (high cross), 0.0 = along the threat direction (low cross). Then
    //    invert so low cross = high score.
    let npc_to_cover = Vector3::new(
        node.pos.x - ctx.npc_pos.x,
        0.0,
        node.pos.z - ctx.npc_pos.z,
    );
    let npc_to_cover_len = (npc_to_cover.x * npc_to_cover.x + npc_to_cover.z * npc_to_cover.z)
        .sqrt()
        .max(1e-3);
    let npc_to_threat = Vector3::new(
        ctx.threat_pos.x - ctx.npc_pos.x,
        0.0,
        ctx.threat_pos.z - ctx.npc_pos.z,
    );
    let npc_to_threat_len = (npc_to_threat.x * npc_to_threat.x
        + npc_to_threat.z * npc_to_threat.z)
        .sqrt()
        .max(1e-3);
    let cos_path = (npc_to_cover.x * npc_to_threat.x + npc_to_cover.z * npc_to_threat.z)
        / (npc_to_cover_len * npc_to_threat_len);
    // Aligned with threat direction = same side as threat = low cross.
    // Perpendicular = full cross = bad.
    let cross_path_score = ((cos_path + 1.0) / 2.0).clamp(0.0, 1.0);

    // 6. Cover-quality term.
    let quality_score = node.quality.score_factor();

    // Sum the weighted terms.
    let mut score = weights.distance * dist_score
        + weights.def_cover * def_cover_score
        + weights.off_cover * off_cover_score
        + weights.move_ * move_score
        + weights.cross_path * cross_path_score
        + weights.cover * quality_score;

    // Subtract squad-affinity penalty.
    if allied_in_chunk > 0 {
        score -= weights.squad_affinity_penalty_per_ally * allied_in_chunk as f32;
    }

    score
}

/// Pick the best unoccupied cover node within `MAX_COVER_DISTANCE` of the
/// NPC's position. Returns the chosen node's index in the index's `all_nodes`
/// slice, or `None` if no viable candidate exists.
///
/// `chunk_ally_counts` is a `chunk_id → count` map of how many allied NPCs
/// already hold cover in each chunk; the scorer applies a squad-affinity
/// penalty per ally. Caller computes this once per scoring pass.
pub fn pick_best<'a>(
    index: &'a CoverIndex,
    reservations: &CoverReservations,
    ctx: &ScoringContext,
    weights: &CoverWeights,
    chunk_ally_counts: &std::collections::HashMap<i32, usize>,
) -> Option<usize> {
    let candidate_indices = index.nearby(&ctx.npc_pos, MAX_COVER_DISTANCE, Some(5.0));
    let mut best_idx: Option<usize> = None;
    let mut best_score = f32::NEG_INFINITY;
    for idx in candidate_indices {
        let n = match index.node(idx) {
            Some(n) => n,
            None => continue,
        };
        if reservations.is_reserved(n.key()) {
            continue;
        }
        let allied = chunk_ally_counts.get(&n.chunk_id).copied().unwrap_or(0);
        let s = score_node(n, ctx, weights, allied);
        if s > best_score {
            best_score = s;
            best_idx = Some(idx);
        }
    }
    best_idx
}

#[cfg(test)]
mod scoring_tests {
    use super::*;
    use crate::cell::cover::types::{CoverHeight, CoverQuality};
    use std::collections::HashMap;

    fn n(chunk_id: i32, node_id: i32, x: f32, z: f32, orient: f32, q: CoverQuality) -> CoverNode {
        CoverNode {
            chunk_id,
            node_id,
            pos: Vector3::new(x, 0.0, z),
            orient,
            height: CoverHeight::Mid,
            quality: q,
            tail: [0; 4],
        }
    }

    #[test]
    fn is_flanked_detects_threat_behind_cover() {
        // Cover at origin facing +X (orient=0 → (cos 0, sin 0) = (1, 0)).
        // Threat at +X is in the defended arc; threat at -X is flanking.
        let cover_pos = Vector3::zero();
        assert!(
            !is_flanked(cover_pos, 0.0, Vector3::new(5.0, 0.0, 0.0)),
            "threat at +X (in front of cover) is NOT flanked"
        );
        assert!(
            is_flanked(cover_pos, 0.0, Vector3::new(-5.0, 0.0, 0.0)),
            "threat at -X (behind cover) IS flanked"
        );
    }

    #[test]
    fn is_flanked_threat_on_boundary_is_flanked() {
        // Threat exactly perpendicular to cover orient — dot product = 0.
        // We treat this as flanked (boundary case favours safety —
        // re-pick to a better position).
        let cover_pos = Vector3::zero();
        assert!(is_flanked(cover_pos, 0.0, Vector3::new(0.0, 0.0, 5.0)));
    }

    #[test]
    fn score_prefers_better_quality_at_equal_geometry() {
        let weights = CoverWeights::default();
        let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(10.0, 0.0, 0.0));
        // Two nodes at identical positions + orient, differing only in quality.
        let best = n(1, 0, 5.0, 0.0, std::f32::consts::PI, CoverQuality::Best);
        let good = n(1, 1, 5.0, 0.0, std::f32::consts::PI, CoverQuality::Good);
        let s_best = score_node(&best, &ctx, &weights, 0);
        let s_good = score_node(&good, &ctx, &weights, 0);
        assert!(s_best > s_good, "Best quality must outscore Good at equal geometry");
    }

    #[test]
    fn score_prefers_closer_cover_at_equal_quality() {
        let weights = CoverWeights::default();
        let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(20.0, 0.0, 0.0));
        // Same quality, same orient (both facing the threat at +X). The
        // 'close' node is close to the NPC AND at tactical engagement
        // range from the threat; the 'far' node is near the threat which
        // means low dist_score AND low move_score (long walk through
        // open ground). Closer-to-NPC wins on both axes.
        let close = n(1, 0, 5.0, 0.0, 0.0, CoverQuality::Best);
        let far = n(1, 1, 18.0, 0.0, 0.0, CoverQuality::Best);
        let s_close = score_node(&close, &ctx, &weights, 0);
        let s_far = score_node(&far, &ctx, &weights, 0);
        assert!(
            s_close > s_far,
            "Closer cover at tactical range must outscore far cover near the threat"
        );
    }

    #[test]
    fn score_prefers_cover_facing_toward_threat() {
        // `orient` is the direction the NPC faces while in cover — the
        // wall is BEHIND the NPC's facing direction. So a cover whose
        // orient points TOWARD the threat puts the wall between the NPC
        // and the threat (good); a cover whose orient points AWAY from
        // the threat has the NPC showing their back to the threat (bad).
        let weights = CoverWeights::default();
        let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(10.0, 0.0, 0.0));
        // Both at the same position; differ only in orient direction.
        let facing_threat = n(1, 0, 5.0, 0.0, 0.0, CoverQuality::Best); // orient=0 → faces +X = toward threat
        let facing_away =
            n(1, 1, 5.0, 0.0, std::f32::consts::PI, CoverQuality::Best); // orient=π → faces -X = away from threat
        let s_facing = score_node(&facing_threat, &ctx, &weights, 0);
        let s_away = score_node(&facing_away, &ctx, &weights, 0);
        assert!(
            s_facing > s_away,
            "Cover facing TOWARD the threat (wall behind NPC) must outscore cover facing AWAY (NPC's back exposed)"
        );
    }

    #[test]
    fn squad_affinity_penalises_clustered_chunks() {
        let weights = CoverWeights::default();
        let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(10.0, 0.0, 0.0));
        let node = n(1, 0, 5.0, 0.0, std::f32::consts::PI, CoverQuality::Best);
        let s_solo = score_node(&node, &ctx, &weights, 0);
        let s_one_ally = score_node(&node, &ctx, &weights, 1);
        let s_two_allies = score_node(&node, &ctx, &weights, 2);
        assert!(s_solo > s_one_ally, "ally in same chunk must penalise");
        assert!(s_one_ally > s_two_allies, "second ally must penalise further");
    }

    #[test]
    fn pick_best_skips_reserved_slots() {
        let weights = CoverWeights::default();
        let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(20.0, 0.0, 0.0));
        let nodes = vec![
            n(1, 0, 5.0, 0.0, std::f32::consts::PI, CoverQuality::Best),
            n(1, 1, 6.0, 0.0, std::f32::consts::PI, CoverQuality::Best),
        ];
        let idx = CoverIndex::build(nodes);
        let mut r = CoverReservations::new();
        // Pin one of the two reserved by some other entity.
        r.reserve_for_entity(cimmeria_common::EntityId(99), super::super::types::CoverSlotKey::new(1, 0))
            .unwrap();
        let ally_counts = HashMap::new();
        let pick = pick_best(&idx, &r, &ctx, &weights, &ally_counts).expect("must pick something");
        assert_eq!(
            idx.node(pick).unwrap().node_id,
            1,
            "must pick the unreserved sibling"
        );
    }

    #[test]
    fn pick_best_returns_none_when_no_candidates_within_radius() {
        let weights = CoverWeights::default();
        let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(10.0, 0.0, 0.0));
        // Only node is 100m away — outside MAX_COVER_DISTANCE.
        let idx = CoverIndex::build(vec![n(
            1,
            0,
            100.0,
            100.0,
            0.0,
            CoverQuality::Best,
        )]);
        let r = CoverReservations::new();
        let ally_counts = HashMap::new();
        assert!(pick_best(&idx, &r, &ctx, &weights, &ally_counts).is_none());
    }
}
