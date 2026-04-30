use super::*;
use super::super::space_manager::SpaceManager;

fn make_manager_with_worlds() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
</Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Castle" />
</Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    mgr
}

fn make_test_record(world_name: &str, tag: Option<&str>, class: &str) -> SpawnRecord {
    SpawnRecord {
        spawn_id: 1,
        world_name: world_name.to_string(),
        x: 10.0, y: 0.0, z: 20.0,
        heading: 1.57,
        tag: tag.map(|t| t.to_string()),
        template_id: 14,
        template_name: "Test Entity".to_string(),
        class: class.to_string(),
        static_mesh: Some("Props.TestMesh".to_string()),
        body_set: "GLB_Components.WorldObject_Small".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(5),
        alignment: Some(0),
        faction: Some(1),
        name_id: Some(7031),
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
        loot_table_id: None,
    }
}

#[test]
fn npc_ids_are_sequential() {
    let mut mgr = make_manager_with_worlds();
    let id1 = mgr.allocate_npc_id();
    let id2 = mgr.allocate_npc_id();
    assert_eq!(id1, 100_000);
    assert_eq!(id2, 100_001);
}

#[test]
fn spawn_in_unknown_world_skipped() {
    let mut mgr = SpaceManager::new(1);
    let npc_id = mgr.allocate_npc_id();
    let result = mgr.spawn_npc(npc_id, "Nonexistent", [0.0; 3], [0.0; 3]);
    assert!(result.is_err());
}

#[test]
fn class_id_mapping() {
    assert_eq!(class_id_for_class("spawnable"), 0x00);
    assert_eq!(class_id_for_class("being"), 0x01);
    assert_eq!(class_id_for_class("mob"), 0x04);
    assert_eq!(class_id_for_class("unknown"), 0x04); // fallback
}

#[test]
fn spawn_npc_from_record_sets_template_fields() {
    let mut mgr = make_manager_with_worlds();
    let record = make_test_record("Agnos", Some("TestTag"), "being");

    let npc_id = mgr.allocate_npc_id();
    let space_id = mgr.spawn_npc_from_record(npc_id, &record).unwrap();
    assert!(space_id > 0);

    let npc = mgr.get_entity(npc_id).unwrap();
    assert_eq!(npc.class_id, 0x01); // SGWBeing
    assert_eq!(npc.template_id, Some(14));
    assert_eq!(npc.tag.as_deref(), Some("TestTag"));
    assert_eq!(npc.name_id, Some(7031));
    assert_eq!(npc.faction, 1);
    assert_eq!(npc.level, 5);
    assert_eq!(npc.npc_name.as_deref(), Some("Test Entity"));
    assert!(npc.has_dynamic_properties);
}

#[test]
fn spawn_from_records_only_in_startup_spaces() {
    let mut mgr = make_manager_with_worlds();
    let records = vec![
        make_test_record("Agnos", Some("Tag1"), "mob"),
        make_test_record("Castle_CellBlock", Some("Tag2"), "being"), // instanced, not loaded
    ];

    let count = spawn_npcs_from_records(&records, &mut mgr);
    assert_eq!(count, 1); // Only Agnos (startup), not Castle_CellBlock (instanced)
}

#[test]
fn spawn_instance_npcs_filters_by_world() {
    let mut mgr = make_manager_with_worlds();
    // Create the instanced space first — returns a new space_id each time
    let space_id = mgr.find_or_create_space("Castle_CellBlock").unwrap();

    let records = vec![
        make_test_record("Castle_CellBlock", Some("Tag1"), "being"),
        make_test_record("Castle_CellBlock", Some("Tag2"), "spawnable"),
        make_test_record("Agnos", Some("Tag3"), "mob"), // wrong world
    ];

    let count = spawn_instance_npcs_from_records(&records, "Castle_CellBlock", space_id, &mut mgr);
    assert_eq!(count, 2);
}

#[test]
fn find_entity_by_tag_works() {
    let mut mgr = make_manager_with_worlds();
    let record = make_test_record("Agnos", Some("TestTag"), "being");
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc_id, &record).unwrap();

    // Use the NPC itself as the source entity -- it lives in Agnos, so the
    // search runs against that single space.
    let found = mgr.find_entity_by_tag(npc_id, "TestTag");
    assert_eq!(found, Some(npc_id));

    let not_found = mgr.find_entity_by_tag(npc_id, "NonexistentTag");
    assert_eq!(not_found, None);
}
