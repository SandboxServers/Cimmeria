use super::super::messages::CellToBaseMsg;
use super::*;
use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;

const TEST_SPACES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    <Space WorldName="SGC_W1" Instanced="true" MinX="-400" MaxX="400" MinY="-400" MaxY="800" />
</Spaces>"#;

const TEST_CELL_SPACES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Castle" />
</Spaces>"#;

fn make_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(TEST_SPACES_XML).unwrap();
    mgr.create_startup_spaces(TEST_CELL_SPACES_XML).unwrap();
    mgr
}

#[test]
fn parse_spaces_xml_loads_all_worlds() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(TEST_SPACES_XML).unwrap();
    assert_eq!(mgr.world_count(), 4);
    assert!(mgr.worlds.contains_key("Agnos"));
    assert!(mgr.worlds.contains_key("Castle_CellBlock"));
    assert!(mgr.worlds["Castle_CellBlock"].instanced);
    assert!(!mgr.worlds["Agnos"].instanced);
}

#[test]
fn startup_spaces_get_correct_ids() {
    let mgr = make_manager();
    assert_eq!(mgr.space_count(), 2);
    // cell_id=1: first space = (1<<16)|0 = 65536, second = 65537
    assert_eq!(mgr.space_id_for_world("Agnos"), Some(65536));
    assert_eq!(mgr.space_id_for_world("Castle"), Some(65537));
}

#[test]
fn instanced_space_created_on_demand() {
    let mut mgr = make_manager();
    assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);

    let id1 = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    assert_eq!(id1, 65538); // next after 65536, 65537

    // Each call creates a NEW instance — they should NOT share a space
    let id2 = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    assert_eq!(id2, 65539);
    assert_ne!(id1, id2);

    // Instanced spaces are NOT cached in world_spaces
    assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);
}

#[test]
fn unknown_world_returns_error() {
    let mut mgr = make_manager();
    assert!(mgr.find_or_create_space("Narnia").is_err());
}

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

#[test]
fn aoi_detects_nearby_players() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    let events = mgr.compute_aoi_changes();

    // Both players should see each other enter AoI
    let entered: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. }))
        .collect();
    assert_eq!(entered.len(), 2);
}

#[test]
fn aoi_detects_entity_leaving() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    // First tick: both enter AoI
    let _ = mgr.compute_aoi_changes();

    // Move entity 200 far away
    mgr.update_entity_position(200, [5000.0, 0.0, 5000.0], [0, 0, 0], [0.0; 3]);

    // Second tick: entity 200 should leave AoI of entity 100
    let events = mgr.compute_aoi_changes();
    let left: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::LeftAoI { .. }))
        .collect();
    assert_eq!(left.len(), 2); // Both should lose sight of each other
}

/// AoI second-tick path: when an entity is in BOTH the previous and
/// current witness sets, an `EntityMoved` event must fire so the
/// witness gets the position update. This covers the third arm of
/// `compute_aoi_changes` that the existing entered/left tests don't.
#[test]
fn aoi_emits_entity_moved_for_entities_in_both_previous_and_current_sets() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    // First tick populates the witness set.
    let _ = mgr.compute_aoi_changes();
    // Move 200 a small amount so it's still in 100's AoI.
    mgr.update_entity_position(200, [25.0, 0.0, 25.0], [0, 0, 0], [1.0, 0.0, 0.0]);
    let events = mgr.compute_aoi_changes();
    let moved: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            CellToBaseMsg::EntityMoved {
                witness_id,
                entity_id,
                velocity,
                ..
            } if *witness_id == 100 && *entity_id == 200 => Some(*velocity),
            _ => None,
        })
        .collect();
    assert_eq!(
        moved.len(),
        1,
        "expected exactly one EntityMoved for 200 to witness 100; got events={events:?}"
    );
    assert_eq!(
        moved[0],
        [1.0, 0.0, 0.0],
        "EntityMoved must carry the velocity from update_entity_position"
    );
}

/// `compute_aoi_changes` must persist the new witness set on the
/// player so that the NEXT tick's diff sees it. A regression that
/// drops the `entity.witnesses = current_aoi` write would re-fire
/// EnteredAoI on every tick.
#[test]
fn aoi_persists_witness_set_so_subsequent_ticks_diff_correctly() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    let first = mgr.compute_aoi_changes();
    let first_entered = first
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. }))
        .count();
    assert!(first_entered >= 2, "first tick must populate AoI");

    // Second tick with no movement: no further EnteredAoI events.
    let second = mgr.compute_aoi_changes();
    let second_entered = second
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. }))
        .count();
    assert_eq!(
        second_entered, 0,
        "second tick must not re-fire EnteredAoI; the witness set should already pin both",
    );
}

