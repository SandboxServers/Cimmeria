//! CellApp service.
//!
//! Manages spatial entity simulation, world cells, movement, and Area of
//! Interest calculations. Mirrors the C++ CellApp that partitions the game
//! world into spatial cells and simulates entity interactions within them.

pub mod abilities;
pub mod cell_methods;
pub mod chat;
pub mod client_methods;
pub mod combat;
pub mod content;
pub mod cover;
pub mod dispatch;
pub mod effects;
pub mod gate_travel;
pub mod gm_command;
pub mod interactions;
pub mod mail;
pub mod messages;
pub mod missions;
pub mod ring_transport;
mod service;
pub mod space_manager;
pub mod spawner;

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
