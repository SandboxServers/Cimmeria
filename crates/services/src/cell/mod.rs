//! CellApp service.
//!
//! Manages spatial entity simulation, world cells, movement, and Area of
//! Interest calculations. Mirrors the C++ CellApp that partitions the game
//! world into spatial cells and simulates entity interactions within them.

pub mod abilities;
pub mod arrival;
pub mod cell_methods;
pub mod chat;
// The server->client method index tables are wire contract (cimmeria-wire).
pub use cimmeria_wire::cell::client_methods;
pub mod combat;
pub mod console;
pub mod content;
pub mod cover;
pub mod dispatch;
pub mod effects;
pub mod gate_travel;
/// Seed-vs-navmesh guards for the Harset coordinates placed from map data
/// (`docs/analysis/harset-rebuild/placements/`). Test-only.
#[cfg(test)]
mod harset_placement_tests;
pub mod interactions;
pub(crate) use cimmeria_wire::cell::kismet;
pub mod mail;
pub mod messages;
pub mod missions;
pub(crate) use cimmeria_wire::cell::player_journal;
pub(crate) mod playtest_friction;
pub(crate) mod playtest_friction_watch;
/// Crate-internal: the shared respawner search behind both
/// [`arrival::resolve_arrival`] and `SpaceManager::resolve_recovery_position`.
/// In `cimmeria-cell-catalog` since wave W2b.
pub(crate) use cimmeria_cell_catalog::cell::respawner_fallback;
pub mod ring_transport;
mod service;
pub mod space_manager;
// Crate-internal: `transfer_player_to_space` is a destructive, unauthenticated
// entry point (privilege is enforced by the console dispatch layer above it),
// so it must not be reachable from outside this crate. Its production callers
// are the `.goto`/`.summon`/`.gotolocation` console commands in
// `cell::console::travel` (P46).
pub(crate) mod space_transfer;
// The spawner's DB loaders are in `cimmeria-cell-catalog` (wave W2b).
// Populating spaces from their records is `space_manager::spawn_npcs_from_records`.
pub use cimmeria_cell_catalog::cell::spawner;
/// The spawner tests that need `SpaceManager`, combat or the GM spawn
/// handler, which are still in this crate. Test-only.
#[cfg(test)]
mod spawner_tests;

use cimmeria_common::{EntityId, SpaceId};

pub use service::CellService;

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

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::ServerConfig;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = CellService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 50000);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let config = ServerConfig::default();
        let mut svc = CellService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }
}
