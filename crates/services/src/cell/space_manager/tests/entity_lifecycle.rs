//! Per-entity lifecycle on `SpaceManager`: create / destroy / connect /
//! position update plus the spatial-grid sync that AoI queries depend on.

use cimmeria_common::Vector3;

use super::make_manager;

#[test]
fn create_entity_in_startup_space() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    assert_eq!(space_id, 65536);
    assert!(mgr.spaces[&65536].entities.contains_key(&100));
}

#[test]
fn create_entity_in_instanced_space() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(200, "SGC_W1", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    assert_eq!(space_id, 65538);
    assert!(mgr.spaces[&65538].entities.contains_key(&200));
}

#[test]
fn destroy_entity_removes_from_space() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.destroy_entity(100);
    assert!(!mgr.spaces[&65536].entities.contains_key(&100));
    assert!(!mgr.entity_space.contains_key(&100));
}

/// Only players can stand on a ring pad, so only a player's teardown
/// belongs in the ring's `pending_player_gone` queue. Destroying an NPC —
/// the overwhelmingly common case: mission despawns, GM `.despawn`, the
/// respawn sweep — must leave it untouched (PR #662 review, finding 2).
#[test]
fn destroying_an_npc_does_not_queue_a_ring_player_gone() {
    let mut mgr = make_manager();
    mgr.spawn_npc(900, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.destroy_entity(900);
    assert!(
        mgr.ring_transporters.take_pending_player_gone().is_empty(),
        "an NPC teardown must not enter the ring transporter's pending \
         player-gone queue",
    );
}

/// The other half of the same gate: a player's teardown still has to
/// queue, or a destroy mid-trip leaves the pad parked out of `Idle` and
/// removes it from every peer in the mesh (audit H-B3).
#[test]
fn destroying_a_player_still_queues_a_ring_player_gone() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
    }
    mgr.destroy_entity(100);
    assert_eq!(
        mgr.ring_transporters.take_pending_player_gone(),
        vec![100],
        "a player teardown must still queue the ring release",
    );
}

#[test]
fn connect_entity_marks_as_player() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    assert!(mgr.spaces[&65536].players.contains(&100));
}

#[test]
fn update_entity_position() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.update_entity_position(100, [50.0, 5.0, 60.0], [0, 0, 0], [0.0; 3]);
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(entity.position, Vector3::new(50.0, 5.0, 60.0));
}

/// `update_entity_position` must keep the spatial grid in sync with
/// the entity's `position`. A grid update miss would let AoI queries
/// return stale neighbours.
#[test]
fn update_entity_position_keeps_spatial_grid_consistent() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    let space_id = mgr.entity_space[&100];

    // Move far enough to land in a different grid cell.
    mgr.update_entity_position(100, [500.0, 0.0, 500.0], [0, 0, 0], [0.0; 3]);

    // Assert via the grid: an `aoi_radius` query around the new position
    // must include the entity. A missed grid update would drop it.
    let space = &mgr.spaces[&space_id];
    let near = space
        .space
        .get_entities_in_range(&Vector3::new(500.0, 0.0, 500.0), 50.0);
    assert!(
        near.iter().any(|eid| eid.0 == 100),
        "entity 100 must be reachable from the spatial grid at its new position"
    );
}
