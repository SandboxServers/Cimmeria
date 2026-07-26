//! Tests for `executor::execute_actions` — exercise the per-action-family
//! handlers via the public match dispatch.
//!
//! Split into per-theme submodules when the original `tests.rs` crossed
//! the 700-line hard cap from `CLAUDE.md`:
//!
//! - [`effects`]          — `Action::LaunchAbility` / `Action::ApplyEffect`
//!   (server-initiated effect application, target resolution).
//! - [`stats`]            — `Action::ChangeStat` (heal / clamp / damage /
//!   set-to-max / ammo-stat skip).
//! - [`inventory_counter`] — `Action::RemoveItem`, increment / reset counter.
//! - [`teleport`]         — `Action::CrossWorldTeleport` gate-travel sends.
//! - [`mission`]          — accept / complete mission, completion-event gate.
//! - [`npc_state`]        — set aggression, generate threat, NPC POI /
//!   follow-target / AI-state actions.
//! - [`negative_logging`] — cell→base send-failure WARN guards.
//!
//! Shared executor scope (`execute_actions`, `Action`, `ResolvedActions`,
//! `SpaceManager`, `mpsc`, `CellToBaseMsg`) is re-exported below so each
//! submodule's `use super::*` resolves it. `make_space_mgr` is shared by
//! every submodule; theme-specific fixtures live with their tests.

// Re-export the parent (executor) scope so submodules can `use super::*`.
pub(super) use super::{execute_actions, CellToBaseMsg, SpaceManager};
pub(super) use cimmeria_content_engine::actions::Action;
pub(super) use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
pub(super) use tokio::sync::mpsc;

mod effects;
mod inventory_counter;
mod mission;
mod negative_logging;
mod npc_state;
mod stats;
mod teleport;

/// Non-instanced "Agnos" startup fixture shared by every executor test.
pub(super) fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}
