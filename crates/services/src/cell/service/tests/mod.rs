//! CellService tick + handler tests — exercise reload-completion, bandolier
//! seeding, logout flushes, disconnect-flush ordering, and NPC AI state
//! transitions without spinning up a full ChainEngine.
//!
//! Split into per-concern submodules (split done when the original
//! `tests.rs` crossed the 700-line hard cap from `CLAUDE.md`):
//!
//! - [`reload`] — `reload_completion_tick` refill / wire-frame coverage.
//! - [`bandolier`] — `InitPlayerState` seeding, `DestroyEntity` /
//!   `DisconnectEntity` flush ordering.
//! - [`npc_ai`] — `npc_ai_tick` state-machine transitions
//!   (Fighting → Idle / Leashing, dead-target prune, stationary, leash snap).
//! - [`regen`] — `regen_tick` out-of-combat HP/focus regen, threat-set /
//!   dead gating, full-pool skip and zero-regen floor cases.

use crate::cell::space_manager::SpaceManager;

mod bandolier;
mod npc_ai;
mod npc_ai_auto_aggro;
mod npc_ai_cover;
mod npc_ai_follow;
mod npc_ai_investigate;
mod npc_ai_patrol;
mod npc_ai_phase7;
mod npc_ai_wander;
mod regen;
mod reload;

/// Castle_CellBlock instanced fixture used by reload + bandolier tests.
/// NPC AI tests use their own non-instanced fixture in [`npc_ai`] so
/// witnesses and target entities live in the same space.
pub(super) fn make_test_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr
}
