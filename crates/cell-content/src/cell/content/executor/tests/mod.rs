//! Tests for `executor::execute_actions` — exercise the per-action-family
//! handlers via the public match dispatch.
//!
//! Split into per-theme submodules when the original `tests.rs` crossed
//! the 700-line hard cap from `CLAUDE.md`:
//!
//! - [`ability_granter`]  — `Action::GmAbilityBulk` (DA-02): the GM gate,
//!   the tree grant, the cooldown clear and the first-click lines.
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
//! - [`mail`]             — `Action::SendSystemMail` (SS-U3): the firings
//!   that send nothing, with their `reason=` rows.
//! - [`once_gate`]        — `content_triggers.once`: fire once per entity.
//! - [`open_loot`]        — `Action::OpenLoot`: per-looter rolls on a live chest.
//! - [`pets`]             — pets PT-02 at the content transport call sites.
//! - [`stargate`]         — `Action::GrantStargateAddress`: the three legs
//!   of a grant and client method 66's byte layout. The refused-then-
//!   accepted dial the packet exists for drives the gate dial, so it is in
//!   `cimmeria-services`' `cell::content_tests::stargate_grant_dial`.
//!
//! Shared executor scope (`execute_actions`, `Action`, `ResolvedActions`,
//! `SpaceManager`, `mpsc`, `CellToBaseMsg`) is re-exported below so each
//! submodule's `use super::*` resolves it. `make_space_mgr` is shared by
//! every submodule; theme-specific fixtures live with their tests.

// Re-export the parent (executor) scope so submodules can `use super::*`.
pub(super) use super::{
    deferred_content_action_tick, execute_actions, CellToBaseMsg, SpaceManager,
};
pub(super) use cimmeria_content_engine::actions::Action;
pub(super) use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
pub(super) use tokio::sync::mpsc;

mod ability_granter;
mod deferred;
mod effects;
mod inventory_counter;
mod mail;
mod mission;
mod negative_logging;
mod npc_state;
mod once_gate;
mod open_loot;
mod pets;
mod stargate;
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
