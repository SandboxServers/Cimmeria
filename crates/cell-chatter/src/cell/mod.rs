//! The chatter tick, under the `cell::` paths the other cell plugins use.

pub mod chatter;

// Lower crates, at the `cell::` paths the code names them by.
pub(crate) use cimmeria_cell_world::cell::space_manager;
pub(crate) use cimmeria_wire::cell::messages;
