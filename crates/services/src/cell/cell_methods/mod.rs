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
// `cell::console::gm`: the GM console calls them, and the console and these
// cell methods are sibling crates in the services split (§2H). A production
// re-export here would be a methods -> console edge, so the callers name the
// new path. Only a test outside this tree still names the old one.
#[cfg(test)]
pub(crate) use super::console::gm;
pub mod inventory;
pub mod mail;
pub mod minigame;
pub mod missionary;
pub mod organization;
pub mod player;
