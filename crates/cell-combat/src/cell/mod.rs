//! The cell's combat, under the `cell::` paths it had in `cimmeria-services`.
//!
//! `cimmeria-services`' `cell` module re-exports each of these modules at the
//! same path, beside the cell systems that sit above this crate (content,
//! interactions, the cell-method handlers, the console and the service loop).

pub mod abilities;
pub mod cell_methods;
pub mod combat;
pub mod effects;
pub mod service;

/// The Harset spawner guards, which drive the NPC AI's ability selector.
/// Test-only; in `cimmeria-services` until wave C6 of the services crate
/// split.
#[cfg(test)]
mod spawner_tests;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::cell::spawner;
pub(crate) use cimmeria_cell_world::cell::{
    content_events, cover, dispatch, playtest_friction, space_manager,
};
pub(crate) use cimmeria_wire::cell::{client_methods, messages, player_journal};
