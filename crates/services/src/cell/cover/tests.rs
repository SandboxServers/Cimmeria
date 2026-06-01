use cimmeria_common::{EntityId, Vector3};

use super::reservation::{CoverReservations, ReserveError};
use super::spatial::CoverIndex;
use super::types::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSlotKey};

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
    assert_eq!(r.holder(slot), Some(EntityId(10)));
}

#[test]
fn reserve_auto_releases_prior_slot() {
    // Pins the SGWCoverSet.def auto-release-prior semantics — without this,
    // an NPC re-picking cover would leak the prior reservation.
    //
    // Per PR #483 review: the implicit release also emits a
    // `cover_reservation_state{state=released}` counter so the
    // derived "currently held = held − released" dashboard query
    // doesn't drift. We can't read counter values from a unit test
    // (no in-process collector), but this test pins the *behavior*
    // the counter shadows. A refactor that removes the
    // `entity_to_slot.remove` branch would trip both this test
    // (state would leak) and the counter balance simultaneously.
    let mut r = CoverReservations::new();
    let slot_a = CoverSlotKey::new(1, 0);
    let slot_b = CoverSlotKey::new(2, 5);
    r.reserve_for_entity(EntityId(10), slot_a).unwrap();
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
    assert_eq!(r.release_for_entity(EntityId(42)), None);
}

#[test]
fn release_slot_idempotent_when_unreserved() {
    let mut r = CoverReservations::new();
    let slot = CoverSlotKey::new(1, 0);
    assert!(!r.release_slot(slot));
}

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
    let nodes = vec![
        node(1, 0, 5.0, 0.0),
        node(1, 1, 20.0, 0.0),
        node(1, 2, 10.0, 0.0),
    ];
    let idx = CoverIndex::build(nodes);

    let hits = idx.nearby(&Vector3::new(0.0, 0.0, 0.0), 30.0, None);
    assert_eq!(hits.len(), 3, "all three nodes are within 30 m");
    assert_eq!(idx.node(hits[0]).unwrap().node_id, 0, "closest first");
    assert_eq!(idx.node(hits[1]).unwrap().node_id, 2);
    assert_eq!(idx.node(hits[2]).unwrap().node_id, 1, "farthest last");
}

#[test]
fn nearby_excludes_nodes_outside_radius() {
    let nodes = vec![
        node(1, 0, 5.0, 0.0),
        node(1, 1, 15.0, 0.0),
        node(1, 2, 0.0, 8.0),
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
            pos: Vector3::new(2.0, 10.0, 2.0),
            orient: 0.0,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            tail: [0; 4],
        },
    ];
    let idx = CoverIndex::build(nodes);
    assert_eq!(idx.nearby(&Vector3::zero(), 20.0, None).len(), 2);
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
    assert!(CoverQuality::Best.score_factor() > CoverQuality::Better.score_factor());
    assert!(CoverQuality::Better.score_factor() > CoverQuality::Good.score_factor());
    assert!(CoverQuality::Good.score_factor() > CoverQuality::None_.score_factor());
    assert_eq!(CoverQuality::None_.score_factor(), 0.0);
}

#[test]
fn cover_empty_has_no_nodes_no_sets_no_reservations() {
    let cover = Cover::empty();
    assert_eq!(cover.node_count(), 0, "empty cover must have 0 nodes");
    assert_eq!(cover.set_count(), 0, "empty cover must have 0 sets");
    assert_eq!(
        cover.release_for_entity(EntityId(42)),
        None,
        "release_for_entity on empty must return None"
    );
}

#[test]
fn cover_from_loaded_exposes_sets_and_node_count() {
    use super::types::CoverSetMeta;
    let sets = vec![
        CoverSetMeta {
            chunk_id: 1,
            chunk_name: "set/one".to_string(),
            primary_author: "x".to_string(),
            has_variant: false,
            src_pak: "p".to_string(),
        },
        CoverSetMeta {
            chunk_id: 2,
            chunk_name: "set/two".to_string(),
            primary_author: "x".to_string(),
            has_variant: true,
            src_pak: "p".to_string(),
        },
    ];
    let nodes = vec![node(1, 0, 0.0, 0.0), node(2, 0, 10.0, 0.0)];
    let cover = Cover::from_loaded(sets, nodes);
    assert_eq!(cover.set_count(), 2);
    assert_eq!(cover.node_count(), 2);
}

