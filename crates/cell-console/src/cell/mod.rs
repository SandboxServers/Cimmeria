//! The cell's GM surfaces, under the `cell::` paths they had in
//! `cimmeria-services`.
//!
//! `cimmeria-services`' `cell` module re-exports `console` at the same path,
//! and `console::chat` at its old `cell::chat` path, beside the cell systems
//! that sit above this crate (the service loop and the cell-method router).

pub mod console;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_combat::cell::{abilities, combat};
pub(crate) use cimmeria_cell_content::cell::{content, missions};
pub(crate) use cimmeria_cell_interactions::cell::{
    gate_travel, interactions, respawn, space_transfer,
};
// The GM gate is world's `cell::dispatch`; the router beside it in
// `cimmeria-services` sits above this crate.
pub(crate) use cimmeria_cell_world::cell::{dispatch, playtest_friction, space_manager};
pub(crate) use cimmeria_wire::cell::{client_methods, messages, player_journal};
// The spawner's records and catalog entries, which only the tests build.
#[cfg(test)]
pub(crate) use cimmeria_cell_catalog::cell::spawner;
