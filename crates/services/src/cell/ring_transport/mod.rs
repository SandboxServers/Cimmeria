//! Ring transporter system.
//!
//! Cross-region (and cross-world) teleportation rings. Players step onto a ring
//! pad (a generic point-set region) and select a destination from a list; the
//! server then drives a multi-second state machine that plays Kismet sequences
//! at both ends, hides players, teleports them, and re-shows them.
//!
//! Reference: `python/cell/RingTransporter.py`. The Python code is the spec —
//! see [`transporter::RingTransporter`] for the exact state graph and timing.
//!
//! Module layout:
//! - `regions` — DB load of `ring_transport_regions` into [`RingRegion`].
//! - `transporter` — [`RingTransporter`] FSM + manager.
//! - `wire` — `RegionInfo` + `onRingTransporterList` payload encoding.
//! - `wire_helpers` — runtime byte-level helpers (`onSequence`, `onVisible`,
//!   `onStateFieldUpdate`, etc.) and the `BSF_MOVEMENT_LOCK` constant.
//! - `dispatch` — turns [`Effect`] values into `CellToBaseMsg` sends and
//!   spatial-grid mutations.
//! - `runtime` — public entry points (`handle_interact`,
//!   `handle_select_destination`, `handle_region_trigger`, `forget_player`,
//!   `run_tick_with_engine`).
//!
//! Every state that waits on something outside the FSM carries a bounded
//! abort deadline, and a departing player releases the rings holding them —
//! see [`transporter`] for the timeout table and audit defect H-B3 for what
//! their absence cost.

mod dispatch;
mod regions;
mod runtime;
mod transporter;
mod wire;
mod wire_helpers;

#[cfg(test)]
mod tests;

pub use regions::{load_ring_regions, RingRegion};
pub use runtime::{
    forget_player, handle_interact, handle_region_trigger, handle_remote_player_loaded,
    handle_select_destination, run_tick_with_engine,
};
pub use transporter::{
    AbortReason, Effect, RegionEvent, RingTransporter, RingTransporterManager, State,
};
pub use wire::{build_on_ring_transporter_list, encode_region_info};
pub use wire_helpers::{BSF_MOVEMENT_LOCK, METHOD_ON_RING_TRANSPORTER_LIST};
