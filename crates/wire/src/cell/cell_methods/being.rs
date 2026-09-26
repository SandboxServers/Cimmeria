//! SGWBeing interface exposed CellMethods (indices 0–1).
//!
//! The handlers are in `cimmeria_services::cell::cell_methods::being`,
//! which re-exports these constants.

/// Set current target entity.
pub const SET_TARGET_ID: u16 = 0;
/// Set movement type (walk/run/sprint).
pub const SET_MOVEMENT_TYPE: u16 = 1;