/// Disconnecting a player must broadcast `LeftAoI` to every other
/// player who currently has that player in their witness set.
/// Regression guard: the broadcast iterates `space.players`, not the
/// disconnecting entity's own `witnesses` (those point in the wrong
/// direction).
#[tokio::test]
async fn disconnect_emits_left_aoi_to_observers_then_destroys_entity() {
    use tokio::sync::mpsc;
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);
    let _ = mgr.compute_aoi_changes(); // populate witness sets

    let (tx, mut rx) = mpsc::channel(8);
    mgr.disconnect_entity(100, &tx).await;

    let mut left_msgs: Vec<(u32, u32)> = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } = m
        {
            left_msgs.push((witness_id, entity_id));
        }
    }
    assert_eq!(
        left_msgs,
        vec![(200, 100)],
        "player 200 must receive LeftAoI for the disconnecting 100"
    );
    // The disconnecting entity must be fully destroyed (entity_space
    // entry removed) — `destroy_entity` chains off the disconnect path.
    assert!(!mgr.entity_space.contains_key(&100));
}

/// When the last player leaves an instanced space, the entire space
/// instance must be destroyed (all NPCs removed too). A regression
/// that just removes the player would leak instances on every map
/// reload.
#[test]
fn destroy_last_player_in_instanced_space_destroys_the_whole_space() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Castle_CellBlock", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    // Seed an NPC into the same instanced space.
    mgr.create_entity(500, "Castle_CellBlock", [10.0, 0.0, 10.0], [0.0; 3])
        .ok(); // either the same instance or a new one
    let space_existed = mgr.spaces.contains_key(&space_id);
    assert!(space_existed);

    mgr.destroy_entity(100);

    assert!(
        !mgr.spaces.contains_key(&space_id),
        "instanced space {space_id} must be destroyed when the last player leaves",
    );
}

