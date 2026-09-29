//! The duel handlers, under the `cell::` paths they had in
//! `cimmeria-cell-world`.

pub mod duel;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_world::cell::space_manager;
pub(crate) use cimmeria_wire::cell::messages;

#[cfg(test)]
pub(crate) use cimmeria_cell_world::cell::combat;
