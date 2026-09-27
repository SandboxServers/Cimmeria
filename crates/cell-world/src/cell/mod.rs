//! The cell's world state, under the `cell::` paths it had in
//! `cimmeria-services`.
//!
//! `cimmeria-services`' `cell` module re-exports each of these modules at the
//! same path, beside the cell systems that sit above this crate (combat,
//! content, interactions, the cell-method handlers, the console and the
//! service loop).

pub mod arrival;
pub mod black_market;
pub mod combat;
pub mod content_events;
pub mod cover;
pub mod dispatch;
pub mod duel;
pub mod effects;
pub mod org_creation;
pub mod pets;
pub mod playtest_friction;
pub mod playtest_friction_watch;
pub mod ring_transport;
pub mod service;
pub mod space_manager;
pub mod squad;

/// Seed-vs-navmesh guards for the Harset coordinates placed from map data
/// (`docs/analysis/harset-rebuild/placements/`). Test-only; in
/// `cimmeria-services` until wave C6 of the services crate split.
#[cfg(test)]
mod harset_placement_tests;
/// The spawner tests that need `SpaceManager`, cover or the aggression
/// helpers. Test-only; in `cimmeria-services` until wave C6.
#[cfg(test)]
mod spawner_tests;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::cell::{respawner_fallback, spawner};
pub(crate) use cimmeria_wire::cell::{kismet, messages, player_journal};

use cimmeria_common::{EntityId, SpaceId};

/// Errors specific to the cell service.
#[derive(Debug, thiserror::Error)]
pub enum CellError {
    #[error("Space {0} not found")]
    SpaceNotFound(SpaceId),

    #[error("Entity {0} not found in any cell")]
    EntityNotFound(EntityId),

    #[error("Failed to create space: {0}")]
    SpaceCreationFailed(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}