/// `get_witnesses_of` returns the player IDs whose current `witnesses`
/// set contains the target. Restricted to the target's space so
/// instanced worlds don't bleed across instances.
#[test]
fn get_witnesses_of_returns_only_players_in_the_targets_space() {
    let mut mgr = make_manager();
    // Player + NPC in Agnos.
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(900, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    // Player + NPC in Castle (different startup space).
    mgr.create_entity(101, "Castle", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(901, "Castle", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(101);

    let _ = mgr.compute_aoi_changes();

    // Player 100 sees NPC 900; player 101 sees NPC 901. Neither sees
    // the other space's NPC.
    let witnesses_900 = mgr.get_witnesses_of(900);
    assert_eq!(witnesses_900, vec![100]);
    let witnesses_901 = mgr.get_witnesses_of(901);
    assert_eq!(witnesses_901, vec![101]);
}

#[test]
fn get_witnesses_of_returns_empty_for_missing_entity() {
    let mgr = make_manager();
    assert!(mgr.get_witnesses_of(999_999).is_empty());
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
        let mut entity =
            CellEntity::new(EntityId(100), SpaceId(inst_a as i32), Vector3::default());
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
        let mut entity =
            CellEntity::new(EntityId(500), SpaceId(inst_b as i32), Vector3::default());
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

#[test]
fn space_id_scheme() {
    let mut mgr = SpaceManager::new(1);
    assert_eq!(mgr.allocate_space_id(), 65536); // (1 << 16) | 0
    assert_eq!(mgr.allocate_space_id(), 65537); // (1 << 16) | 1
    assert_eq!(mgr.allocate_space_id(), 65538); // (1 << 16) | 2
}

#[test]
fn full_xml_file_loading() {
    // Test with the actual XML content (same structure as files)
    let mut mgr = SpaceManager::new(1);

    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Agnos_Library" Instanced="false" MinX="-600" MaxX="600" MinY="-600" MaxY="600" />
    <Space WorldName="Beta_Site_Evo_1" Instanced="false" MinX="-1600" MaxX="2600" MinY="-3000" MaxY="3000" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    <Space WorldName="SGC_W1" Instanced="true" MinX="-400" MaxX="400" MinY="-400" MaxY="800" />
</Spaces>"#;

    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Agnos_Library" />
    <Space WorldName="Beta_Site_Evo_1" />
    <Space WorldName="Castle" />
</Spaces>"#;

    mgr.parse_spaces_xml(spaces_xml).unwrap();
    assert_eq!(mgr.world_count(), 6);

    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    assert_eq!(mgr.space_count(), 4);

    // Startup spaces get sequential IDs
    assert_eq!(mgr.space_id_for_world("Agnos"), Some(65536));
    assert_eq!(mgr.space_id_for_world("Agnos_Library"), Some(65537));
    assert_eq!(mgr.space_id_for_world("Beta_Site_Evo_1"), Some(65538));
    assert_eq!(mgr.space_id_for_world("Castle"), Some(65539));

    // Instanced worlds not yet created
    assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);
    assert_eq!(mgr.space_id_for_world("SGC_W1"), None);
}

// ── NPC spawn and AI ─────────────────────────────────────────────────

#[test]
fn spawn_npc_sets_class_id_and_spawn_position() {
    let mut mgr = make_manager();
    let pos = [50.0, 0.0, 75.0];
    mgr.spawn_npc(500, "Agnos", pos, [0.0; 3]).unwrap();

    let npc = mgr.get_entity(500).unwrap();
    assert_eq!(npc.class_id, 0x04); // SGWMob
    assert!(!npc.is_player);
    assert_eq!(npc.spawn_position.unwrap(), Vector3::new(50.0, 0.0, 75.0));
}

#[test]
fn spawn_npc_gets_default_ability() {
    let mut mgr = make_manager();
    mgr.spawn_npc(500, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    let npc = mgr.get_entity(500).unwrap();
    assert!(npc
        .abilities
        .has_ability(crate::cell::combat::NPC_DEFAULT_ABILITY));
}

#[test]
fn all_npc_entity_ids_returns_only_npcs() {
    let mut mgr = make_manager();
    // Add a player entity
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.connect_entity(1);
    // Add two NPC entities
    mgr.spawn_npc(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();

    let npc_ids = mgr.all_npc_entity_ids();
    assert_eq!(npc_ids.len(), 2);
    assert!(npc_ids.contains(&100));
    assert!(npc_ids.contains(&200));
    // Player should NOT be in the list
    assert!(!npc_ids.contains(&1));
}

#[test]
fn all_npc_entity_ids_empty_when_no_npcs() {
    let mgr = make_manager();
    assert!(mgr.all_npc_entity_ids().is_empty());
}

#[test]
fn get_entity_world_name_for_npc() {
    let mut mgr = make_manager();
    mgr.spawn_npc(500, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    assert_eq!(mgr.get_entity_world_name(500), Some("Agnos".to_string()));
}

#[test]
fn spawn_npc_from_record_sets_template_fields() {
    use crate::cell::spawner::SpawnRecord;
    let mut mgr = make_manager();
    let record = SpawnRecord {
        spawn_id: 1,
        world_name: "Agnos".to_string(),
        x: 10.0,
        y: 0.0,
        z: 20.0,
        heading: 1.5,
        class: "SGWMob".to_string(),
        template_id: 42,
        template_name: "TestGuard".to_string(),
        tag: Some("Guard01".to_string()),
        name_id: Some(1001),
        speaker_id: None,
        event_set_id: None,
        interaction_type: 0,
        flags: 0,
        faction: Some(10),
        alignment: Some(1),
        level: Some(5),
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        static_mesh: None,
        body_set: "BS_NID_Soldier.BS_NID_Soldier".to_string(),
        components: Some(vec!["Comp1".to_string()]),
        loot_table_id: Some(2),
        is_stationary: false,
    };

    mgr.spawn_npc_from_record(600, &record).unwrap();

    let npc = mgr.get_entity(600).unwrap();
    assert_eq!(npc.template_id, Some(42));
    assert_eq!(npc.tag.as_deref(), Some("Guard01"));
    assert_eq!(npc.faction, 10);
    assert_eq!(npc.alignment, 1);
    assert_eq!(npc.level, 5);
    assert_eq!(npc.spawn_position.unwrap(), Vector3::new(10.0, 0.0, 20.0));
    // 592 = NPC_DEFAULT_ABILITY (Pistol Shot — was previously 597/Heal Focus).
    assert!(npc.abilities.has_ability(592));
    // Health should be scaled: 200 + (5 * 50) = 450
    assert_eq!(
        npc.stats.get(cimmeria_entity::stats::HEALTH).unwrap().max,
        450
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
