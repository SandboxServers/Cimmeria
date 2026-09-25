//! Cover-node scoring for the NPC AI.
//!
//! Implements the six-weight scoring formula that the SGW dev console
//! `Event_NetOut_ChangeCoverWeight` (at SGW.exe `0x00c87430`) was designed
//! to tune. The six weights:
//!
//! | Weight | Term meaning |
//! |---|---|
//! | `aDistanceWeight` | farther from threat is better — encourages picking cover at tactical engagement range |
//! | `aDefCoverWeight` | how well the cover faces the current threat |
//! | `aOffCoverWeight` | how well the cover faces the *direction the NPC will shoot* |
//! | `aMoveWeight` | penalty for picking a node far from the NPC's current position |
//! | `aCrossPathWeight` | penalty for paths that cross the threat's line of fire |
//! | `aCoverWeight` | quality of the cover itself (Best/Better/Good/None) |
//!
//! Plus two Cimmeria additions on top of the six base terms:
//!
//! - **Flank check** (`is_flanked`, `defends_for_pick`): half-plane test —
//!   is the threat outside the cover's defensive arc (`orient ± π/2`)? A held
//!   slot is released 20 degrees past side-on, a free one is only picked in
//!   front of side-on (NA23's hysteresis band).
//! - **Squad-affinity penalty**: discourages multiple NPCs from piling onto
//!   the same obstacle when alternatives exist. It counts allies holding a
//!   slot within [`SQUAD_AFFINITY_RADIUS`] of the candidate, not allies in
//!   the same set: NA21's transitive grouping makes some sets very large
//!   (105 nodes across a 17 x 14 m Castle courtyard), and a per-set count
//!   would penalise every node in the courtyard for one ally (NA22).
//!
//! All sub-scores are normalised to `[0.0, 1.0]` so the final score has a
//! predictable range. Higher = better. The caller picks the highest-scoring
//! unoccupied node.

use cimmeria_common::Vector3;

use super::reservation::CoverReservations;
use super::spatial::CoverIndex;
use super::types::{CoverNode, CoverSlotKey};

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
    /// holding a slot within [`SQUAD_AFFINITY_RADIUS`] of the candidate.
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

/// Horizontal radius (m) inside which another NPC's held slot counts
/// against a candidate for squad affinity. Two metres is "the same side of
/// the same desk": the extracted slots along one obstacle sit about a
/// metre apart.
pub const SQUAD_AFFINITY_RADIUS: f32 = 2.0;

/// Inputs to a scoring pass — passed by reference to avoid copies.
#[derive(Debug, Clone, Copy)]
pub struct ScoringContext {
    /// NPC's current world position.
    pub npc_pos: Vector3,
    /// Top-threat target's current world position.
    pub threat_pos: Vector3,
    /// A candidate further than this from the threat is not considered: the
    /// NPC could not shoot from it (NA22: cover is a firing position).
    /// `INFINITY` by default.
    pub max_threat_dist: f32,
    /// A candidate further than this from the NPC is not considered.
    /// Defaults to [`MAX_COVER_DISTANCE`]; an NPC that already has a shot
    /// only takes a short walk to cover.
    pub max_move_dist: f32,
    /// A slot this NPC may not pick (NA23: the one it just gave up as
    /// flanked or blind, while its cooldown runs).
    pub excluded: Option<CoverSlotKey>,
}

impl ScoringContext {
    pub fn new(npc_pos: Vector3, threat_pos: Vector3) -> Self {
        Self {
            npc_pos,
            threat_pos,
            max_threat_dist: f32::INFINITY,
            max_move_dist: MAX_COVER_DISTANCE,
            excluded: None,
        }
    }

    /// Never pick `slot`.
    pub fn excluding(mut self, slot: Option<CoverSlotKey>) -> Self {
        self.excluded = slot;
        self
    }

    /// Restrict candidates to those within `max_threat_dist` of the threat
    /// and `max_move_dist` of the NPC.
    pub fn with_limits(mut self, max_threat_dist: f32, max_move_dist: f32) -> Self {
        self.max_threat_dist = max_threat_dist;
        self.max_move_dist = max_move_dist.min(MAX_COVER_DISTANCE);
        self
    }
}

/// How many of `ally_slots` sit within [`SQUAD_AFFINITY_RADIUS`] of `pos`,
/// horizontally.
pub fn allies_near(pos: Vector3, ally_slots: &[Vector3]) -> usize {
    let r2 = SQUAD_AFFINITY_RADIUS * SQUAD_AFFINITY_RADIUS;
    ally_slots
        .iter()
        .filter(|a| {
            let (dx, dz) = (a.x - pos.x, a.z - pos.z);
            dx * dx + dz * dz <= r2
        })
        .count()
}