#[test]
fn cover_release_for_entity_round_trips_reserved_slot() {
    let cover = Cover::from_loaded(Vec::new(), vec![node(7, 3, 0.0, 0.0)]);
    cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(99), CoverSlotKey::new(7, 3))
        .unwrap();

    let freed = cover.release_for_entity(EntityId(99));
    assert_eq!(
        freed,
        Some(CoverSlotKey::new(7, 3)),
        "release_for_entity must return the prior slot key so callers can \
         log / re-pick. Pre-fix this could leak; the regression shape is a \
         silent None return after reservation."
    );
    // Idempotent: second call returns None.
    assert_eq!(cover.release_for_entity(EntityId(99)), None);
}

#[test]
fn reservation_iter_yields_every_held_slot() {
    let mut r = CoverReservations::new();
    r.reserve_for_entity(EntityId(1), CoverSlotKey::new(10, 0))
        .unwrap();
    r.reserve_for_entity(EntityId(2), CoverSlotKey::new(10, 1))
        .unwrap();
    r.reserve_for_entity(EntityId(3), CoverSlotKey::new(20, 0))
        .unwrap();

    let mut pairs: Vec<(EntityId, CoverSlotKey)> = r.iter().collect();
    pairs.sort_by_key(|(e, _)| e.0);
    assert_eq!(
        pairs.len(),
        3,
        "iter must enumerate every active reservation"
    );
    assert_eq!(pairs[0], (EntityId(1), CoverSlotKey::new(10, 0)));
    assert_eq!(pairs[1], (EntityId(2), CoverSlotKey::new(10, 1)));
    assert_eq!(pairs[2], (EntityId(3), CoverSlotKey::new(20, 0)));
}

#[test]
fn reservation_release_slot_returns_true_when_held_and_drops_both_maps() {
    let mut r = CoverReservations::new();
    let slot = CoverSlotKey::new(5, 0);
    r.reserve_for_entity(EntityId(10), slot).unwrap();
    assert!(
        r.release_slot(slot),
        "release_slot must return true when the slot was held"
    );
    assert!(!r.is_reserved(slot), "slot must be cleared");
    assert_eq!(
        r.slot_for_entity(EntityId(10)),
        None,
        "entity_to_slot side must also clear — without this the maps drift \
         and a re-reserve would corrupt accounting"
    );
    assert_eq!(r.reserved_count(), 0);
}

#[test]
fn spatial_all_nodes_returns_full_slice() {
    let nodes = vec![node(1, 0, 0.0, 0.0), node(2, 0, 10.0, 0.0)];
    let idx = CoverIndex::build(nodes);
    let all = idx.all_nodes();
    assert_eq!(all.len(), 2);
    // Sanity check on identity — the slice must be the same data the
    // build pass took in, not a synthetic placeholder.
    assert_eq!(all[0].chunk_id, 1);
    assert_eq!(all[1].chunk_id, 2);
}

#[test]
fn spatial_nearby_with_zero_radius_returns_empty() {
    let idx = CoverIndex::build(vec![node(1, 0, 0.0, 0.0), node(1, 1, 0.5, 0.0)]);
    // Radius 0 is a degenerate input; the loop's guard short-circuits
    // to an empty result rather than scanning every cell with `radius_sq = 0`.
    assert!(idx.nearby(&Vector3::zero(), 0.0, None).is_empty());
    // Negative radius is the more dangerous degenerate — without the
    // `radius <= 0.0` early-out the `cell_radius` cast would clamp to
    // 0 but the dist comparison would always pass for the origin.
    assert!(idx.nearby(&Vector3::zero(), -1.0, None).is_empty());
}

#[test]
fn cover_height_meters_are_monotonic() {
    // Anchors to binary-confirmed RE values — a regression that swapped
    // the constants would break cover-vs-stance gating downstream.
    let h: [CoverHeight; 4] = [
        CoverHeight::Low,
        CoverHeight::Mid,
        CoverHeight::High,
        CoverHeight::Los,
    ];
    for w in h.windows(2) {
        assert!(w[0].meters() < w[1].meters(), "{:?} < {:?}", w[0], w[1]);
    }
    assert!((CoverHeight::Low.meters() - 0.71).abs() < 0.01);
    assert!((CoverHeight::Los.meters() - 2.52).abs() < 0.01);
}
