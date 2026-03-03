//! BaseApp service.
//!
//! Manages persistent entity state, player base data, and shard connections.
//! Mirrors the C++ BaseApp that listens on port 32832 for client connections
//! after authentication and maintains base entity halves.

use std::net::SocketAddr;

use cimmeria_common::{EntityId, ServerConfig};

/// Errors specific to the base service.
#[derive(Debug, thiserror::Error)]
pub enum BaseError {
    #[error("Entity {0} not found")]
    EntityNotFound(EntityId),

    #[error("Entity creation failed: {0}")]
    CreationFailed(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

/// BaseApp service managing persistent entity state and client connections.
///
/// In the original C++ architecture, this was the `BaseApp` process that:
/// - Accepted client connections on `shard_port` (default 32832)
/// - Managed base entity halves (persistent state)
/// - Routed entity messages between clients and CellApp
/// - Handled entity creation, destruction, and persistence to the database
pub struct BaseService {
    /// Address the base service listener binds to.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently running.
    pub is_running: bool,
}

impl BaseService {
    /// Create a new base service from server configuration.
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = format!("{}:{}", config.base_host, config.base_port)
            .parse()
            .unwrap_or_else(|_| {
                SocketAddr::from(([127, 0, 0, 1], config.base_port))
            });

        Self {
            listener_addr,
            is_running: false,
        }
    }

    /// Start the base service.
    ///
    /// Binds the TCP listener, registers with the authentication server,
    /// and begins accepting authenticated client connections.
    pub async fn start(&mut self) -> Result<(), BaseError> {
        tracing::info!(addr = %self.listener_addr, "Starting base service");
        // TODO: Bind listener, register with auth service, start accept loop
        self.is_running = true;
        Ok(())
    }

    /// Stop the base service gracefully.
    ///
    /// Persists all dirty entity state, notifies connected clients, and
    /// closes the listener.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping base service");
        // TODO: Persist entity state, close connections
        self.is_running = false;
    }

    /// Create a new base entity.
    ///
    /// Allocates an entity ID, initializes default properties from the entity
    /// definition, and registers it with the entity manager.
    pub async fn create_base_entity(&self) -> Result<EntityId, BaseError> {
        if !self.is_running {
            return Err(BaseError::NotRunning);
        }

        tracing::debug!("Creating base entity");
        // TODO: Allocate entity ID, create from definition, register
        Ok(EntityId(0))
    }

    /// Destroy a base entity and clean up all associated state.
    ///
    /// Removes the entity from the manager, notifies the CellApp to destroy
    /// the cell half (if any), and deletes persistent data.
    pub async fn destroy_base_entity(&self, entity_id: EntityId) -> Result<(), BaseError> {
        if !self.is_running {
            return Err(BaseError::NotRunning);
        }

        tracing::debug!(%entity_id, "Destroying base entity");
        // TODO: Remove from manager, notify CellApp, delete persistent data
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 32832);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let config = ServerConfig::default();
        let mut svc = BaseService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }

    #[tokio::test]
    async fn create_entity_fails_when_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        let result = svc.create_base_entity().await;
        assert!(result.is_err());
    }
}
