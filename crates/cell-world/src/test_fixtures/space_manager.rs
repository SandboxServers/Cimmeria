//! The standard `SpaceManager` test fixtures.

use crate::cell::space_manager::SpaceManager;

/// Standard test space setup: SpaceManager with a single Agnos space.
///
/// The same ~5-line block was duplicated across dispatch, interaction,
/// and vendor test modules. Extracted here so every cell test shares
/// the same default world geometry.
pub fn make_space_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    mgr
}

/// Same as [`make_space_manager`], but also creates a player entity at
/// the origin so tests that need an avatar don't repeat the entity
/// creation boilerplate.
pub fn make_space_manager_with_player(entity_id: u32) -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(entity_id, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr
}

/// Register minimal `AbilityDef`s so the trainability predicate's
/// "ability exists" gate passes for these ids.
pub fn seed_ability_defs(mgr: &mut SpaceManager, ability_ids: &[i32]) {
    for &ability_id in ability_ids {
        mgr.ability_defs.insert(
            ability_id,
            cimmeria_entity::abilities::AbilityDef {
                ability_id,
                name: format!("TestAbility{ability_id}"),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: false,
                min_range: 0.0,
                max_range: 0.0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
                type_id: Default::default(),
            },
        );
    }
}
