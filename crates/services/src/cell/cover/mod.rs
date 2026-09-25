//! Server-driven NPC cover system.
//!
//! Loads world-space cover from `resources.cover_sets` + `resources.cover_nodes`
//! (extracted from the `SGWSpecCoverNode` / `CoverNodeArray` markers baked
//! into each map's `.umap` chunks by the `cover_extract` binary in
//! `crates/navmesh-extractor` — `docs/engine/cover-extraction.md`), indexes
//! it spatially per world, and provides reservation + scoring primitives
//! for the NPC AI cover-advance behavior.
//!
//! See `docs/reverse-engineering/findings/cover-system.md` for the binary
//! format + wire-surface reverse-engineering that motivated the design.
//! There is no architecture doc yet; the NPC AI restoration packet NA22
//! writes `docs/architecture/cover-system.md`.
//!
//! Submodules:
//! - [`types`] — `CoverNode`, `CoverSetMeta`, `CoverHeight`, `CoverQuality`,
//!   `CoverSlotKey`, `Cover` service handle.
//! - [`loader`] — PostgreSQL → in-memory `Vec<CoverNode>` load at cell startup.
//! - [`spatial`] — uniform-grid spatial index over the loaded nodes for fast
//!   `nearby(world_id, pos, radius)` lookups. One index per process,
//!   partitioned by `resources.worlds.world_id`: a query names a world and
//!   never sees another world's nodes. Instances of one world share its
//!   cover (positions are per world, not per space instance).
//! - [`reservation`] — `reserve_cover_slot` / `release_cover_slot` honoring
//!   the `SGWCoverSet.def`'s auto-release-prior semantics.

mod ai_integration;
mod coverage;
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

pub use ai_integration::{
    maintain_cover_for_npc, maintain_cover_for_npc_traced, CoverDecision, CoverTrace, NoCoverReason,
};
pub use coverage::{log_space_coverage, space_coverage, SpaceCoverage, NODE_FLOOR_TOLERANCE};
pub use detection::{
    run_detection_tick, sets_near, CoverDetectionTable, CoverDetectionTick, DurationCoverEvent,
    EnteredCoverEvent, LeftCoverEvent, COVER_DURATION_MILESTONES_SECS, COVER_PROXIMITY_RADIUS,
};
pub use loader::{load_cover_nodes, load_cover_sets, CoverLoadError};
pub use reservation::{CoverReservations, ReserveError};
pub use scoring::{
    is_flanked, pick_best, pick_best_traced, score_node, CoverWeights, PickTrace, ScoredCandidate,
    ScoringContext, MAX_COVER_DISTANCE,
};
pub use spatial::CoverIndex;
pub use types::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSetMeta, CoverSlotKey};

/// World id the cover unit tests place their nodes in (Castle_CellBlock).
#[cfg(test)]
pub(crate) const TEST_WORLD_ID: i32 = 12;
