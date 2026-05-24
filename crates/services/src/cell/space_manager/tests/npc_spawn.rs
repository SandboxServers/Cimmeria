//! NPC spawn surface: class id, default ability, NPC-only iteration,
//! world lookup, and template-record application.

use cimmeria_common::Vector3;

use super::make_manager;

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
        ability_ids: vec![],
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
