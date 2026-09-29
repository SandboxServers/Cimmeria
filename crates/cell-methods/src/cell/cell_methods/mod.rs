//! Client→server exposed CellMethod indices for SGWPlayer.
//!
//! These constants define the flattened exposed CellMethod indices used in
//! cell method call packets sent FROM the client TO the server. They are a
//! DIFFERENT index space from the client method indices in `client_methods/`.
//!
//! BigWorld flattening order for exposed cell methods follows the same rule
//! as client methods: Implements interfaces first, then own methods, at each
//! level of the inheritance chain. Only methods marked `<Exposed/>` are counted.
//!
//! See `docs/protocol/cell-method-dispatch-table.md` for the complete
//! 109-method table.

pub mod ability_manager;
pub mod being;
pub mod black_market;
pub mod combatant;
pub mod contact_list;
pub mod gate_travel;
// SGWGmPlayer own CellMethods (flattened index 109+, GM-gated upstream) are
// `cell::console::gm` in `cimmeria-cell-console`: the GM console calls them,
// and the console and these cell methods are sibling crates in the services
// split (§2H), so this module does not re-export them.
pub mod inventory;
pub mod mail;
pub mod minigame;
/// The `abandonMission` path of the Harset H54 `mission_abandoned` guards
/// (wave C5a moved them here from `cimmeria-services`). Test-only.
#[cfg(test)]
mod mission_abandoned_tests;
pub mod missionary;
// The OrganizationMember interface (8-19) and `onOrganizationCreation` (94)
// are the org plugin's (`cimmeria-cell-org`, #962 step 3); the half the
// base-message handler and the console call is
// `cimmeria_cell_interactions::cell::organization`.
pub mod player;
