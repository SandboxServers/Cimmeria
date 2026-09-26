//! Ring transporter system, at its old path.
//!
//! The state machine, the region loader, the wire payloads and helpers, and
//! the departing-player hook are in `cimmeria-cell-world` (wave C1 of
//! `docs/architecture/services-crate-split.md`). This module re-exports all of
//! it, so every `crate::cell::ring_transport::X` path compiles unchanged, and
//! declares the two pieces that fire content chains and so sit above the world
//! crate (§2G):
//!
//! - `dispatch` — turns [`Effect`] values into `CellToBaseMsg` sends and
//!   spatial-grid mutations, including `FireTeleportIn`.
//! - `runtime` — public entry points (`handle_interact`,
//!   `handle_select_destination`, `handle_region_trigger`,
//!   `run_tick_with_engine`), beside the world crate's `forget_player`.

pub use cimmeria_cell_world::cell::ring_transport::*;

mod dispatch;
// Public because it shadows the world crate's `runtime`, which the glob above
// re-exports publicly; this one re-exports that one beside the entry points.
pub mod runtime;

#[cfg(test)]
mod tests;

pub use runtime::{
    handle_interact, handle_region_trigger, handle_remote_player_loaded, handle_select_destination,
    run_tick_with_engine,
};
