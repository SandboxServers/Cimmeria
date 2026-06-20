//! Tests for `handle_use_ability`, split by theme.
//!
//! Shared fixtures live here so every themed submodule can reach them
//! via `use super::*`. The submodules reach `use_ability`'s public
//! surface (`handle_use_ability`, etc.) through this module's
//! `use super::super::*` re-export below.

use super::super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::abilities::AbilityDef;
use tokio::sync::mpsc;

mod auto_cycle;
mod gating;
mod holster_queue;
mod target_validity;
mod weapon_grant;

fn make_ability(id: i32, required_ammo: i32, max_range: i32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "test".to_string(),
        cooldown: 0.5,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0,
        max_range,
        target_type_id: 0,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo,
        event_set_id: None,
        velocity: 0.0,
    }
}

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr
}

fn make_player(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.create_entity(id, "Castle_CellBlock", pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(id) {
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
    }
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}
