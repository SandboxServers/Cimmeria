//! Unit tests for [`super::ai_integration`]: the cover decision tree on
//! synthetic nodes. Split out of `ai_integration.rs` when NA22 grew the
//! decision (hold / reach / seek hysteresis) past the file-size cap.

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, Vector3};

use super::ai_integration::*;
use super::types::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSlotKey};
use super::CoverWeights;

fn cover_with(nodes: Vec<CoverNode>) -> Cover {
    Cover::from_loaded(Vec::new(), nodes)
}

fn n(chunk_id: i32, node_id: i32, x: f32, z: f32, orient: f32) -> CoverNode {
    CoverNode {
        chunk_id,
        node_id,
        world_id: super::TEST_WORLD_ID,
        pos: Vector3::new(x, 0.0, z),
        orient,
        height: CoverHeight::Mid,
        quality: CoverQuality::Best,
        width: 1.0,
        tail: [0; 4],
    }
}

/// A query from `npc_pos` against a threat at `threat_pos`, with a 30 u
/// attack range (the server default) and `in_range` computed from the two.
fn q(npc: i32, npc_pos: Vector3, threat_pos: Vector3) -> CoverQuery {
    CoverQuery {
        npc_id: EntityId(npc),
        npc_pos,
        world_id: Some(super::TEST_WORLD_ID),
        threat_pos,
        in_range: npc_pos.distance_to(&threat_pos) <= 30.0,
        attack_range: 30.0,
        use_cover: true,
        now: Instant::now(),
    }
}

fn decide(query: CoverQuery, cover: &Cover) -> CoverDecision {
    maintain_cover_for_npc(query, cover, &CoverWeights::default())
}

fn reserve(cover: &Cover, npc: i32, slot: CoverSlotKey) {
    cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(npc), slot)
        .unwrap();
}

#[test]
fn use_cover_false_returns_no_cover() {
    let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
    let query = CoverQuery {
        use_cover: false,
        ..q(1, Vector3::zero(), Vector3::new(40.0, 0.0, 0.0))
    };
    assert_eq!(decide(query, &cover), CoverDecision::NoCover);
}

#[test]
fn out_of_range_with_candidate_returns_move_to_cover() {
    let cover = cover_with(vec![n(1, 0, 15.0, 0.0, 0.0)]);
    let dec = decide(q(1, Vector3::zero(), Vector3::new(40.0, 0.0, 0.0)), &cover);
    match dec {
        CoverDecision::MoveToCover { slot, .. } => {
            assert_eq!(slot, CoverSlotKey::new(1, 0));
            let r = cover.reservations.lock().unwrap();
            assert_eq!(r.holder(slot), Some(EntityId(1)));
        }
        other => panic!("expected MoveToCover, got {other:?}"),
    }
}

/// Audit C3: an NPC that already has a shot still takes a good slot within
/// a short walk. Before NA22 `in_range` short-circuited to `NoCover`, so an
/// in-range NPC never took cover. Reverting the step-2 early return makes
/// this `NoCover`.
#[test]
fn in_range_npc_takes_a_nearby_slot() {
    let cover = cover_with(vec![n(1, 0, 4.0, 0.0, 0.0)]);
    let dec = decide(q(1, Vector3::zero(), Vector3::new(20.0, 0.0, 0.0)), &cover);
    assert!(
        matches!(dec, CoverDecision::MoveToCover { slot, .. } if slot == CoverSlotKey::new(1, 0)),
        "in range with a slot 4 u away: {dec:?}"
    );
}

/// Cover is a firing position: a slot the target is out of attack range
/// from is never picked, however good it scores otherwise.
#[test]
fn slot_beyond_attack_range_is_never_picked() {
    // Slot 10 u behind the NPC: 50 u from a threat 40 u out.
    let cover = cover_with(vec![n(1, 0, -10.0, 0.0, 0.0)]);
    let (dec, trace) = maintain_cover_for_npc_traced(
        q(1, Vector3::zero(), Vector3::new(40.0, 0.0, 0.0)),
        &cover,
        &CoverWeights::default(),
    );
    assert_eq!(dec, CoverDecision::NoCover);
    assert_eq!(trace.no_cover, Some(NoCoverReason::NoCandidateInRadius));
    assert_eq!(trace.pick.unwrap().out_of_reach, 1);
}

