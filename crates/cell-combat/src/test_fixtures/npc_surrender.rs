//! The H08 surrender guards' fixtures: a meshless Castle space, connected
//! players, hostile NPCs, the content-style `Submit` write, the player
//! auto-fire loop, and a real AI tick.
//!
//! Shared by `cell::service::npc_ai::lifecycle::tests` here and by the two
//! surrender test files that drive the service loop's auto-cycle tick in
//! `cimmeria-services` (`cell::service::npc_ai::lifecycle_tests`).

use tokio::sync::mpsc;

use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;

use crate::cell::combat::{arm_auto_cycle, HOSTILE_FACTION};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const NPC: u32 = 200;
pub const PLAYER_A: u32 = 1;
pub const PLAYER_B: u32 = 2;
/// Ranged, 30-unit, no-ammo — keeps the auto-cycle fixtures about loop
/// semantics rather than ammo or range.
pub const ABILITY: i32 = 7;

pub fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

/// Connected player at `x`. `connect_entity` is required: the auto-cycle
/// sweep iterates `all_player_entity_ids()`, which only returns players
/// in the space's `players` set.
pub fn add_player(mgr: &mut SpaceManager, id: u32, x: f32) {
    mgr.create_entity(id, "Castle", [x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(id) {
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
        p.weapon_holstered = false;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr.connect_entity(id);
}

/// Hostile NPC at `x`. The hostile faction matters because the
/// idle-auto-aggro scan skips same-faction players, and players default
/// to faction 0.
pub fn add_npc(mgr: &mut SpaceManager, id: u32, x: f32) {
    mgr.spawn_npc(id, "Castle", [x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(id) {
        npc.faction = HOSTILE_FACTION;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
}

/// Put `npc_id` into Submit exactly the way the content action does —
/// a bare `ai_state` write plus a nav clear, no cleanup of its own.
pub fn content_sets_submit(mgr: &mut SpaceManager, npc_id: u32) {
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Submit);
        npc.nav_path.clear();
    }
}

pub async fn run_ai_tick(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    crate::cell::service::npc_ai::npc_ai_tick(
        tx,
        mgr,
        &cimmeria_cell_world::test_fixtures::NoContentEvents,
    )
    .await;
}

/// Arm `player_id`'s auto-fire loop at `target_id`, the way a first
/// committed fire with `setAutoCycle(1)` already pressed leaves it.
pub fn arm_loop_at(mgr: &mut SpaceManager, player_id: u32, target_id: u32) {
    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.abilities.add_ability(ABILITY);
        p.abilities.auto_cycle = true;
        p.current_target_id = Some(target_id as i32);
    }
    let armed = arm_auto_cycle(mgr, player_id, ABILITY, target_id as i32);
    assert!(
        armed.is_some(),
        "fixture invariant: the loop must actually arm"
    );
}

/// The loop's ability, with the shared no-op mechanic effect so a player's
/// cast of it passes the AB-12 launch gate.
pub fn install_ability_def(mgr: &mut SpaceManager) {
    cimmeria_cell_world::test_fixtures::seed_mechanic_effect(mgr);
    mgr.ability_defs.insert(
        ABILITY,
        AbilityDef {
            ability_id: ABILITY,
            name: "test".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 0,
            effect_ids: vec![cimmeria_cell_world::test_fixtures::MECHANIC_FIXTURE_EFFECT],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
            type_id: Default::default(),
            passive: false,
        },
    );
}

/// `onStateFieldUpdate` payloads addressed to `entity_id`'s own client.
pub fn state_updates_for(msgs: &[CellToBaseMsg], entity_id: u32) -> Vec<u32> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: eid,
                method_index,
                args,
            } if *eid == entity_id
                && *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE =>
            {
                Some(u32::from_le_bytes(args[..4].try_into().unwrap()))
            }
            _ => None,
        })
        .collect()
}

pub fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}
