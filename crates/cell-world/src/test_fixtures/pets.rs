//! Pet fixtures (issue #570): a pet-shaped template in the startup cache and
//! a ready player owner, so every pet packet's tests start from the same
//! world.

use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::SpawnRecord;

/// Template id the pet fixtures cache. In the pets range (350-369), like the
/// seeded Jaffa Soldier (350).
pub const PET_FIXTURE_TEMPLATE_ID: i32 = 350;
/// Ability set the fixture template carries: the pet bar.
pub const PET_FIXTURE_ABILITIES: [i32; 2] = [592, 1652];

/// A pet template shaped like the PT-S Jaffa row: authored hostile at
/// level 3 with loot, a respawn timer, a patrol and a wander radius, so a
/// test can see the spawn path strip each of them.
pub fn pet_template_record(template_id: i32) -> SpawnRecord {
    SpawnRecord {
        spawn_id: -1,
        world_name: String::new(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: Some("template_tag".to_string()),
        template_id,
        template_name: "Jaffa Soldier (pet)".to_string(),
        class: "pet".to_string(),
        static_mesh: None,
        body_set: "BS_JaffaMale.BS_JaffaMale".to_string(),
        components: Some(vec!["Jaffa_Armor".to_string()]),
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(3),
        alignment: Some(0),
        faction: Some(10),
        name_id: Some(8087),
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        loot_table_id: Some(9),
        is_stationary: false,
        ability_ids: PET_FIXTURE_ABILITIES.to_vec(),
        respawn_secs: Some(30),
        patrol_path: vec![cimmeria_common::Vector3::new(1.0, 0.0, 1.0)],
        patrol_point_delay_secs: 2.0,
        wander_radius: 8.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
    }
}

/// Cache [`pet_template_record`] under `template_id` on `mgr`.
pub fn seed_pet_template(mgr: &mut SpaceManager, template_id: i32) {
    mgr.spawn_templates
        .insert(template_id, pet_template_record(template_id));
}

/// Create a connected, introducible player `entity_id` in `world` at
/// `position` with `level`: what a pet owner looks like once
/// `InitPlayerState` has run.
pub fn add_pet_owner(
    mgr: &mut SpaceManager,
    entity_id: u32,
    world: &str,
    position: [f32; 3],
    level: u32,
) {
    mgr.create_entity(entity_id, world, position, [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    if let Some(e) = mgr.get_entity_mut(entity_id) {
        e.account_id = Some(entity_id);
        e.archetype_id = Some(1);
        e.level = level;
    }
}
