//! The cell-method handlers the combat path drives, under the
//! `cell::cell_methods` paths they had in `cimmeria-services`: the bandolier
//! slot operations (`inventory::bandolier`), and the reload and item-sequence
//! handlers (`player::world`).
//!
//! The rest of `cell_methods` (the per-interface dispatchers and every other
//! handler) sits above this crate, in `cimmeria-services`, whose modules
//! re-export these at the same paths.

pub mod inventory;
pub mod player;
