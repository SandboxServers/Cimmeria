//! SGWCombatant interface exposed CellMethods (indices 5–7).
//!
//! The handlers are in `cimmeria_services::cell::cell_methods::combatant`,
//! which re-exports these constants.

/// Set crouched state.
pub const SET_CROUCHED: u16 = 5;
/// Toggle heal debug overlay.
pub const TOGGLE_HEAL_DEBUG: u16 = 6;
/// Request holster/unholster weapon.
pub const REQUEST_HOLSTER_WEAPON: u16 = 7;
