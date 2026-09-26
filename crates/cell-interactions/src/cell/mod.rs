//! The cell's player interactions, under the `cell::` paths they had in
//! `cimmeria-services`.
//!
//! `cimmeria-services`' `cell` module re-exports each of these modules at the
//! same path, beside the cell systems that sit above this crate (the
//! cell-method handlers, the console and the service loop).

pub mod gate_travel;
pub mod interactions;
pub mod mail;
pub mod respawn;
pub mod space_transfer;
pub mod trade;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::cell::spawner;
pub(crate) use cimmeria_cell_combat::cell::{abilities, combat};
pub(crate) use cimmeria_cell_content::cell::{content, missions, ring_transport};
pub(crate) use cimmeria_cell_world::cell::{arrival, playtest_friction, space_manager};
pub(crate) use cimmeria_wire::cell::{client_methods, kismet, messages};
