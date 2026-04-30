//! Service orchestrator.
//!
//! Manages the lifecycle of all server services and provides shared access
//! to server state. In the original C++ architecture, each service ran as a
//! separate process; the orchestrator unifies them into a single Rust process
//! with coordinated startup and shutdown.

use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::RwLock;

use tokio::sync::mpsc;

use cimmeria_common::ServerConfig;

use tokio::sync::broadcast;

use crate::audit::{LoginEvent, LoginEventBuffer};
use crate::auth::{AuthService, ShardInfo};
use crate::base::BaseService;
use crate::cell::CellService;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::database::DatabasePool;
use crate::orchestrator_postgres::ensure_postgresql_running;
use crate::orchestrator_shards::{ShardRow, query_all_shards};

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

    /// Sender to the CellService message loop (for admin-triggered reload, etc.).
    pub cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
}

/// Service orchestrator that coordinates startup, shutdown, and state access.
///
/// This is the top-level entry point for running the Cimmeria server. It owns
/// the shared `ServerState` and provides methods to start all services in the
/// correct order (database -> auth -> base -> cell) and shut them down cleanly.
pub struct Orchestrator {
    state: Arc<RwLock<ServerState>>,
    login_tx: Option<broadcast::Sender<LoginEvent>>,
    login_buffer: Option<LoginEventBuffer>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given configuration.
    ///
    /// Initializes all services in a stopped state. Call [`start_all`] to
    /// begin accepting connections.
    pub fn new(config: ServerConfig) -> Self {
        tracing::trace!("Constructing AuthService");
        let auth = AuthService::new(&config);
        tracing::trace!("Constructing BaseService");
        let mut base = BaseService::new(&config);
        tracing::trace!("Constructing CellService");
        let mut cell = CellService::new(&config);

        // Wire Base↔Cell inter-service channels
        let (base_to_cell_tx, base_to_cell_rx) = mpsc::channel::<BaseToCellMsg>(256);
        let (cell_to_base_tx, cell_to_base_rx) = mpsc::channel::<CellToBaseMsg>(256);
        let cell_tx_for_admin = base_to_cell_tx.clone();
        cell.set_channels(base_to_cell_rx, cell_to_base_tx);
        base.set_cell_channel(base_to_cell_tx, cell_to_base_rx);

        let state = ServerState {
            auth,
            base,
            cell,
            db: None,
            start_time: Instant::now(),
            config,
            cell_tx: Some(cell_tx_for_admin),
        };

        tracing::trace!("Orchestrator constructed");
        Self {
            state: Arc::new(RwLock::new(state)),
            login_tx: None,
            login_buffer: None,
        }
    }

    /// Set the login event channel for audit logging.
    ///
    /// Must be called before [`start_all`] so auth handlers can emit events.
    pub fn set_login_event_channel(
        &mut self,
        tx: broadcast::Sender<LoginEvent>,
        buffer: LoginEventBuffer,
    ) {
        self.login_tx = Some(tx);
        self.login_buffer = Some(buffer);
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

        tracing::trace!("Acquiring state write lock");
        let mut state = self.state.write().await;

        // 0. Ensure PostgreSQL is running (auto-start if possible)
        ensure_postgresql_running(&state.config.db_connection_string).await;

        // 1. Connect to the database
        let db_conn = state.config.db_connection_string.clone();
        tracing::trace!(db_conn = %db_conn, "Connecting to database");
        let db_pool: Option<Arc<PgPool>> = match DatabasePool::connect(&db_conn).await {
            Ok(pool) => {
                let arc_pool = Arc::new(pool.pool().clone());
                state.db = Some(pool);
                tracing::info!("Database connected");
                Some(arc_pool)
            }
            Err(e) => {
                tracing::warn!("Database connection failed (continuing without DB): {e}");
                // Non-fatal for now: allows starting in dev mode without a database
                None
            }
        };

        // Pass DB pool to auth, base, and cell services
        if let Some(ref pool) = db_pool {
            state.auth.set_db_pool(Arc::clone(pool));
            state.base.set_db_pool(Arc::clone(pool));
            state.cell.set_db_pool(Arc::clone(pool));
        }

        // Wire login audit channel into auth service
        if let (Some(tx), Some(buf)) = (&self.login_tx, &self.login_buffer) {
            state.auth.set_login_event_tx(tx.clone(), buf.clone());
        }

        // Register shards with auth before starting the HTTP listener so
        // Phase 1 responses include them immediately. Shard names come from
        // the DB `shards` table; host/port are runtime config for now (only
        // one BaseApp process). When we support multiple BaseApps each shard
        // row will need its own host/port columns.
        let shard_rows = match &db_pool {
            Some(pool) => query_all_shards(pool).await,
            None => vec![ShardRow { name: "Shard".to_string(), protected: false }],
        };
        for row in shard_rows {
            let shard = ShardInfo {
                name: row.name,
                host: state.config.base_external_host.clone(),
                port: state.config.base_port,
                protected: row.protected,
            };
            state.auth.register_shard(shard);
        }

        // 2. Start auth service
        tracing::trace!(addr = %state.auth.logon_addr, "Starting auth service");
        state
            .auth
            .start()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Auth service failed to start");
                OrchestratorError::AuthStartFailed(e.to_string())
            })?;
        tracing::trace!("Auth service started successfully");

        // 3. Start base service (wire in pending_logins from auth first)
        let pending_logins = state.auth.pending_logins_arc();
        state.base.set_pending_logins(pending_logins);
        tracing::trace!(addr = %state.base.listener_addr, "Starting base service");
        state
            .base
            .start()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Base service failed to start");
                OrchestratorError::BaseStartFailed(e.to_string())
            })?;
        tracing::trace!("Base service started successfully");

        // 4. Start cell service
        tracing::trace!(addr = %state.cell.listener_addr, "Starting cell service");
        state
            .cell
            .start()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Cell service failed to start");
                OrchestratorError::CellStartFailed(e.to_string())
            })?;
        tracing::trace!("Cell service started successfully");

        // 5. Start minigame server
        let mg_registry = state.base.minigame_registry.clone();
        let mg_port = state.config.minigame_port;
        let mg_result_tx = state.cell.cell_to_base_tx();
        if let Some(result_tx) = mg_result_tx {
            tokio::spawn(async move {
                crate::minigame::server::run(
                    "0.0.0.0", mg_port, mg_port,
                    mg_registry, result_tx,
                ).await;
            });
            tracing::info!(port = mg_port, "Minigame server started");
        } else {
            tracing::warn!("Minigame server not started — no CellToBase channel available");
        }

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

        tracing::trace!("Acquiring state write lock for shutdown");
        let mut state = self.state.write().await;

        tracing::trace!("Stopping cell service");
        state.cell.stop().await;
        tracing::trace!("Cell service stopped");

        tracing::trace!("Stopping base service");
        state.base.stop().await;
        tracing::trace!("Base service stopped");

        tracing::trace!("Stopping auth service");
        state.auth.stop().await;
        tracing::trace!("Auth service stopped");

        // Drop the database pool
        tracing::trace!("Dropping database pool");
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