/// The flank test's release threshold: an NPC holding a slot gives it up
/// as flanked only once the threat is more than 20 degrees behind the
/// cover's side-on line (normalised dot below `-sin 20°`), i.e. more than
/// 110 degrees off the node's facing.
///
/// NA22 used 5 degrees (`-0.0872`). In the tight Castle_CellBlock mess hall
/// that flipped on a strafe: UAT-1's three `cover_released_flanked` rows
/// (NPCs 100160 and 100161, 12:14:06-12:14:12) all had the threat 100-102
/// degrees off the facing (dot -0.17 to -0.20), a couple of metres of
/// sidestep from "in front" (dot +0.17). 20 degrees holds every one of them.
pub const FLANK_RELEASE_DOT: f32 = -0.342; // -sin(20°)

/// The pick threshold: a slot is only picked when the threat is in front of
/// the cover's side-on line (dot at least 0). With [`FLANK_RELEASE_DOT`] this
/// is a 20 degree hysteresis band: a slot is taken only when it clearly
/// defends and given up only when it clearly no longer does, so a strafing
/// threat cannot flip an NPC between the two.
pub const FLANK_PICK_DOT: f32 = 0.0;

/// Normalised horizontal dot of the node's facing with the direction to the
/// threat: +1 straight ahead, 0 side-on, -1 straight behind. The hysteresis
/// is angular, not distance-dependent.
fn facing_dot(cover_pos: Vector3, cover_orient: f32, threat_pos: Vector3) -> f32 {
    let (dx, dz) = (threat_pos.x - cover_pos.x, threat_pos.z - cover_pos.z);
    let len = (dx * dx + dz * dz).sqrt().max(1e-3);
    (dx * cover_orient.cos() + dz * cover_orient.sin()) / len
}

/// Half-plane flank test for a slot an NPC already holds. Returns `true`
/// when `threat_pos` is clearly outside the cover's defensive arc: more
/// than 20 degrees past side-on ([`FLANK_RELEASE_DOT`]).
///
/// The cover's `orient` faces outward from the wall. The defended
/// half-plane is the half-space "in front of" the cover.
pub fn is_flanked(cover_pos: Vector3, cover_orient: f32, threat_pos: Vector3) -> bool {
    facing_dot(cover_pos, cover_orient, threat_pos) < FLANK_RELEASE_DOT
}

/// Whether a free slot defends against `threat_pos` well enough to pick:
/// the threat is in front of the side-on line ([`FLANK_PICK_DOT`]).
pub fn defends_for_pick(cover_pos: Vector3, cover_orient: f32, threat_pos: Vector3) -> bool {
    facing_dot(cover_pos, cover_orient, threat_pos) >= FLANK_PICK_DOT
}

/// Score a single candidate cover node. Returns a final score in roughly
/// `[-N, 1+N]` (where N is the number of squad allies near the node,
/// times the squad-affinity penalty). Higher is better.
///
/// `allied_near` is the count of other NPCs holding a slot within
/// [`SQUAD_AFFINITY_RADIUS`] of this node ([`allies_near`]).
pub fn score_node(
    node: &CoverNode,
    ctx: &ScoringContext,
    weights: &CoverWeights,
    allied_near: usize,
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
    let cos_def = (to_threat_from_node.x * face_x + to_threat_from_node.z * face_z) / to_threat_len;
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
    let npc_to_cover = Vector3::new(node.pos.x - ctx.npc_pos.x, 0.0, node.pos.z - ctx.npc_pos.z);
    let npc_to_cover_len = (npc_to_cover.x * npc_to_cover.x + npc_to_cover.z * npc_to_cover.z)
        .sqrt()
        .max(1e-3);
    let npc_to_threat = Vector3::new(
        ctx.threat_pos.x - ctx.npc_pos.x,
        0.0,
        ctx.threat_pos.z - ctx.npc_pos.z,
    );
    let npc_to_threat_len = (npc_to_threat.x * npc_to_threat.x + npc_to_threat.z * npc_to_threat.z)
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
    if allied_near > 0 {
        score -= weights.squad_affinity_penalty_per_ally * allied_near as f32;
    }

    score
}

/// Pick the best unoccupied cover node within `MAX_COVER_DISTANCE` of the
/// NPC's position. Returns the chosen node's index in the index's `all_nodes`
/// slice, or `None` if no viable candidate exists.
///
/// `ally_slots` are the positions of the slots other NPCs hold; the scorer
/// applies a squad-affinity penalty per ally near each candidate
/// ([`allies_near`]).
///
/// Only nodes in `world_id` are candidates, and only those inside the
/// context's `max_threat_dist` / `max_move_dist` limits.
pub fn pick_best(
    index: &CoverIndex,
    world_id: i32,
    reservations: &CoverReservations,
    ctx: &ScoringContext,
    weights: &CoverWeights,
    ally_slots: &[Vector3],
) -> Option<usize> {
    pick_best_traced(index, world_id, reservations, ctx, weights, ally_slots)
        .best
        .map(|c| c.idx)
}

/// Vertical tolerance for cover candidates: cover on a different floor of
/// a multi-level chunk is unreachable without a pathfinding stair-climb.
const MAX_COVER_Y_DIFF: f32 = 2.0;
/// How many losing candidates a [`PickTrace`] keeps.
const PICK_TRACE_RUNNERS_UP: usize = 3;

