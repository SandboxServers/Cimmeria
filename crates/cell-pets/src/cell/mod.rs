//! The pet cell methods, under the `cell::` path they had in
//! `cimmeria-cell-methods`.

pub mod cell_methods;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_combat::cell::{abilities, combat};
pub(crate) use cimmeria_cell_content::cell::content;
pub(crate) use cimmeria_cell_world::cell::{pets, space_manager};
pub(crate) use cimmeria_wire::cell::{client_methods, messages};
