//! Unit tests for `cover::scoring`: the flank bands, the six-weight score,
//! squad affinity and the pick. Split out of `scoring.rs` at the size cap
//! (NA23).

use super::reservation::CoverReservations;
use super::scoring::*;
use super::spatial::CoverIndex;
use super::types::{CoverNode, CoverSlotKey};
use crate::cell::cover::types::{CoverHeight, CoverQuality};
use cimmeria_common::Vector3;

fn n(chunk_id: i32, node_id: i32, x: f32, z: f32, orient: f32, q: CoverQuality) -> CoverNode {
    CoverNode {
        chunk_id,
        node_id,
        world_id: crate::cell::cover::TEST_WORLD_ID,
        pos: Vector3::new(x, 0.0, z),
        orient,
        height: CoverHeight::Mid,
        quality: q,
        width: 1.0,
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
fn is_flanked_perpendicular_stays_in_cover_hysteresis() {
    // Threat exactly perpendicular to cover orient — dot product = 0.
    // Strict `dot <= 0.0` would say "flanked" and trigger re-pick.
    // With the 20° hysteresis (FLANK_RELEASE_DOT = -sin 20°), a
    // perpendicular threat stays inside the defensive arc — no
    // tick-to-tick oscillation. Threat must move clearly behind
    // the cover to flip the test.
    let cover_pos = Vector3::zero();
    assert!(
        !is_flanked(cover_pos, 0.0, Vector3::new(0.0, 0.0, 5.0)),
        "perpendicular threat must stay in defensive arc (hysteresis)"
    );
}

#[test]
fn is_flanked_does_not_oscillate_within_hysteresis() {
    // Threat behind the perpendicular boundary by as much as UAT-1's
    // mess-hall releases (10-12°). With NA22's 5° band these flipped;
    // with 20° the NPC keeps the slot until the threat is clearly
    // flanking.
    let cover_pos = Vector3::zero();
    let at = |deg: f32| {
        let t = deg.to_radians();
        Vector3::new(5.0 * t.cos(), 0.0, 5.0 * t.sin())
    };
    assert!(
        !is_flanked(cover_pos, 0.0, at(-89.0)),
        "1° past perpendicular must NOT flip"
    );
    assert!(
        !is_flanked(cover_pos, 0.0, at(-102.0)),
        "12° past perpendicular (the UAT-1 mess-hall flank) must NOT flip"
    );
    // 25° past perpendicular (clearly flanked) flips.
    assert!(
        is_flanked(cover_pos, 0.0, at(-115.0)),
        "25° past perpendicular must flip — clear flank"
    );
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
    assert!(
        s_best > s_good,
        "Best quality must outscore Good at equal geometry"
    );
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
    let facing_away = n(1, 1, 5.0, 0.0, std::f32::consts::PI, CoverQuality::Best); // orient=π → faces -X = away from threat
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
    assert!(
        s_solo > s_one_ally,
        "an ally at the same obstacle must penalise"
    );
    assert!(
        s_one_ally > s_two_allies,
        "second ally must penalise further"
    );
}

#[test]
fn pick_best_skips_reserved_slots() {
    let weights = CoverWeights::default();
    let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(20.0, 0.0, 0.0));
    let nodes = vec![
        n(1, 0, 5.0, 0.0, 0.0, CoverQuality::Best),
        n(1, 1, 6.0, 0.0, 0.0, CoverQuality::Best),
    ];
    let idx = CoverIndex::build(nodes);
    let mut r = CoverReservations::new();
    // Pin one of the two reserved by some other entity.
    r.reserve_for_entity(cimmeria_common::EntityId(99), CoverSlotKey::new(1, 0))
        .unwrap();
    let ally_counts: Vec<Vector3> = Vec::new();
    let pick = pick_best(
        &idx,
        crate::cell::cover::TEST_WORLD_ID,
        &r,
        &ctx,
        &weights,
        &ally_counts,
    )
    .expect("must pick something");
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
    let idx = CoverIndex::build(vec![n(1, 0, 100.0, 100.0, 0.0, CoverQuality::Best)]);
    let r = CoverReservations::new();
    let ally_counts: Vec<Vector3> = Vec::new();
    assert!(pick_best(
        &idx,
        crate::cell::cover::TEST_WORLD_ID,
        &r,
        &ctx,
        &weights,
        &ally_counts
    )
    .is_none());
}

/// An NPC never picks a slot from another world, however close its
/// coordinates: the scorer's candidate set comes from the per-world
/// index.
#[test]
fn pick_best_never_picks_another_worlds_slot() {
    let weights = CoverWeights::default();
    let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(20.0, 0.0, 0.0));
    let idx = CoverIndex::build(vec![n(1, 0, 5.0, 0.0, 0.0, CoverQuality::Best)]);
    let r = CoverReservations::new();
    let ally_counts: Vec<Vector3> = Vec::new();
    let other_world = crate::cell::cover::TEST_WORLD_ID + 1;
    assert!(pick_best(&idx, other_world, &r, &ctx, &weights, &ally_counts).is_none());
    assert!(pick_best(
        &idx,
        crate::cell::cover::TEST_WORLD_ID,
        &r,
        &ctx,
        &weights,
        &ally_counts
    )
    .is_some());
}

