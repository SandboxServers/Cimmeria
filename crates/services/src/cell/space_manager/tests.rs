use super::*;
use super::super::messages::CellToBaseMsg;
use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;

const TEST_SPACES_XML: &str = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    <Space WorldName="SGC_W1" Instanced="true" MinX="-400" MaxX="400" MinY="-400" MaxY="800" />
</Spaces>"#;

const TEST_CELL_SPACES_XML: &str = r#"<?xml version="1.0" charset="UTF-8"?>
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
    let space_id = mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
    assert_eq!(space_id, 65536);
    assert!(mgr.spaces[&65536].entities.contains_key(&100));
}

#[test]
fn create_entity_in_instanced_space() {
    let mut mgr = make_manager();
    let space_id = mgr.create_entity(200, "SGC_W1", [5.0, 0.0, 5.0], [0.0; 3]).unwrap();
    assert_eq!(space_id, 65538);
    assert!(mgr.spaces[&65538].entities.contains_key(&200));
}

#[test]
fn destroy_entity_removes_from_space() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
    mgr.destroy_entity(100);
    assert!(!mgr.spaces[&65536].entities.contains_key(&100));
    assert!(!mgr.entity_space.contains_key(&100));
}

#[test]
fn connect_entity_marks_as_player() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
    mgr.connect_entity(100);
    assert!(mgr.spaces[&65536].players.contains(&100));
}

#[test]
fn update_entity_position() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
    mgr.update_entity_position(100, [50.0, 5.0, 60.0], [0, 0, 0], [0.0; 3]);
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(entity.position, Vector3::new(50.0, 5.0, 60.0));
}

#[test]
fn aoi_detects_nearby_players() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3]).unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    let events = mgr.compute_aoi_changes();

    // Both players should see each other enter AoI
    let entered: Vec<_> = events.iter().filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. })).collect();
    assert_eq!(entered.len(), 2);
}

#[test]
fn aoi_detects_entity_leaving() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
    mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3]).unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    // First tick: both enter AoI
    let _ = mgr.compute_aoi_changes();

    // Move entity 200 far away
    mgr.update_entity_position(200, [5000.0, 0.0, 5000.0], [0, 0, 0], [0.0; 3]);

    // Second tick: entity 200 should leave AoI of entity 100
    let events = mgr.compute_aoi_changes();
    let left: Vec<_> = events.iter().filter(|e| matches!(e, CellToBaseMsg::LeftAoI { .. })).collect();
    assert_eq!(left.len(), 2); // Both should lose sight of each other
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

    let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Agnos_Library" Instanced="false" MinX="-600" MaxX="600" MinY="-600" MaxY="600" />
    <Space WorldName="Beta_Site_Evo_1" Instanced="false" MinX="-1600" MaxX="2600" MinY="-3000" MaxY="3000" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    <Space WorldName="SGC_W1" Instanced="true" MinX="-400" MaxX="400" MinY="-400" MaxY="800" />
</Spaces>"#;

    let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
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
    assert!(npc.abilities.has_ability(crate::cell::combat::NPC_DEFAULT_ABILITY));
}

#[test]
fn all_npc_entity_ids_returns_only_npcs() {
    let mut mgr = make_manager();
    // Add a player entity
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.connect_entity(1);
    // Add two NPC entities
    mgr.spawn_npc(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
    mgr.spawn_npc(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3]).unwrap();

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
        x: 10.0, y: 0.0, z: 20.0, heading: 1.5,
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
    assert_eq!(npc.stats.get(cimmeria_entity::stats::HEALTH).unwrap().max, 450);
}

#[test]
fn instanced_space_destroyed_when_last_player_leaves() {
    let mut mgr = make_manager();

    // Create a player entity in an instanced space
    let space_id = mgr.create_entity(200, "Castle_CellBlock", [5.0, 0.0, 5.0], [0.0; 3]).unwrap();
    mgr.connect_entity(200);

    // Manually add an NPC into the same instanced space (simulates what
    // spawn_instance_npcs_from_records does with a specific space_id)
    let npc_pos = Vector3::new(10.0, 0.0, 10.0);
    let mut npc = CellEntity::new(
        EntityId(100_000),
        SpaceId(space_id as i32),
        npc_pos,
    );
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
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
    mgr.connect_entity(100);

    // Destroy the player — non-instanced space should NOT be destroyed
    mgr.destroy_entity(100);

    assert!(mgr.spaces.contains_key(&65536)); // Agnos space still exists
}

#[test]
fn two_players_get_separate_instances() {
    let mut mgr = make_manager();

    let space1 = mgr.create_entity(100, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();
    let space2 = mgr.create_entity(200, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

    assert_ne!(space1, space2);
    assert!(mgr.spaces.contains_key(&space1));
    assert!(mgr.spaces.contains_key(&space2));

    // Each space has exactly one entity
    assert_eq!(mgr.spaces[&space1].entities.len(), 1);
    assert_eq!(mgr.spaces[&space2].entities.len(), 1);
}
