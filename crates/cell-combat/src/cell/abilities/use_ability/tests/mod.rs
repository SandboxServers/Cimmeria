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
mod beneficial;
mod beneficial_live_db;
mod cast_correlation;
mod combat_debug;
mod content_events;
mod duel_end;
mod duel_end_cc;
mod duel_gate;
mod duel_nonlethal;
mod effect_routing;
mod effect_routing_live_db;
mod fire_los;
mod gating;
mod holster_queue;
mod incapacitated;
mod interrupt_effect;
mod launch_timer_rows;
mod min_range;
mod no_mechanics;
mod no_mechanics_live_db;
mod npc_timer_routing;
mod passive_cast;
mod pet_kill_credit;
mod range_units;
mod range_units_live_db;
mod registered_pet_kill_credit;
mod sequence;
mod sequence_phases;
mod shield_full;
mod silent_paths;
mod summon;
mod summon_live_db;
mod summon_logs;
mod summon_roster_live_db;
mod summoned_pet_kill_credit;
mod support_shot;
mod target_validity;
mod timed_buffs;
mod toggles;
mod warmup;
mod warmup_interrupt;
mod weapon_grant;
mod weapon_range;

/// Every [`make_ability`] fixture carries the shared no-op mechanic effect,
/// so it passes the AB-12 launch gate (`no_mechanics`) and behaves as the
/// effectless fixtures did before the gate. [`make_mgr`] seeds it; a test
/// that builds its own manager calls [`seed_fixture_effect`].
const FIXTURE_EFFECT: i32 = crate::test_support::MECHANIC_FIXTURE_EFFECT;

fn make_ability(id: i32, required_ammo: i32, max_range: i32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "test".to_string(),
        cooldown: 0.5,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0.0,
        max_range: max_range as f32,
        target_type_id: 0,
        effect_ids: vec![FIXTURE_EFFECT],
        moniker_ids: vec![],
        required_ammo,
        event_set_id: None,
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    }
}

/// One shared Castle_CellBlock space. Non-instanced on purpose: for an
/// instanced world every `create_entity` opens a fresh space, which put
/// each test's caster and target in different instances (#906).
fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    seed_fixture_effect(&mut mgr);
    mgr
}

/// Seed [`FIXTURE_EFFECT`] (and its no-op script) on a manager a test built
/// itself, so its [`make_ability`] fixtures have their mechanic.
fn seed_fixture_effect(mgr: &mut SpaceManager) {
    crate::test_support::seed_mechanic_effect(mgr);
}

fn make_player(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.create_entity(id, "Castle_CellBlock", pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(id) {
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
        p.account_id = Some(900 + id);
    }
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// Fire `def` from a player at the origin at a hostile NPC `distance` metres
/// along +X. Returns whether the cast drew the
/// `CONDITION_FEEDBACK_OutsideWeaponRange` (42) refusal.
async fn fire_at_hostile(def: &AbilityDef, distance: f32) -> bool {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    mgr.create_entity(2, "Castle_CellBlock", [distance, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(2) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(def.ability_id);
    }
    mgr.ability_defs.insert(def.ability_id, def.clone());
    let (tx, mut rx) = mpsc::channel(64);

    handle_use_ability(1, def.ability_id, 2, &tx, &mut mgr).await;

    drain(&mut rx).iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } if *method_index == method_idx::ON_ERROR_CODE && args.len() == 7 => {
            u16::from_le_bytes([args[5], args[6]]) == 42
        }
        _ => false,
    })
}
