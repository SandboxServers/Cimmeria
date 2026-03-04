//! CellApp service.
//!
//! Manages spatial entity simulation, world cells, movement, and Area of
//! Interest calculations. Mirrors the C++ CellApp that partitions the game
//! world into spatial cells and simulates entity interactions within them.

use std::net::SocketAddr;

use cimmeria_common::{EntityId, ServerConfig, SpaceId};

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

/// CellApp service managing spatial entity simulation.
///
/// In the original C++ architecture, this was the `CellApp` process that:
/// - Managed game spaces (world zones/instances)
/// - Simulated cell entity halves (spatial state, movement, AoI)
/// - Processed entity interactions within spatial proximity
/// - Ran the game tick loop for entity updates
pub struct CellService {
    /// Address the cell service binds to for BaseApp communication.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently running.
    pub is_running: bool,
}

impl CellService {
    /// Create a new cell service from server configuration.
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = format!("{}:{}", config.cell_host, config.cell_port)
            .parse()
            .unwrap_or_else(|_| {
                SocketAddr::from(([127, 0, 0, 1], config.cell_port))
            });

        Self {
            listener_addr,
            is_running: false,
        }
    }

    /// Start the cell service.
    ///
    /// Initializes the spatial grid, loads space definitions, and begins
    /// the game tick loop.
    pub async fn start(&mut self) -> Result<(), CellError> {
        tracing::info!(addr = %self.listener_addr, "Starting cell service");
        // TODO: Initialize spatial grid, load spaces, start tick loop
        tracing::trace!(addr = %self.listener_addr, "Cell service initialized (no bind yet — stub)");
        self.is_running = true;
        Ok(())
    }

    /// Stop the cell service gracefully.
    ///
    /// Stops the tick loop, saves spatial state, and notifies the BaseApp.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping cell service");
        // TODO: Stop tick loop, save state
        self.is_running = false;
        tracing::trace!("Cell service stopped");
    }

    /// Create a new game space.
    ///
    /// Initializes spatial partitioning for the space and loads navigation
    /// meshes and spawn data from the content engine.
    pub async fn create_space(&self, space_id: SpaceId) -> Result<(), CellError> {
        if !self.is_running {
            return Err(CellError::NotRunning);
        }

        tracing::debug!(%space_id, "Creating game space");
        // TODO: Initialize space grid, load navmesh, populate spawns
        Ok(())
    }

    /// Destroy a game space and remove all entities within it.
    pub async fn destroy_space(&self, space_id: SpaceId) -> Result<(), CellError> {
        if !self.is_running {
            return Err(CellError::NotRunning);
        }

        tracing::debug!(%space_id, "Destroying game space");
        // TODO: Remove all entities, clean up spatial data
        Ok(())
    }

    /// Add an entity to a space at a given position.
    ///
    /// Registers the entity with the spatial grid and begins AoI tracking.
    pub async fn add_entity_to_space(
        &self,
        entity_id: EntityId,
        space_id: SpaceId,
    ) -> Result<(), CellError> {
        if !self.is_running {
            return Err(CellError::NotRunning);
        }

        tracing::debug!(%entity_id, %space_id, "Adding entity to space");
        // TODO: Register with spatial grid, start AoI tracking
        Ok(())
    }

    /// Remove an entity from its current space.
    pub async fn remove_entity_from_space(
        &self,
        entity_id: EntityId,
    ) -> Result<(), CellError> {
        if !self.is_running {
            return Err(CellError::NotRunning);
        }

        tracing::debug!(%entity_id, "Removing entity from space");
        // TODO: Unregister from spatial grid, notify AoI subscribers
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn create_space_fails_when_not_running() {
        let config = ServerConfig::default();
        let svc = CellService::new(&config);
        let result = svc.create_space(SpaceId(1)).await;
        assert!(result.is_err());
    }
}