/// An NPC with a shot does not cross the room for cover: the walk is capped
/// at `IN_RANGE_MAX_MOVE`.
#[test]
fn in_range_seek_ignores_a_long_walk() {
    let far = IN_RANGE_MAX_MOVE + 3.0;
    let cover = cover_with(vec![n(1, 0, 0.0, far, 0.0)]);
    let (dec, trace) = maintain_cover_for_npc_traced(
        q(1, Vector3::zero(), Vector3::new(15.0, 0.0, 0.0)),
        &cover,
        &CoverWeights::default(),
    );
    assert_eq!(dec, CoverDecision::NoCover);
    assert_eq!(trace.no_cover, Some(NoCoverReason::InRangeNoBetterSlot));
}

/// The seek hysteresis: an in-range seek that found nothing is not retried
/// on the next tick, only after `SEEK_RETRY`.
#[test]
fn failed_in_range_seek_waits_out_the_retry_window() {
    let cover = cover_with(Vec::new());
    let first = q(1, Vector3::zero(), Vector3::new(15.0, 0.0, 0.0));
    let weights = CoverWeights::default();
    let (_, t1) = maintain_cover_for_npc_traced(first, &cover, &weights);
    assert_eq!(t1.no_cover, Some(NoCoverReason::InRangeNoBetterSlot));

    let (_, t2) = maintain_cover_for_npc_traced(
        CoverQuery {
            now: first.now + Duration::from_secs(1),
            ..first
        },
        &cover,
        &weights,
    );
    assert_eq!(
        t2.no_cover,
        Some(NoCoverReason::SeekCooldown),
        "the tick after a failed seek must not rescan"
    );
    let (_, t3) = maintain_cover_for_npc_traced(
        CoverQuery {
            now: first.now + SEEK_RETRY + Duration::from_millis(1),
            ..first
        },
        &cover,
        &weights,
    );
    assert_eq!(t3.no_cover, Some(NoCoverReason::InRangeNoBetterSlot));
}

#[test]
fn flanked_npc_releases_and_returns_released() {
    // Cover at (5,0,0) facing +X (orient=0). NPC reserves it.
    let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
    reserve(&cover, 42, CoverSlotKey::new(1, 0));

    // Threat moves to (-20, 0, 0) — flanked.
    let dec = decide(
        q(
            42,
            Vector3::new(5.0, 0.0, 0.0),
            Vector3::new(-20.0, 0.0, 0.0),
        ),
        &cover,
    );
    match dec {
        CoverDecision::Released { prior_slot, reason } => {
            assert_eq!(prior_slot, CoverSlotKey::new(1, 0));
            assert_eq!(reason, ReleaseReason::Flanked);
            let r = cover.reservations.lock().unwrap();
            assert!(r.holder(prior_slot).is_none());
        }
        other => panic!("expected Released, got {other:?}"),
    }
}

/// A held slot the target has moved out of range of is released, so the
/// NPC can advance to a closer one. Reverting the reach test keeps the NPC
/// behind a desk it cannot shoot from.
#[test]
fn held_slot_is_released_when_the_target_leaves_range() {
    let cover = cover_with(vec![n(1, 0, 0.0, 0.0, 0.0)]);
    reserve(&cover, 42, CoverSlotKey::new(1, 0));
    let dec = decide(q(42, Vector3::zero(), Vector3::new(45.0, 0.0, 0.0)), &cover);
    assert_eq!(
        dec,
        CoverDecision::Released {
            prior_slot: CoverSlotKey::new(1, 0),
            reason: ReleaseReason::OutOfRange,
        }
    );
}

#[test]
fn stay_in_cover_when_not_flanked_and_in_range() {
    let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
    reserve(&cover, 42, CoverSlotKey::new(1, 0));

    // Threat in defensive arc (still in front of cover orient), in range.
    let dec = decide(
        q(
            42,
            Vector3::new(5.0, 0.0, 0.0),
            Vector3::new(25.0, 0.0, 0.0),
        ),
        &cover,
    );
    match dec {
        CoverDecision::StayInCover { slot, .. } => {
            assert_eq!(slot, CoverSlotKey::new(1, 0));
            let r = cover.reservations.lock().unwrap();
            assert_eq!(r.holder(slot), Some(EntityId(42)));
        }
        other => panic!("expected StayInCover, got {other:?}"),
    }
}

