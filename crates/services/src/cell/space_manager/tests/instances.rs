//! Instanced-space lifecycle and scoping invariants: spaces are torn down
//! when the last player leaves (and NPCs go with them), non-instanced
//! spaces persist, each connecting player gets their own instance, and
//! tag lookups never bleed across instances.

use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;

use super::make_manager;

/// When the last player leaves an instanced space, the entire space
/// instance must be destroyed AND every NPC inside it must be removed
/// from the entity_space index. A regression that just removes the
/// player would leak instances on every map reload.
///
/// Note: `create_entity` on an instanced world allocates a fresh
/// instance per call, so we can't just call it twice with the same
/// world name and expect both entities to share a space. We insert
/// the NPC directly into the player's instance using the same
/// pattern as `find_by_tag_does_not_cross_instance_boundaries` below.
#[test]
fn destroy_last_player_in_instanced_space_destroys_the_whole_space() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Castle_CellBlock", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);

    // Place an NPC directly into the player's instance.
    let npc_id: u32 = 500;
    {
        let entity = CellEntity::new(
            EntityId(npc_id as i32),
            SpaceId(space_id as i32),
            Vector3::default(),
        );
        mgr.spaces
            .get_mut(&space_id)
            .unwrap()
            .entities
            .insert(npc_id, entity);
        mgr.entity_space.insert(npc_id, space_id);
    }

    assert!(mgr.spaces.contains_key(&space_id));
    assert!(mgr.spaces[&space_id].entities.contains_key(&npc_id));

    mgr.destroy_entity(100);

    assert!(
        !mgr.spaces.contains_key(&space_id),
        "instanced space {space_id} must be destroyed when the last player leaves",
    );
    assert!(
        !mgr.entity_space.contains_key(&npc_id),
        "NPC entity_space mapping must be cleared when the host instance is destroyed"
    );
}

/// `find_entity_by_tag` and `find_entities_by_template` both restrict
/// the search to the source entity's space. Pin the cross-instance
/// invariant: a tagged entity in one Castle_CellBlock instance is
/// invisible to a source in a different instance.
#[test]
fn find_by_tag_does_not_cross_instance_boundaries() {
    let mut mgr = make_manager();
    let inst_a = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    let inst_b = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    assert_ne!(inst_a, inst_b, "fixture sanity: two distinct instances");

    // Source player in instance A.
    {
        let mut entity = CellEntity::new(EntityId(100), SpaceId(inst_a as i32), Vector3::default());
        entity.is_player = true;
        mgr.spaces
            .get_mut(&inst_a)
            .unwrap()
            .entities
            .insert(100, entity);
        mgr.entity_space.insert(100, inst_a);
    }
    // Tagged NPC in instance B.
    {
        let mut entity = CellEntity::new(EntityId(500), SpaceId(inst_b as i32), Vector3::default());
        entity.tag = Some("Boss".to_string());
        mgr.spaces
            .get_mut(&inst_b)
            .unwrap()
            .entities
            .insert(500, entity);
        mgr.entity_space.insert(500, inst_b);
    }

    assert_eq!(
        mgr.find_entity_by_tag(100, "Boss"),
        None,
        "tag lookup must be scoped to the source entity's instance"
    );
}

#[test]
fn instanced_space_destroyed_when_last_player_leaves() {
    let mut mgr = make_manager();

    // Create a player entity in an instanced space
    let space_id = mgr
        .create_entity(200, "Castle_CellBlock", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(200);

    // Manually add an NPC into the same instanced space (simulates what
    // spawn_instance_npcs_from_records does with a specific space_id)
    let npc_pos = Vector3::new(10.0, 0.0, 10.0);
    let mut npc = CellEntity::new(EntityId(100_000), SpaceId(space_id as i32), npc_pos);
    npc.class_id = 0x04;
    npc.is_player = false;
    let space = mgr.spaces.get_mut(&space_id).unwrap();
    space.space.add_entity(EntityId(100_000), &npc_pos);
    space.entities.insert(100_000, npc);
    mgr.entity_space.insert(100_000, space_id);

    assert!(mgr.spaces.contains_key(&space_id));
    assert_eq!(mgr.spaces[&space_id].entities.len(), 2); // player + NPC

    // Destroy the player — last player in the instance, should destroy the space
    mgr.destroy_entity(200);

    // Space should be fully cleaned up
    assert!(!mgr.spaces.contains_key(&space_id));
    assert!(!mgr.entity_space.contains_key(&200));
    assert!(!mgr.entity_space.contains_key(&100_000)); // NPC also cleaned up
}

#[test]
fn non_instanced_space_survives_player_leaving() {
    let mut mgr = make_manager();

    // Create a player in a non-instanced startup space
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);

    // Destroy the player — non-instanced space should NOT be destroyed
    mgr.destroy_entity(100);

    assert!(mgr.spaces.contains_key(&65536)); // Agnos space still exists
}

#[test]
fn two_players_get_separate_instances() {
    let mut mgr = make_manager();

    let space1 = mgr
        .create_entity(100, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let space2 = mgr
        .create_entity(200, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    assert_ne!(space1, space2);
    assert!(mgr.spaces.contains_key(&space1));
    assert!(mgr.spaces.contains_key(&space2));

    // Each space has exactly one entity
    assert_eq!(mgr.spaces[&space1].entities.len(), 1);
    assert_eq!(mgr.spaces[&space2].entities.len(), 1);
}
