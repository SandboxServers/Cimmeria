//! Unit tests for the cover module's pure components: spatial index +
//! reservation lifecycle. Live-DB tests for the loader live alongside
//! the other `cimmeria-services` live-DB tests; see
//! `crates/services/src/cell/cover/tests.rs::live_db_loader_tests`.

use cimmeria_common::{EntityId, Vector3};

use super::reservation::{CoverReservations, ReserveError};
use super::spatial::CoverIndex;
use super::types::{CoverHeight, CoverNode, CoverQuality, CoverSlotKey};

fn node(chunk_id: i32, node_id: i32, x: f32, z: f32) -> CoverNode {
    CoverNode {
        chunk_id,
        node_id,
        pos: Vector3::new(x, 0.0, z),
        orient: 0.0,
        height: CoverHeight::Mid,
        quality: CoverQuality::Best,
        tail: [0; 4],
    }
}

// ─── reservation tests ──────────────────────────────────────────────

#[test]
fn reserve_succeeds_on_empty_table() {
    let mut r = CoverReservations::new();
    let slot = CoverSlotKey::new(1, 0);
    assert!(r.reserve_for_entity(EntityId(10), slot).is_ok());
    assert_eq!(r.holder(slot), Some(EntityId(10)));
    assert_eq!(r.reserved_count(), 1);
}

#[test]
fn reserve_same_slot_by_holder_is_idempotent() {
    let mut r = CoverReservations::new();
    let slot = CoverSlotKey::new(1, 0);
    r.reserve_for_entity(EntityId(10), slot).unwrap();
    // Idempotent re-reserve must succeed (NPC ticking while still in cover).
    assert!(r.reserve_for_entity(EntityId(10), slot).is_ok());
    assert_eq!(r.reserved_count(), 1);
}

#[test]
fn reserve_collision_returns_holder() {
    let mut r = CoverReservations::new();
    let slot = CoverSlotKey::new(1, 0);
    r.reserve_for_entity(EntityId(10), slot).unwrap();
    match r.reserve_for_entity(EntityId(11), slot) {
        Err(ReserveError::AlreadyReserved { holder }) => assert_eq!(holder, EntityId(10)),
        other => panic!("expected AlreadyReserved, got {other:?}"),
    }
    // Original holder unchanged.
    assert_eq!(r.holder(slot), Some(EntityId(10)));
}

#[test]
fn reserve_auto_releases_prior_slot() {
    let mut r = CoverReservations::new();
    let slot_a = CoverSlotKey::new(1, 0);
    let slot_b = CoverSlotKey::new(2, 5);
    r.reserve_for_entity(EntityId(10), slot_a).unwrap();
    // Auto-release-prior semantics from SGWCoverSet.def — re-reserving
    // a different slot must atomically release the prior one.
    r.reserve_for_entity(EntityId(10), slot_b).unwrap();
    assert!(!r.is_reserved(slot_a), "prior slot must be released");
    assert_eq!(r.holder(slot_b), Some(EntityId(10)));
    assert_eq!(r.reserved_count(), 1);
}

#[test]
fn release_for_entity_clears_both_maps() {
    let mut r = CoverReservations::new();
    let slot = CoverSlotKey::new(7, 3);
    r.reserve_for_entity(EntityId(42), slot).unwrap();
    let released = r.release_for_entity(EntityId(42));
    assert_eq!(released, Some(slot));
    assert!(!r.is_reserved(slot));
    assert_eq!(r.slot_for_entity(EntityId(42)), None);
    // Idempotent — second release returns None, no panic.
    assert_eq!(r.release_for_entity(EntityId(42)), None);
}

#[test]
fn release_slot_idempotent_when_unreserved() {
    let mut r = CoverReservations::new();
    let slot = CoverSlotKey::new(1, 0);
    assert!(!r.release_slot(slot));
}

// ─── spatial-index tests ───────────────────────────────────────────

#[test]
fn empty_index_returns_no_hits() {
    let idx = CoverIndex::empty();
    assert_eq!(idx.node_count(), 0);
    assert!(idx
        .nearby(&Vector3::new(0.0, 0.0, 0.0), 100.0, None)
        .is_empty());
}

#[test]
fn nearby_returns_sorted_by_distance() {
    // Place 3 nodes at known offsets from origin.
    let nodes = vec![
        node(1, 0, 5.0, 0.0),  // dist = 5
        node(1, 1, 20.0, 0.0), // dist = 20
        node(1, 2, 10.0, 0.0), // dist = 10
    ];
    let idx = CoverIndex::build(nodes);

    let hits = idx.nearby(&Vector3::new(0.0, 0.0, 0.0), 30.0, None);
    assert_eq!(hits.len(), 3, "all three nodes are within 30 m");
    // Order must be ascending distance.
    assert_eq!(idx.node(hits[0]).unwrap().node_id, 0, "closest first");
    assert_eq!(idx.node(hits[1]).unwrap().node_id, 2);
    assert_eq!(idx.node(hits[2]).unwrap().node_id, 1, "farthest last");
}

