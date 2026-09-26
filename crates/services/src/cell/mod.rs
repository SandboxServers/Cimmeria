//! CellApp service.
//!
//! Manages spatial entity simulation, world cells, movement, and Area of
//! Interest calculations. Mirrors the C++ CellApp that partitions the game
//! world into spatial cells and simulates entity interactions within them.
//!
//! The world state every cell system shares (`space_manager`, `arrival`,
//! `cover`, the NPC AI's state primitives, the ring-transporter state machine,
//! the synchronous effect scripts, the GM gate and `CellError`) is in
//! `cimmeria-cell-world` (wave C1 of `docs/architecture/services-crate-split.md`).
//! Its modules are re-exported here at their old paths; `dispatch`,
//! `ring_transport` and `service` are partly there, and their modules here
//! re-export the moved halves.
//!
//! Combat (`abilities`, `combat`, the effect pulsing in `effects`, the NPC AI's
//! behaviour in `service::npc_ai`, and the bandolier, reload and item-sequence
//! cell methods) is in `cimmeria-cell-combat` (wave C2), re-exported the same
//! way.

// In `cimmeria-cell-combat` (wave C2).
pub use cimmeria_cell_combat::cell::abilities;
// In `cimmeria-cell-world` (wave C1).
pub use cimmeria_cell_world::cell::arrival;
pub mod cell_methods;
// Under the console since the services-split preparation for C4-C6 (§2H).
pub use console::chat;
// The server->client method index tables are wire contract (cimmeria-wire).
pub use cimmeria_cell_combat::cell::combat;
pub use cimmeria_wire::cell::client_methods;
pub mod console;
pub mod content;
// The seam combat raises content events through (§2E), in `cimmeria-cell-world`.
pub use cimmeria_cell_world::cell::content_events;
pub use cimmeria_cell_world::cell::cover;
pub mod dispatch;
pub use cimmeria_cell_combat::cell::effects;
pub mod gate_travel;
/// Seed-vs-navmesh guards for the Harset coordinates placed from map data
/// (`docs/analysis/harset-rebuild/placements/`). Test-only.
#[cfg(test)]
mod harset_placement_tests;
pub mod interactions;
pub(crate) use cimmeria_wire::cell::kismet;
pub mod mail;
// The Base<->Cell message contract is in `cimmeria-wire` (wave W3a).
pub use cimmeria_wire::cell::messages;
pub mod missions;
pub(crate) use cimmeria_cell_world::cell::{playtest_friction, playtest_friction_watch};
pub(crate) use cimmeria_wire::cell::player_journal;
pub mod ring_transport;
mod service;
pub use cimmeria_cell_world::cell::space_manager;
// Crate-internal: `transfer_player_to_space` is a destructive, unauthenticated
// entry point (privilege is enforced by the console dispatch layer above it),
// so it must not be reachable from outside this crate. Its production callers
// are the `.goto`/`.summon`/`.gotolocation` console commands in
// `cell::console::travel` (P46).
pub(crate) mod space_transfer;
// The respawn fork the player's Defeat Window and the GM `gmRespawn` share,
// with the client-cache replays it queues after the reanchor. Beside gate
// travel and the space transfer, one layer below the cell methods and the GM
// console (services-crate-split.md §2H).
pub(crate) mod respawn;
// Player-to-player trade session state and its outbound wire, below the trade
// cell-method handlers, which gate travel and the space transfer cancel on
// departure (§2H).
pub(crate) mod trade;
// The spawner's DB loaders are in `cimmeria-cell-catalog` (wave W2b).
// Populating spaces from their records is `space_manager::spawn_npcs_from_records`.
pub use cimmeria_cell_catalog::cell::spawner;
/// The spawner tests that need `SpaceManager`, combat or the GM spawn
/// handler, which are still in this crate. Test-only.
#[cfg(test)]
mod spawner_tests;

pub use service::CellService;

/// Errors specific to the cell service. In `cimmeria-cell-world` (wave C1).
pub use cimmeria_cell_world::cell::CellError;

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