#[test]
fn squad_affinity_routes_two_npcs_to_different_obstacles() {
    // NPC at origin, threat at +X (20,0,0).
    //  - (1,0) at 5.5 is held by NPC #1.
    //  - (1,1) at 5.0 is open and the best baseline geometry, but it is
    //    0.5 u from the ally: penalised.
    //  - (2,0) at 8.0 is 2.5 u from the ally, outside the affinity radius.
    // Removing the affinity term sends NPC #2 to (1,1).
    let cover = cover_with(vec![
        n(1, 0, 5.5, 0.0, 0.0),
        n(1, 1, 5.0, 0.0, 0.0),
        n(2, 0, 8.0, 0.0, 0.0),
    ]);
    reserve(&cover, 1, CoverSlotKey::new(1, 0));

    let dec = decide(q(2, Vector3::zero(), Vector3::new(20.0, 0.0, 0.0)), &cover);
    match dec {
        CoverDecision::MoveToCover { slot, .. } => assert_eq!(
            slot,
            CoverSlotKey::new(2, 0),
            "squad affinity must steer the second NPC off the ally's obstacle"
        ),
        other => panic!("expected MoveToCover into (2,0), got {other:?}"),
    }
}

/// NA21's transitive grouping makes big sets. An ally holding a slot in the
/// same set but far away must not penalise a candidate: affinity counts
/// allies near the node, not allies in the set. With the old per-set count
/// the ally's set loses to the worse slot in set 2.
#[test]
fn squad_affinity_ignores_a_distant_ally_in_the_same_set() {
    let cover = cover_with(vec![
        n(1, 0, 5.0, 12.0, 0.0), // held by the ally, 12 u away
        n(1, 1, 5.0, 0.0, 0.0),  // same set, best geometry
        n(2, 0, 7.0, 0.0, 0.0),  // other set, slightly worse
    ]);
    reserve(&cover, 1, CoverSlotKey::new(1, 0));
    let dec = decide(q(2, Vector3::zero(), Vector3::new(20.0, 0.0, 0.0)), &cover);
    assert!(
        matches!(dec, CoverDecision::MoveToCover { slot, .. } if slot == CoverSlotKey::new(1, 1)),
        "a distant ally in the same set must not push the NPC off the best slot: {dec:?}"
    );
}

/// **Negative-logging regression guard (audit #482 P0).** When the
/// reservation table reports a slot already held by a *different*
/// entity, the reserve attempt must emit a structured `warn!` with
/// `target = "cover.reservation"` and `reason = "cover_slot_taken"`
/// before falling back to `Err(())`.
///
/// Bug shape this catches: revert the warn to a bare `Err(())`
/// return and the test asserts on a missing event.
#[test]
fn try_reserve_warns_when_slot_taken_by_other_holder() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();

    let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
    reserve(&cover, 99, CoverSlotKey::new(1, 0));

    let mut guard = cover.reservations.lock().unwrap();
    let result = try_reserve_or_warn(&mut guard, EntityId(42), CoverSlotKey::new(1, 0));
    drop(guard);

    assert!(
        result.is_err(),
        "race-lost path must return Err so the caller falls back to NoCover"
    );

    let event = capture
        .find_event(
            Level::WARN,
            "cover reserve_for_entity lost the race",
            "cover_slot_taken",
        )
        .unwrap_or_else(|| {
            panic!(
                "must emit warn at target=cover.reservation with \
                 reason=cover_slot_taken. Captured: {:#?}",
                capture.all()
            )
        });
    assert!(event.has_field("npc_id", "42"), "{event:#?}");
    assert!(event.has_field("holder", "99"), "{event:#?}");
    assert!(event.has_field("chunk_id", "1"), "{event:#?}");
}

/// Same helper, but called for the entity that ALREADY holds the
/// slot — idempotent re-reserve must NOT emit a warn.
#[test]
fn try_reserve_no_warn_on_idempotent_reserve_by_same_holder() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();

    let cover = cover_with(vec![n(1, 0, 5.0, 0.0, 0.0)]);
    reserve(&cover, 42, CoverSlotKey::new(1, 0));

    let mut guard = cover.reservations.lock().unwrap();
    let result = try_reserve_or_warn(&mut guard, EntityId(42), CoverSlotKey::new(1, 0));
    drop(guard);

    assert!(result.is_ok());
    assert!(
        capture
            .find_event(
                Level::WARN,
                "cover reserve_for_entity lost the race",
                "cover_slot_taken"
            )
            .is_none(),
        "idempotent re-reserve must NOT emit the race-lost warn"
    );
}