/// A threat side-on or just behind a free slot is inside the hold band
/// but outside the pick band: the slot is kept if held, never taken.
#[test]
fn pick_band_is_narrower_than_the_hold_band() {
    let at = |deg: f32| {
        let t = deg.to_radians();
        Vector3::new(5.0 * t.cos(), 0.0, 5.0 * t.sin())
    };
    let o = Vector3::zero();
    assert!(defends_for_pick(o, 0.0, at(80.0)));
    assert!(!defends_for_pick(o, 0.0, at(100.0)));
    assert!(!is_flanked(o, 0.0, at(100.0)), "held through 100°");
}

/// The slot in the context's `excluded` (the one this NPC just gave up)
/// is never picked, even as the only candidate.
#[test]
fn pick_best_skips_the_excluded_slot() {
    let idx = CoverIndex::build(vec![n(1, 0, 5.0, 0.0, 0.0, CoverQuality::Best)]);
    let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(20.0, 0.0, 0.0));
    let pick = |ctx: &ScoringContext| {
        pick_best_traced(
            &idx,
            crate::cell::cover::TEST_WORLD_ID,
            &CoverReservations::new(),
            ctx,
            &CoverWeights::default(),
            &[],
        )
    };
    assert!(pick(&ctx).best.is_some(), "fixture: pickable");
    let trace = pick(&ctx.excluding(Some(CoverSlotKey::new(1, 0))));
    assert!(trace.best.is_none());
    assert_eq!(trace.out_of_reach, 1);
}

/// A slot the threat already flanks is never picked: the next tick's
/// flank test would release it, and the NPC would loop pick/release.
#[test]
fn pick_best_skips_a_slot_the_threat_already_flanks() {
    let ctx = ScoringContext::new(Vector3::zero(), Vector3::new(20.0, 0.0, 0.0));
    // Faces -X: the threat at +X is behind it.
    let idx = CoverIndex::build(vec![n(
        1,
        0,
        5.0,
        0.0,
        std::f32::consts::PI,
        CoverQuality::Best,
    )]);
    let trace = pick_best_traced(
        &idx,
        crate::cell::cover::TEST_WORLD_ID,
        &CoverReservations::new(),
        &ctx,
        &CoverWeights::default(),
        &[],
    );
    assert!(trace.best.is_none());
    assert_eq!(trace.out_of_reach, 1);
}

#[test]
fn with_limits_excludes_slots_out_of_reach() {
    let idx = CoverIndex::build(vec![
        n(1, 0, 5.0, 0.0, 0.0, CoverQuality::Best), // 15 u from the threat
        n(1, 1, -5.0, 0.0, 0.0, CoverQuality::Best), // 25 u from the threat
    ]);
    let ctx =
        ScoringContext::new(Vector3::zero(), Vector3::new(20.0, 0.0, 0.0)).with_limits(20.0, 30.0);
    let pick = pick_best(
        &idx,
        crate::cell::cover::TEST_WORLD_ID,
        &CoverReservations::new(),
        &ctx,
        &CoverWeights::default(),
        &[],
    )
    .expect("the near slot reaches");
    assert_eq!(idx.node(pick).unwrap().node_id, 0);
}

#[test]
fn allies_near_counts_only_within_the_radius() {
    let pos = Vector3::zero();
    let allies = [
        Vector3::new(1.0, 0.0, 1.0),
        Vector3::new(0.0, 5.0, SQUAD_AFFINITY_RADIUS), // on the edge; height ignored
        Vector3::new(SQUAD_AFFINITY_RADIUS + 0.5, 0.0, 0.0),
    ];
    assert_eq!(allies_near(pos, &allies), 2);
}
