//! `AiState::Submit` disengagement guards (Harset H08).
//!
//! The bug shape these reproduce: `npc_ai_submit` used to wipe the NPC's
//! own `threat_list` in place, which leaves every attacker holding the
//! NPC in `threatened_mobs` forever (stuck `BSF_InCombat`, weapon stays
//! drawn, `regen_tick` permanently gated off) and leaves their auto-fire
//! loop running, so the surrendered NPC gets shot dead seconds later.
//!
//! Three stop mechanisms are pinned across these modules because they
//! run on different clocks: the AI-side handler (~2 s cadence, reached
//! via `npc_ai_tick`), the `auto_cycle_tick` target-validity gate
//! (100 ms), which is what actually closes the auto-fire kill window,
//! and the `fire_pulse` surrender floor (100 ms), which closes the
//! damage-over-time one. The pulse floor's own guards live with the
//! pulsing tests (`cell::effects::pulsing::tests`), next to the code
//! they revert-verify.
//!
//! This module holds the shared fixtures; the guards are split by what
//! they assert about:
//!
//! - [`player_scrub`] — the attacker's `threatened_mobs` /
//!   `BSF_InCombat` / re-engage behaviour.
//! - [`auto_cycle`] — the auto-fire loop, both the one-shot sweep at
//!   surrender and the per-tick validity gate.
//! - [`npc_quiescence`] — what the NPC itself ends up looking like:
//!   non-hostile, channel-free, out of cover, stopped, and facing the
//!   player it surrendered to.
//! - [`health_crossing`] — the packet's acceptance case driven end to
//!   end through the real damage seam, rather than through a hand-built
//!   `ai_state` write.

mod auto_cycle;
mod health_crossing;
mod npc_quiescence;
mod player_scrub;

use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::cell_entity::{AiState, MobMovementType};
use cimmeria_entity::stats::HEALTH;

use crate::cell::combat::{
    arm_auto_cycle, generate_threat, HOSTILE_FACTION, {BSF_AUTO_CYCLING, BSF_IN_COMBAT},
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const NPC: u32 = 200;
const PLAYER_A: u32 = 1;
const PLAYER_B: u32 = 2;
/// Ranged, 30-unit, no-ammo — keeps the auto-cycle fixtures about loop
/// semantics rather than ammo or range.
const ABILITY: i32 = 7;

fn make_mgr() -> SpaceManager {
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
fn add_player(mgr: &mut SpaceManager, id: u32, x: f32) {
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
fn add_npc(mgr: &mut SpaceManager, id: u32, x: f32) {
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
fn content_sets_submit(mgr: &mut SpaceManager, npc_id: u32) {
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Submit);
        npc.nav_path.clear();
    }
}

async fn run_ai_tick(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    crate::cell::service::npc_ai::npc_ai_tick(
        tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// Arm `player_id`'s auto-fire loop at `target_id`, the way a first
/// committed fire with `setAutoCycle(1)` already pressed leaves it.
fn arm_loop_at(mgr: &mut SpaceManager, player_id: u32, target_id: u32) {
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

fn install_ability_def(mgr: &mut SpaceManager) {
    mgr.ability_defs.insert(
        ABILITY,
        AbilityDef {
            ability_id: ABILITY,
            name: "test".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
}

/// `onStateFieldUpdate` payloads addressed to `entity_id`'s own client.
fn state_updates_for(msgs: &[CellToBaseMsg], entity_id: u32) -> Vec<u32> {
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

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}
