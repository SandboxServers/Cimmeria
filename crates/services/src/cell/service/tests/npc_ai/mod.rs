//! `npc_ai_tick` state-machine transitions and combat-decision coverage.
//!
//! Split into per-concern submodules when the original `npc_ai.rs`
//! crossed the 700-line hard cap from `CLAUDE.md`:
//!
//! - [`state_machine`] — Fighting → Idle / Leashing, dead-target prune,
//!   stationary no-pathfind, leash snap-to-spawn / heal / cooldown clear,
//!   top-threat selection, NaN-threat safety, leash witness fan-out.
//! - [`aggression`]    — Idle-NPC auto-aggro via the `aggression` field
//!   (opposing-faction aggro, no-aggro defaults, same-faction skip).
//! - [`selector`]      — `choose_npc_ability` three-bucket selection
//!   (cooldown skip, all-cooling, empty-bucket fallback, ammo-bearing).
//! - [`ability_range`] — per-ability min/max range gating, max_range=0
//!   fallback, min-range backup waypoint, launch-failure retry schedule
//!   + sweep, missing-def fallback, stationary no-backoff.
//!
//! Uses a non-instanced `Castle` fixture rather than the parent
//! `make_test_space_mgr` (Castle_CellBlock, instanced) so the NPC and
//! its threat targets co-locate in the same space — otherwise
//! `has_line_of_sight` falls back to true and `find_path` to None,
//! masking what the in-range / LOS branches of `npc_ai_fight` actually do.
//!
//! `make_aggression_fixture` is `pub(super)` so the sibling
//! `npc_ai_auto_aggro` module (a peer under `service::tests`) keeps
//! resolving it as `super::npc_ai::make_aggression_fixture`.

use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;

mod ability_range;
mod aggression;
mod selector;
mod state_machine;

/// Build a non-instanced "Castle" space and seed an NPC at id=200 in
/// AiState::Fighting with the given spawn position. Returns the
/// SpaceManager. Caller layers in the threat list and ability defs.
pub(super) fn make_ai_fixture(npc_spawn: [f32; 3], npc_pos: [f32; 3]) -> SpaceManager {
    use cimmeria_common::Vector3;
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", npc_pos, [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04; // SGWMob — required for all_npc_entity_ids()
        npc.ai_state = AiState::Fighting;
        npc.spawn_position = Some(Vector3::new(npc_spawn[0], npc_spawn[1], npc_spawn[2]));
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr
}

/// Castle test space + one NPC at the origin and one player. NPC defaults
/// to `class_id = 0x04` so `npc_ai_tick` sees it; the caller flips
/// faction / aggression as needed. The NPC's default ability bucket is
/// left intact (Pistol Shot 592) so the fight path doesn't wedge on an
/// empty selector.
///
/// Visibility is `pub(super)` so the sibling `npc_ai_auto_aggro`
/// module (split out when this file crossed the 700-line cap) can
/// reuse the same setup.
pub(super) fn make_aggression_fixture(
    npc_id: u32,
    npc_faction: u8,
    player_id: u32,
    player_pos: [f32; 3],
) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.spawn_npc(npc_id, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.faction = npc_faction;
        npc.ai_state = AiState::Idle;
    }
    mgr.create_entity(player_id, "Castle", player_pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.is_player = true;
        p.player_id = Some(player_id as i32);
        p.faction = 0;
    }
    mgr.connect_entity(player_id);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// Seed an `AbilityDef` for `NPC_DEFAULT_ABILITY` with explicit min/max
/// range. Cooldown stays at the same 1.0s used by
/// `selector_picks_ammo_bearing_ability_for_npc` so a future cooldown
/// bump doesn't accidentally start gating these tests.
pub(super) fn seed_default_ability(mgr: &mut SpaceManager, min_range: i32, max_range: i32) {
    use cimmeria_entity::abilities::AbilityDef;
    mgr.ability_defs.insert(
        crate::cell::combat::NPC_DEFAULT_ABILITY,
        AbilityDef {
            ability_id: crate::cell::combat::NPC_DEFAULT_ABILITY,
            name: "Test Ability".to_string(),
            cooldown: 1.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range,
            max_range,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
}

/// Spawn an in-range threat player with full HP at the given position
/// and seed the NPC's threat_list. Mirrors the pattern from the
/// existing leash / dead-target tests so the fight tick runs.
pub(super) fn seed_target_with_threat(
    mgr: &mut SpaceManager,
    npc_id: u32,
    target_id: u32,
    target_pos: [f32; 3],
) {
    mgr.create_entity(target_id, "Castle", target_pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(target_id) {
        p.is_player = true;
        p.player_id = Some(target_id as i32);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.threat_list.insert(target_id, 1.0);
    }
}
