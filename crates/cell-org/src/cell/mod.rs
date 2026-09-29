//! The organization handlers, under the `cell::` paths they had in
//! `cimmeria-cell-methods` (`cell_methods::organization` there,
//! `organization` here).

pub mod organization;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_world::cell::{org_creation, space_manager, squad};
pub(crate) use cimmeria_wire::cell::messages;
