//! Behaviour tests for the inventory cell-method dispatch surface,
//! grouped by tested message: slot-swap, ammo-change, active-slot
//! change, and move-item.

use cimmeria_entity::abilities::AbilityDef;

use crate::cell::space_manager::SpaceManager;

mod active_slot_change;
mod ammo_change;
mod move_item;
mod slot_swap;

fn make_test_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Register a no-warmup, ranged ability with `required_ammo = 1` and no
/// event-set (silences onSequence noise during tests).
fn register_test_fire_ability(mgr: &mut SpaceManager, ability_id: i32) {
    mgr.ability_defs.insert(
        ability_id,
        AbilityDef {
            ability_id,
            name: format!("test_fire_{ability_id}"),
            cooldown: 0.001, // very short so back-to-back fires aren't gated
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );
}
