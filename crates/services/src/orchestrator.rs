//! Service orchestrator.
//!
//! Manages the lifecycle of all server services and provides shared access
//! to server state. In the original C++ architecture, each service ran as a
//! separate process; the orchestrator unifies them into a single Rust process
//! with coordinated startup and shutdown.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::RwLock;

use tokio::sync::mpsc;

use cimmeria_common::ServerConfig;

use crate::auth::{AuthService, ShardInfo};
use crate::base::BaseService;
use crate::cell::CellService;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
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
        tracing::trace!("Constructing AuthService");
        let auth = AuthService::new(&config);
        tracing::trace!("Constructing BaseService");
        let mut base = BaseService::new(&config);
        tracing::trace!("Constructing CellService");
        let mut cell = CellService::new(&config);

        // Wire Base↔Cell inter-service channels
        let (base_to_cell_tx, base_to_cell_rx) = mpsc::channel::<BaseToCellMsg>(256);
        let (cell_to_base_tx, cell_to_base_rx) = mpsc::channel::<CellToBaseMsg>(256);
        cell.set_channels(base_to_cell_rx, cell_to_base_tx);
        base.set_cell_channel(base_to_cell_tx, cell_to_base_rx);

        let state = ServerState {
            auth,
            base,
            cell,
            db: None,
            start_time: Instant::now(),
            config,
        };

        tracing::trace!("Orchestrator constructed");
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

        // Register shards with auth before starting the HTTP listener so
        // Phase 1 responses include them immediately. Shard names come from
        // the DB `shards` table; host/port are runtime config for now (only
        // one BaseApp process). When we support multiple BaseApps each shard
        // row will need its own host/port columns.
        let shard_names = match &db_pool {
            Some(pool) => query_all_shards(pool).await,
            None => vec!["Shard".to_string()],
        };
        for name in shard_names {
            let shard = ShardInfo {
                name,
                host: state.config.base_external_host.clone(),
                port: state.config.base_port,
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

// ── Shard lookup ─────────────────────────────────────────────────────────────

/// Query all shard names from the `shards` table, ordered by shard_id.
/// Returns a fallback single-entry list on error.
async fn query_all_shards(pool: &Arc<PgPool>) -> Vec<String> {
    match sqlx::query_scalar::<_, String>(
        "SELECT name FROM shards ORDER BY shard_id",
    )
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(names) if !names.is_empty() => {
            tracing::info!(count = names.len(), "Loaded shards from database: {:?}", names);
            names
        }
        Ok(_) => {
            tracing::error!("No shards found in database — using fallback name 'Shard'. Run: INSERT INTO shards (shard_id, name, key, protected) VALUES (1, 'Test', '', false);");
            vec!["Shard".to_string()]
        }
        Err(e) => {
            tracing::error!("Failed to query shards table: {e} — using fallback name 'Shard'. Ensure the 'shards' table exists (re-run Initialize-CimmeriaDatabase -Force).");
            vec!["Shard".to_string()]
        }
    }
}

// ── PostgreSQL auto-start ─────────────────────────────────────────────────────

/// Parse the `port=NNNN` value from a libpq-style connection string.
fn parse_pg_port(conn_str: &str) -> u16 {
    for part in conn_str.split_whitespace() {
        if let Some(val) = part.strip_prefix("port=") {
            if let Ok(p) = val.parse::<u16>() {
                return p;
            }
        }
    }
    5433 // project default
}

/// Parse the `host=...` value from a libpq-style connection string.
fn parse_pg_host(conn_str: &str) -> String {
    for part in conn_str.split_whitespace() {
        if let Some(val) = part.strip_prefix("host=") {
            return val.to_string();
        }
    }
    "localhost".to_string()
}

/// Try to find pg_ctl.exe by searching common project-relative paths.
fn find_pg_ctl() -> Option<PathBuf> {
    let candidates = [
        // Relative to CWD (typical: running from project root)
        PathBuf::from("external/postgresql_server/bin/pg_ctl.exe"),
        // One level up (if running from a subdirectory)
        PathBuf::from("../external/postgresql_server/bin/pg_ctl.exe"),
    ];

    for path in &candidates {
        if path.exists() {
            tracing::debug!(path = %path.display(), "Found pg_ctl");
            return Some(path.clone());
        }
    }

    // Try relative to the executable location
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let pg_ctl = exe_dir.join("external/postgresql_server/bin/pg_ctl.exe");
            if pg_ctl.exists() {
                tracing::debug!(path = %pg_ctl.display(), "Found pg_ctl relative to exe");
                return Some(pg_ctl);
            }
        }
    }

    None
}

/// Find the PostgreSQL data directory.
fn find_pg_data() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("server/pgdata"),
        PathBuf::from("../server/pgdata"),
    ];

    for path in &candidates {
        if path.join("PG_VERSION").exists() {
            tracing::debug!(path = %path.display(), "Found pgdata directory");
            return Some(path.clone());
        }
    }

    None
}

/// Check if PostgreSQL is reachable on the given host:port.
async fn pg_is_running(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    tokio::net::TcpStream::connect(&addr)
        .await
        .is_ok()
}

/// Ensure PostgreSQL is running before we try to connect. If it's not listening
/// on the configured port, attempt to start it using `pg_ctl` from the bundled
/// PostgreSQL installation at `external/postgresql_server/`.
async fn ensure_postgresql_running(conn_str: &str) {
    let host = parse_pg_host(conn_str);
    let port = parse_pg_port(conn_str);

    // Only auto-start for localhost connections
    if host != "localhost" && host != "127.0.0.1" && host != "::1" {
        tracing::debug!(host = %host, "PostgreSQL host is remote — skipping auto-start");
        return;
    }

    if pg_is_running(&host, port).await {
        tracing::info!(port, "PostgreSQL already running");
        return;
    }

    tracing::warn!(port, "PostgreSQL not responding — attempting auto-start");

    let pg_ctl = match find_pg_ctl() {
        Some(p) => p,
        None => {
            tracing::warn!(
                "pg_ctl.exe not found in external/postgresql_server/bin/ — \
                 cannot auto-start PostgreSQL. Start it manually or check your project layout."
            );
            return;
        }
    };

    let pg_data = match find_pg_data() {
        Some(p) => p,
        None => {
            tracing::warn!(
                "PostgreSQL data directory (server/pgdata/) not found — \
                 run `Initialize-CimmeriaDatabase` first to set up the database."
            );
            return;
        }
    };

    // Ensure log directory exists
    let log_dir = PathBuf::from("server/logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("postgresql.log");

    tracing::info!(
        pg_ctl = %pg_ctl.display(),
        data_dir = %pg_data.display(),
        port,
        "Starting PostgreSQL"
    );

    let result = std::process::Command::new(&pg_ctl)
        .arg("start")
        .arg("-D")
        .arg(&pg_data)
        .arg("-l")
        .arg(&log_file)
        .arg("-o")
        .arg(format!("-p {}", port))
        .arg("-w") // wait for startup to complete
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                tracing::info!(port, "PostgreSQL started successfully");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::warn!(
                    "pg_ctl start returned non-zero: stdout={}, stderr={}",
                    stdout.trim(),
                    stderr.trim()
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to run pg_ctl: {e}");
        }
    }

    // Wait for PostgreSQL to accept connections (up to 10 seconds)
    for i in 0..20 {
        if pg_is_running(&host, port).await {
            tracing::info!(port, attempts = i + 1, "PostgreSQL is accepting connections");
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::warn!(
        port,
        "PostgreSQL did not start accepting connections within 10 seconds. \
         Check server/logs/postgresql.log for details."
    );
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
