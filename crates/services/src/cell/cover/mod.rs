//! Server-driven NPC cover system.
//!
//! Loads world-space cover from `resources.cover_sets` + `resources.cover_nodes`
//! (extracted from the `SGWSpecCoverNode` / `CoverNodeArray` markers baked
//! into each map's `.umap` chunks by the `cover_extract` binary in
//! `crates/navmesh-extractor` — `docs/engine/cover-extraction.md`), indexes
//! it spatially per world, and provides reservation + scoring primitives
//! for the NPC AI cover behaviour: hold the slot an NPC spawns at, seek a
//! firing position in combat, Cover Stance on arrival.
//!
//! Design and decisions: `docs/architecture/cover-system.md` (NA22). The
//! binary format + wire-surface reverse engineering is in
//! `docs/reverse-engineering/findings/cover-system.md` and
//! `cover-world-placement.md`.
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
//!   the `SGWCoverSet.def`'s auto-release-prior semantics, plus the per-NPC
//!   stance and seek-deferral state that lives with a reservation.
//! - [`ai_integration`] — the per-tick hold / release / seek decision.
//! - [`peek`] — the peek point an NPC at a cover slot looks from (NA23).
//! - [`stance`] — spawn hold, Cover Stance grant/revoke, and the one release
//!   every combat-end path calls.

mod ai_integration;
#[cfg(test)]
mod ai_integration_tests;
mod coverage;
mod detection;
mod loader;
mod peek;
mod reservation;
mod scoring;
#[cfg(test)]
mod scoring_tests;
mod spatial;
mod stance;
mod types;

#[cfg(test)]
mod loader_live_db_tests;
#[cfg(test)]
mod tests;

pub(crate) use ai_integration::horizontal;
pub use ai_integration::{
    maintain_cover_for_npc, maintain_cover_for_npc_checked, maintain_cover_for_npc_traced,
    CoverDecision, CoverQuery, CoverTrace, NoCoverReason, ReleaseReason, COVER_ARRIVE_RADIUS,
    COVER_BLIND_GRACE, COVER_REPICK_COOLDOWN, IN_RANGE_MAX_MOVE, PICK_RANGE_MARGIN, SEEK_RETRY,
};
pub use coverage::{log_space_coverage, space_coverage, SpaceCoverage, NODE_FLOOR_TOLERANCE};
pub use detection::{
    run_detection_tick, sets_near, CoverDetectionTable, CoverDetectionTick, DurationCoverEvent,
    EnteredCoverEvent, LeftCoverEvent, COVER_DURATION_MILESTONES_SECS, COVER_PROXIMITY_RADIUS,
};
pub use loader::{load_cover_nodes, load_cover_sets, CoverLoadError};
pub use peek::{
    find_peek, find_peek_point, sight_from_slot, stand_behind, Peek, PeekKind, SlotSight,
    PEEK_FORWARD_MAX, PEEK_FORWARD_MIN, PEEK_FORWARD_STEP, PEEK_LATERAL_MARGIN, PEEK_MAX_WALK,
    PEEK_MIN_CLEARANCE, PEEK_SNAP_HALF_HEIGHT, PEEK_SNAP_RADIUS,
};
pub use reservation::{CoverReservations, ReserveError};
pub use scoring::{
    allies_near, defends_for_pick, is_flanked, pick_best, pick_best_filtered, pick_best_traced,
    score_node, CoverWeights, PickTrace, ScoredCandidate, ScoringContext, FLANK_PICK_DOT,
    FLANK_RELEASE_DOT, MAX_COVER_DISTANCE, SQUAD_AFFINITY_RADIUS,
};
pub use spatial::CoverIndex;
pub use stance::{
    grant_cover_stance, hold_spawn_cover, hold_spawn_cover_all, release_npc_cover,
    revoke_cover_stance, COVER_STANCE_ABILITY, COVER_STANCE_EFFECT, COVER_STANCE_REMOVE_EFFECT,
};
pub use types::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSetMeta, CoverSlotKey};

/// World id the cover unit tests place their nodes in (Castle_CellBlock).
#[cfg(test)]
pub(crate) const TEST_WORLD_ID: i32 = 12;
