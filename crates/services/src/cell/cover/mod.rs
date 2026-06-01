//! Server-driven NPC cover system.
//!
//! Loads cover-prefab data from `resources.cover_sets` + `resources.cover_nodes`
//! (extracted from the SGW client's `covernodes_*.pak` archives via
//! `tools/ue3_extract_cover_nodes.py`), indexes it spatially, and provides
//! reservation + scoring primitives for the NPC AI cover-advance behavior.
//!
//! See `docs/architecture/cover-system.md` for the design overview and
//! `docs/reverse-engineering/findings/cover-system.md` for the binary
//! format + wire-surface reverse-engineering that motivated the design.
//!
//! Submodules:
//! - [`types`] — `CoverNode`, `CoverSetMeta`, `CoverHeight`, `CoverQuality`,
//!   `CoverSlotKey`, `Cover` service handle.
//! - [`loader`] — PostgreSQL → in-memory `Vec<CoverNode>` load at cell startup.
//! - [`spatial`] — uniform-grid spatial index over the loaded nodes for fast
//!   `nearby_nodes(pos, radius)` lookups. Per-process (not per-space — see
//!   the chunk→space mapping caveat in the ADR).
//! - [`reservation`] — `reserve_cover_slot` / `release_cover_slot` honoring
//!   the `SGWCoverSet.def`'s auto-release-prior semantics.

mod ai_integration;
mod detection;
mod loader;
mod reservation;
mod scoring;
mod spatial;
mod types;

#[cfg(test)]
mod loader_live_db_tests;
#[cfg(test)]
mod tests;

pub use ai_integration::{maintain_cover_for_npc, CoverDecision};
pub use detection::{
    run_detection_tick, CoverDetectionTable, CoverDetectionTick, DurationCoverEvent,
    EnteredCoverEvent, LeftCoverEvent, COVER_DURATION_MILESTONES_SECS, COVER_PROXIMITY_RADIUS,
};
pub use loader::{load_cover_nodes, load_cover_sets, CoverLoadError};
pub use reservation::{CoverReservations, ReserveError};
pub use scoring::{
    is_flanked, pick_best, score_node, CoverWeights, ScoringContext, MAX_COVER_DISTANCE,
};
pub use spatial::CoverIndex;
pub use types::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSetMeta, CoverSlotKey};
