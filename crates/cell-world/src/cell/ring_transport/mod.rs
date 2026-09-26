//! Ring transporter system: the state machine and its world-side pieces.
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
//! - `runtime::teardown` — [`forget_player`], the departing-player hook
//!   `SpaceManager::disconnect_entity` calls, and the release-only effect
//!   dispatch it shares with the abort paths.
//!
//! The effect dispatcher and the runtime entry points (`handle_interact`,
//! `handle_select_destination`, `handle_region_trigger`,
//! `run_tick_with_engine`) fire content chains, so they sit above this crate
//! (`cimmeria-services`, `cell::ring_transport`, which re-exports this module
//! beside them; docs/architecture/services-crate-split.md §2G). They reach the
//! FSM and the wire helpers through the public `transporter`, `regions` and
//! `wire_helpers` modules.
//!
//! Every state that waits on something outside the FSM carries a bounded
//! abort deadline, and a departing player releases the rings holding them —
//! see [`transporter`] for the timeout table and audit defect H-B3 for what
//! their absence cost.

pub mod regions;
pub mod runtime;
pub mod transporter;
mod wire;
pub mod wire_helpers;

pub use regions::{audit_ring_pads, load_ring_regions, RingRegion};
pub use runtime::forget_player;
pub use transporter::{
    AbortReason, Effect, RegionEvent, RingTransporter, RingTransporterManager, State,
};
pub use wire::{build_on_ring_transporter_list, encode_region_info};
pub use wire_helpers::{BSF_MOVEMENT_LOCK, METHOD_ON_RING_TRANSPORTER_LIST};