/// One scored cover candidate, for the `cover.selection` log.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoredCandidate {
    pub idx: usize,
    pub chunk_id: i32,
    pub node_id: i32,
    pub score: f32,
    /// NPC to node: the walk (the `move` term's input).
    pub move_dist: f32,
    /// Threat to node (the `distance` term's input).
    pub threat_dist: f32,
    /// Held by another NPC, so it never competed.
    pub reserved: bool,
}

/// What [`pick_best_traced`] looked at, as well as what it chose.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickTrace {
    /// Nodes inside `MAX_COVER_DISTANCE` and the vertical band.
    pub scanned: usize,
    /// Of those, how many were outside the context's reach limits (too far
    /// from the threat to shoot from, or too long a walk).
    pub out_of_reach: usize,
    /// Of those, how many were already reserved.
    pub reserved_skipped: usize,
    /// Free candidates that scored higher than the winner but were passed
    /// over because the NPC would have no shot from them (NA23).
    pub no_shot: usize,
    pub best: Option<ScoredCandidate>,
    /// The best-scoring losers (reserved or lower score), best first.
    pub runners_up: Vec<ScoredCandidate>,
}

/// [`pick_best`] with its working shown. The choice is identical: the first
/// unreserved candidate with the strictly highest score, in `nearby` order.
pub fn pick_best_traced(
    index: &CoverIndex,
    world_id: i32,
    reservations: &CoverReservations,
    ctx: &ScoringContext,
    weights: &CoverWeights,
    ally_slots: &[Vector3],
) -> PickTrace {
    pick_best_filtered(
        index,
        world_id,
        reservations,
        ctx,
        weights,
        ally_slots,
        &|_| true,
    )
}

/// [`pick_best_traced`], but the winner must also pass `accept`: the best
/// unreserved candidate that does, in score order. `accept` runs lazily, best
/// first, and stops at the first pass; each rejection counts in
/// [`PickTrace::no_shot`]. The fight tick passes the slot shot check (NA23:
/// a slot the NPC could not see its target from is not a firing position).
pub fn pick_best_filtered(
    index: &CoverIndex,
    world_id: i32,
    reservations: &CoverReservations,
    ctx: &ScoringContext,
    weights: &CoverWeights,
    ally_slots: &[Vector3],
    accept: &dyn Fn(&CoverNode) -> bool,
) -> PickTrace {
    let candidate_indices = index.nearby(
        world_id,
        &ctx.npc_pos,
        ctx.max_move_dist.min(MAX_COVER_DISTANCE),
        Some(MAX_COVER_Y_DIFF),
    );
    let mut trace = PickTrace {
        scanned: candidate_indices.len(),
        ..PickTrace::default()
    };
    let mut scored: Vec<ScoredCandidate> = Vec::new();
    for idx in candidate_indices {
        let n = match index.node(idx) {
            Some(n) => n,
            None => continue,
        };
        let move_dist = n.pos.distance_to(&ctx.npc_pos);
        let threat_dist = n.pos.distance_to(&ctx.threat_pos);
        // A slot must clearly defend to be picked (NA23: the pick side of
        // the flank hysteresis band), and the slot this NPC just gave up as
        // flanked or blind waits out its cooldown.
        if threat_dist > ctx.max_threat_dist
            || move_dist > ctx.max_move_dist
            || !defends_for_pick(n.pos, n.orient, ctx.threat_pos)
            || ctx.excluded == Some(n.key())
        {
            trace.out_of_reach += 1;
            continue;
        }
        let allied = allies_near(n.pos, ally_slots);
        let reserved = reservations.is_reserved(n.key());
        let c = ScoredCandidate {
            idx,
            chunk_id: n.chunk_id,
            node_id: n.node_id,
            score: score_node(n, ctx, weights, allied),
            move_dist,
            threat_dist,
            reserved,
        };
        if reserved {
            trace.reserved_skipped += 1;
        }
        scored.push(c);
    }
    // Best first. The sort is stable, so equal scores keep `nearby` order and
    // the first of them wins, as the original strictly-greater loop chose. A
    // NaN score never wins.
    let mut pool: Vec<ScoredCandidate> = scored
        .iter()
        .filter(|c| !c.reserved && !c.score.is_nan())
        .copied()
        .collect();
    pool.sort_by(|a, b| b.score.total_cmp(&a.score));
    for c in pool {
        let Some(n) = index.node(c.idx) else {
            continue;
        };
        if accept(n) {
            trace.best = Some(c);
            break;
        }
        trace.no_shot += 1;
    }
    let mut losers: Vec<ScoredCandidate> = scored
        .into_iter()
        .filter(|c| trace.best.is_none_or(|b| b.idx != c.idx))
        .collect();
    losers.sort_by(|a, b| b.score.total_cmp(&a.score));
    losers.truncate(PICK_TRACE_RUNNERS_UP);
    trace.runners_up = losers;
    trace
}
