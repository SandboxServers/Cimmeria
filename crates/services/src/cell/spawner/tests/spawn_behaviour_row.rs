//! NA44 (handoff §25 "Spawn"): the `spawner.npc_behaviour` row carries the
//! common `world` / `space_id` fields and what the NPC fights with, so a
//! guard firing the wrong animation is diagnosable from its spawn row.
//!
//! Filter prefix: `spawn_behaviour_row_`.

use cimmeria_entity::abilities::AbilityDef;
use tracing::Level;

use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{spawn_npcs_from_records, SpawnRecord};
use crate::test_support::LogCapture;

fn agnos() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

fn ability(id: i32, event_set_id: Option<i32>) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: format!("ability {id}"),
        cooldown: 0.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: true,
        min_range: 0,
        max_range: 0,
        target_type_id: 0,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id,
        velocity: 0.0,
    }
}

/// An SMG guard with a two-ability set: 559 animates through event set 15,
/// 579's definition carries no event set.
fn smg_guard() -> SpawnRecord {
    SpawnRecord {
        spawn_id: 7,
        world_name: "Agnos".to_string(),
        x: 10.0,
        y: 0.0,
        z: 10.0,
        heading: 0.0,
        tag: Some("Guard1".to_string()),
        template_id: 24,
        template_name: "NID Guard".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: Some(vec![
            "AR_H_Ablative.AR_HM_AT3_AT300".to_string(),
            "WP-Human.WP_SMG_1A".to_string(),
        ]),
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(1),
        alignment: Some(0),
        faction: Some(10),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![579, 559],
        respawn_secs: None,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
    }
}

/// Revert proof: remove any of the five fields from `log_spawn_behaviour`
/// and the matching assertion fails.
#[test]
fn spawn_behaviour_row_names_the_world_the_abilities_and_the_weapon() {
    let mut mgr = agnos();
    mgr.ability_defs.insert(559, ability(559, Some(15)));
    mgr.ability_defs.insert(579, ability(579, None));
    let logs = LogCapture::install();

    assert_eq!(spawn_npcs_from_records(&[smg_guard()], &mut mgr), 1);

    let row = logs
        .find_message(Level::DEBUG, "resolved behaviour")
        .expect("spawner.npc_behaviour row");
    assert_eq!(row.target, "spawner.npc_behaviour");
    assert!(row.has_field("world", "Agnos"), "{:?}", row.fields);
    let space_id = mgr.find_or_create_space("Agnos").unwrap();
    assert!(row.has_field("space_id", &space_id.to_string()));
    assert!(row.has_field("ability_ids", "[559, 579]"), "sorted");
    assert!(
        row.has_field("event_set_ids", "[15, 0]"),
        "aligned with ability_ids; 0 = cannot animate"
    );
    assert!(row.has_field("weapon_visual", "WP-Human.WP_SMG_1A"));
}

/// The startup spaces spawn their NPCs inside `CellService::start`, which
/// used to load the ability definitions only afterwards, so every startup
/// NPC's `event_set_ids` read 0 (Castle's guards included). The start-up
/// sequence needs a database and a message loop, so this pins the order in
/// the source instead. Revert proof: move the `load_ability_defs` block back
/// below `spawn_npcs_from_records` and this fails.
#[test]
fn spawn_behaviour_row_ability_defs_load_before_the_startup_spawn() {
    let src = include_str!("../../service/startup.rs");
    let defs = src
        .find("spawner::load_ability_defs(")
        .expect("startup loads ability defs");
    let spawn = src
        .find("spawner::spawn_npcs_from_records(")
        .expect("startup spawns NPCs");
    assert!(
        defs < spawn,
        "ability defs must be loaded before the startup NPC spawn"
    );
}
