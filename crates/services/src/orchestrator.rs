//! Service orchestrator.
//!
//! Manages the lifecycle of all server services and provides shared access
//! to server state. In the original C++ architecture, each service ran as a
//! separate process; the orchestrator unifies them into a single Rust process
//! with coordinated startup and shutdown.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use cimmeria_common::ServerConfig;

use crate::auth::AuthService;
use crate::base::BaseService;
use crate::cell::CellService;
use crate::database::DatabasePool;

/// Errors specific to the orchestrator.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Auth service failed to start: {0}")]
    AuthStartFailed(String),

    #[error("Base service failed to start: {0}")]
    BaseStartFailed(String),

    #[error("Cell service failed to start: {0}")]
    CellStartFailed(String),

    #[error("Database connection failed: {0}")]
    DatabaseFailed(String),
}

/// Shared server state accessible by all services and the admin API.
///
/// Wrapped in `Arc<RwLock<...>>` so that the admin API can inspect and
/// modify server state while services are running.
pub struct ServerState {
    /// Authentication service instance.
    pub auth: AuthService,

    /// BaseApp service instance.
    pub base: BaseService,

    /// CellApp service instance.
    pub cell: CellService,

    /// Database connection pool (None if not yet connected).
    pub db: Option<DatabasePool>,

    /// Timestamp when the server was started.
    pub start_time: Instant,

    /// Server configuration snapshot.
    pub config: ServerConfig,
}

/// Service orchestrator that coordinates startup, shutdown, and state access.
///
/// This is the top-level entry point for running the Cimmeria server. It owns
/// the shared `ServerState` and provides methods to start all services in the
/// correct order (database -> auth -> base -> cell) and shut them down cleanly.
pub struct Orchestrator {
    state: Arc<RwLock<ServerState>>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given configuration.
    ///
    /// Initializes all services in a stopped state. Call [`start_all`] to
    /// begin accepting connections.
    pub fn new(config: ServerConfig) -> Self {
        let auth = AuthService::new(&config);
        let base = BaseService::new(&config);
        let cell = CellService::new(&config);

        let state = ServerState {
            auth,
            base,
            cell,
            db: None,
            start_time: Instant::now(),
            config,
        };

        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Start all services in dependency order.
    ///
    /// Startup sequence:
    /// 1. Database connection pool
    /// 2. Authentication service
    /// 3. Base service (depends on auth for session validation)
    /// 4. Cell service (depends on base for entity routing)
    pub async fn start_all(&self) -> Result<(), OrchestratorError> {
        tracing::info!("Starting all services");

        let mut state = self.state.write().await;

        // 1. Connect to the database
        let db_conn = state.config.db_connection_string.clone();
        match DatabasePool::connect(&db_conn).await {
            Ok(pool) => {
                state.db = Some(pool);
                tracing::info!("Database connected");
            }
            Err(e) => {
                tracing::warn!("Database connection failed (continuing without DB): {e}");
                // Non-fatal for now: allows starting in dev mode without a database
            }
        }

        // 2. Start auth service
        state
            .auth
            .start()
            .await
            .map_err(|e| OrchestratorError::AuthStartFailed(e.to_string()))?;

        // 3. Start base service
        state
            .base
            .start()
            .await
            .map_err(|e| OrchestratorError::BaseStartFailed(e.to_string()))?;

        // 4. Start cell service
        state
            .cell
            .start()
            .await
            .map_err(|e| OrchestratorError::CellStartFailed(e.to_string()))?;

        // Record actual start time now that all services are up
        state.start_time = Instant::now();

        tracing::info!("All services started successfully");
        Ok(())
    }

    /// Stop all services in reverse dependency order.
    ///
    /// Shutdown sequence: cell -> base -> auth (reverse of startup).
    pub async fn stop_all(&self) {
        tracing::info!("Stopping all services");

        let mut state = self.state.write().await;
        state.cell.stop().await;
        state.base.stop().await;
        state.auth.stop().await;

        // Drop the database pool
        state.db = None;

        tracing::info!("All services stopped");
    }

    /// Get a shared reference to the server state.
    ///
    /// Used by the admin API to inspect and modify server state.
    pub fn state(&self) -> Arc<RwLock<ServerState>> {
        Arc::clone(&self.state)
    }

    /// Get the server uptime since the last successful start.
    pub async fn uptime(&self) -> Duration {
        let state = self.state.read().await;
        state.start_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_orchestrator_creates_stopped_services() {
        let config = ServerConfig::default();
        let orch = Orchestrator::new(config);
        // Just verify it constructs without panicking
        let _ = orch.state();
    }

    #[tokio::test]
    async fn uptime_increases() {
        let config = ServerConfig::default();
        let orch = Orchestrator::new(config);
        let d1 = orch.uptime().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        let d2 = orch.uptime().await;
        assert!(d2 > d1);
    }
}
