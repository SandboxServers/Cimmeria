//! The client-callable cell methods, under the `cell::` path they had in
//! `cimmeria-services`.
//!
//! `cimmeria-services`' `cell` module re-exports `cell_methods` at the same
//! path, beside the cell systems that sit above this crate (the dispatch
//! router and the service loop) and the GM console beside it.

pub mod cell_methods;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::cell::spawner;
pub(crate) use cimmeria_cell_combat::cell::{abilities, combat};
pub(crate) use cimmeria_cell_content::cell::{content, missions, ring_transport};
pub(crate) use cimmeria_cell_interactions::cell::{
    gate_travel, interactions, mail, respawn, trade,
};
pub(crate) use cimmeria_cell_world::cell::{
    duel, org_creation, pets, playtest_friction, space_manager, squad,
};
pub(crate) use cimmeria_wire::cell::{client_methods, messages, player_journal};
