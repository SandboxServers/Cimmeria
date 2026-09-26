//! SGWAbilityManager interface exposed CellMethods (indices 2–4).
//!
//! The handlers are in `cimmeria_services::cell::cell_methods::ability_manager`,
//! which re-exports these constants.

/// Toggle combat debug overlay.
pub const TOGGLE_COMBAT_DEBUG: u16 = 2;
/// Toggle verbose combat debug logging.
pub const TOGGLE_COMBAT_VERBOSE_DEBUG: u16 = 3;
/// Respond to a confirmation prompt.
pub const CONFIRMATION_RESPONSE: u16 = 4;