#[test]
fn nearby_excludes_nodes_outside_radius() {
    let nodes = vec![
        node(1, 0, 5.0, 0.0),  // inside r=10
        node(1, 1, 15.0, 0.0), // outside r=10
        node(1, 2, 0.0, 8.0),  // inside r=10
    ];
    let idx = CoverIndex::build(nodes);
    let hits = idx.nearby(&Vector3::new(0.0, 0.0, 0.0), 10.0, None);
    assert_eq!(hits.len(), 2);
    let ids: Vec<_> = hits.iter().map(|i| idx.node(*i).unwrap().node_id).collect();
    assert!(ids.contains(&0));
    assert!(ids.contains(&2));
    assert!(
        !ids.contains(&1),
        "node at distance 15 must be excluded for radius 10"
    );
}

#[test]
fn nearby_y_axis_filter_excludes_different_floors() {
    let nodes = vec![
        CoverNode {
            chunk_id: 1,
            node_id: 0,
            pos: Vector3::new(2.0, 0.0, 2.0),
            orient: 0.0,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            tail: [0; 4],
        },
        CoverNode {
            chunk_id: 1,
            node_id: 1,
            // Same XZ but 10m up — different floor.
            pos: Vector3::new(2.0, 10.0, 2.0),
            orient: 0.0,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            tail: [0; 4],
        },
    ];
    let idx = CoverIndex::build(nodes);
    // Without Y filter both match.
    assert_eq!(idx.nearby(&Vector3::zero(), 20.0, None).len(), 2);
    // With 2-m Y tolerance only node 0 matches.
    let hits = idx.nearby(&Vector3::zero(), 20.0, Some(2.0));
    assert_eq!(hits.len(), 1);
    assert_eq!(idx.node(hits[0]).unwrap().node_id, 0);
}

#[test]
fn nodes_in_chunk_returns_only_matching() {
    let nodes = vec![
        node(1, 0, 0.0, 0.0),
        node(1, 1, 1.0, 1.0),
        node(2, 0, 50.0, 50.0),
        node(2, 1, 51.0, 51.0),
        node(3, 0, 100.0, 100.0),
    ];
    let idx = CoverIndex::build(nodes);
    let chunk_1_nodes: Vec<_> = idx.nodes_in_chunk(1).map(|(_, n)| n.node_id).collect();
    assert_eq!(chunk_1_nodes, vec![0, 1]);
    let chunk_3_nodes: Vec<_> = idx.nodes_in_chunk(3).map(|(_, n)| n.node_id).collect();
    assert_eq!(chunk_3_nodes, vec![0]);
}

#[test]
fn node_by_key_finds_correct_record() {
    let nodes = vec![
        node(1, 0, 0.0, 0.0),
        node(1, 1, 1.0, 1.0),
        node(2, 0, 50.0, 50.0),
    ];
    let idx = CoverIndex::build(nodes);
    let n = idx.node_by_key(CoverSlotKey::new(2, 0)).unwrap();
    assert!((n.pos.x - 50.0).abs() < 1e-4);
    assert!(idx.node_by_key(CoverSlotKey::new(99, 99)).is_none());
}

// ─── height / quality enum tests ────────────────────────────────────

#[test]
fn cover_height_sql_round_trip() {
    for (name, h) in [
        ("HEIGHT_Low", CoverHeight::Low),
        ("HEIGHT_Mid", CoverHeight::Mid),
        ("HEIGHT_High", CoverHeight::High),
        ("HEIGHT_LOS", CoverHeight::Los),
    ] {
        assert_eq!(CoverHeight::from_sql_name(name), Some(h));
    }
    assert_eq!(CoverHeight::from_sql_name("HEIGHT_Unknown"), None);
}

#[test]
fn cover_quality_score_factor_ordering() {
    // Best > Better > Good > None — ordering matters for the scorer.
    assert!(CoverQuality::Best.score_factor() > CoverQuality::Better.score_factor());
    assert!(CoverQuality::Better.score_factor() > CoverQuality::Good.score_factor());
    assert!(CoverQuality::Good.score_factor() > CoverQuality::None_.score_factor());
    assert_eq!(CoverQuality::None_.score_factor(), 0.0);
}

#[test]
fn cover_height_meters_are_monotonic() {
    // Empirically-confirmed values must stay monotonically increasing
    // by enum ordinal — a regression that swapped the constants would
    // break cover-vs-stance gating downstream.
    let h: [CoverHeight; 4] = [
        CoverHeight::Low,
        CoverHeight::Mid,
        CoverHeight::High,
        CoverHeight::Los,
    ];
    for w in h.windows(2) {
        assert!(w[0].meters() < w[1].meters(), "{:?} < {:?}", w[0], w[1]);
    }
    // Anchor specific values per the binary-confirmed RE.
    assert!((CoverHeight::Low.meters() - 0.71).abs() < 0.01);
    assert!((CoverHeight::Los.meters() - 2.52).abs() < 0.01);
}
