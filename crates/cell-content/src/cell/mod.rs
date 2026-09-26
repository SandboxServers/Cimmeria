//! The cell's content layer, under the `cell::` paths it had in
//! `cimmeria-services`.
//!
//! `cimmeria-services`' `cell` module re-exports each of these modules at the
//! same path, beside the cell systems that sit above this crate (the rest of
//! interactions, gate travel and the space transfer in
//! `cimmeria-cell-interactions`, the cell-method handlers, the console and the
//! service loop).

pub mod content;
pub mod interactions;
pub mod missions;
pub mod ring_transport;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::cell::spawner;
pub(crate) use cimmeria_cell_combat::cell::{abilities, combat, effects};
pub(crate) use cimmeria_cell_world::cell::{
    arrival, content_events, cover, playtest_friction, playtest_friction_watch, space_manager,
};
pub(crate) use cimmeria_wire::cell::{client_methods, messages, player_journal};

// The combat crate's `cell::service`, for the tests' NPC AI hooks
// (`force_ai_state`, `npc_ai_tick_for_test`, `npc_ai_follow_for_test`).
// Production code names the NPC AI's state primitives in the world crate.
#[cfg(test)]
pub(crate) use cimmeria_cell_combat::cell::service;
