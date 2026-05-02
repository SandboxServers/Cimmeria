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
//! - `transporter` — [`RingTransporter`] state machine + manager + tick.
//! - `wire` — `RegionInfo` + `onRingTransporterList` payload encoding.

mod regions;
mod transporter;
mod wire;

pub use regions::{load_ring_regions, RingRegion};
pub use transporter::{
    ring_transport_tick, Effect, RegionEvent, RingTransporter, RingTransporterManager, State,
};
pub use wire::{build_on_ring_transporter_list, encode_region_info};
