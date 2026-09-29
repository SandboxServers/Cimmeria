//! The effect scripts, under the `cell::` paths they had in
//! `cimmeria-cell-world`.

pub mod effects;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::cell::spawner;
pub(crate) use cimmeria_cell_world::cell::{pets, space_manager};
