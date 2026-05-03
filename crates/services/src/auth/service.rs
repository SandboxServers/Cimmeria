//! AuthService lifecycle — construction, shard registration, start/stop.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::{routing::post, Router};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use cimmeria_common::ServerConfig;

use crate::audit::{LoginEvent, LoginEventBuffer};

use super::handlers::{handle_server_selection, handle_user_auth};
use super::{
    AuthError, HandlerState, PendingLogin, ShardInfo, REAPER_INTERVAL, SESSION_TTL, TICKET_TTL,
};

// ── AuthService ──────────────────────────────────────────────────────────────

/// Authentication service managing player login via HTTP/SOAP.
pub struct AuthService {
    /// Internal TCP port for BaseApp registration messages (13001).
    pub listener_addr: SocketAddr,
    /// HTTP port for client SOAP login requests (8081).
    pub logon_addr: SocketAddr,
    /// Whether the HTTP listener is running.
    pub is_running: bool,
    /// Registered BaseApp shards included in Phase 1 responses.
    pub shards: Vec<ShardInfo>,
    /// Pending logins keyed by ticket; shared with the BaseService for Phase 3 validation.
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    developer_mode: bool,
    /// Database connection pool for credential validation.
    db_pool: Option<Arc<PgPool>>,
    /// Login event broadcast channel.
    login_tx: Option<broadcast::Sender<LoginEvent>>,
    /// Login event ring buffer for WebSocket replay.
    login_buffer: Option<LoginEventBuffer>,
}

impl AuthService {
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = SocketAddr::from(([127, 0, 0, 1], config.auth_port));
        let logon_addr = SocketAddr::new(
            config
                .auth_host
                .parse()
                .unwrap_or_else(|_| [0, 0, 0, 0].into()),
            config.logon_port,
        );

        Self {
            listener_addr,
            logon_addr,
            is_running: false,
            shards: Vec::new(),
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
            developer_mode: config.developer_mode,
            db_pool: None,
            login_tx: None,
            login_buffer: None,
        }
    }

    /// Set the database connection pool for credential validation.
    pub fn set_db_pool(&mut self, pool: Arc<PgPool>) {
        self.db_pool = Some(pool);
    }

    /// Set the login event broadcast channel and buffer.
    pub fn set_login_event_tx(
        &mut self,
        tx: broadcast::Sender<LoginEvent>,
        buffer: LoginEventBuffer,
    ) {
        self.login_tx = Some(tx);
        self.login_buffer = Some(buffer);
    }

    /// Register a BaseApp shard.
    ///
    /// Must be called before [`start`] so the shard appears in Phase 1 responses.
    /// Logs a warning and skips registration if a shard with the same name
    /// already exists (matches C++ `ALREADY_REGISTERED` behaviour).
    pub fn register_shard(&mut self, info: ShardInfo) {
        if self.shards.iter().any(|s| s.name == info.name) {
            tracing::warn!(name = %info.name, "Duplicate shard name — skipping registration");
            return;
        }
        tracing::info!(
            name = %info.name, host = %info.host, port = info.port,
            protected = info.protected, "Registering shard"
        );
        self.shards.push(info);
    }

    /// Start the HTTP/SOAP login listener on `logon_addr`.
    ///
    /// Spawns an axum server as a background tokio task. Returns once the
    /// listener is bound; the task runs until the process exits.
    pub async fn start(&mut self) -> Result<(), AuthError> {
        tracing::info!(addr = %self.logon_addr, "Starting auth HTTP listener");
        tracing::trace!(
            shard_count = self.shards.len(),
            developer_mode = self.developer_mode,
            "Auth service config"
        );

        let sessions = Arc::new(Mutex::new(HashMap::new()));
        let pending_logins = Arc::clone(&self.pending_logins);

        // Clone Arcs for the reaper task *before* moving into HandlerState.
        let reaper_sessions = Arc::clone(&sessions);
        let reaper_pending = Arc::clone(&pending_logins);

        let state = Arc::new(HandlerState {
            shards: self.shards.clone(),
            sessions,
            pending_logins,
            developer_mode: self.developer_mode,
            db: self.db_pool.clone(),
            login_tx: self.login_tx.clone(),
            login_buffer: self.login_buffer.clone(),
        });

        let app = Router::new()
            .route("/SGWLogin/UserAuth", post(handle_user_auth))
            .route("/SGWLogin/ServerSelection", post(handle_server_selection))
            .with_state(state);

        tracing::trace!(addr = %self.logon_addr, "Binding TCP listener for auth HTTP");
        let listener = TcpListener::bind(self.logon_addr).await.map_err(|e| {
            tracing::error!(addr = %self.logon_addr, error = %e, "Failed to bind auth TCP listener");
            e
        })?;
        tracing::info!(addr = %listener.local_addr().unwrap(), "Auth HTTP listener bound");

        // Spawn the session/ticket reaper before the HTTP server so it's
        // already running when the first request arrives.
        {
            let sessions = reaper_sessions;
            let pending = reaper_pending;
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(REAPER_INTERVAL).await;
                    let now = std::time::Instant::now();

                    let expired_sessions = {
                        let mut map = sessions.lock().unwrap();
                        let before = map.len();
                        map.retain(|_, s| now.duration_since(s.created) < SESSION_TTL);
                        before - map.len()
                    };
                    let expired_tickets = {
                        let mut map = pending.lock().unwrap();
                        let before = map.len();
                        map.retain(|_, p| now.duration_since(p.created) < TICKET_TTL);
                        before - map.len()
                    };
                    if expired_sessions > 0 || expired_tickets > 0 {
                        tracing::debug!(
                            expired_sessions,
                            expired_tickets,
                            "Reaped expired auth entries"
                        );
                    }
                }
            });
        }

        tracing::trace!("Spawning auth HTTP server task");
        tokio::spawn(async move {
            tracing::trace!("Auth HTTP server task started");
            if let Err(e) = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            {
                tracing::error!("Auth HTTP server error: {e}");
            }
            tracing::trace!("Auth HTTP server task exited");
        });

        self.is_running = true;
        Ok(())
    }

    /// Stop the auth service.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping authentication service");
        self.is_running = false;
        tracing::trace!("Authentication service stopped");
    }

    /// Consume a pending login by ticket.
    ///
    /// Called by the BaseService when a `baseAppLogin` Mercury message arrives.
    /// Returns `None` if the ticket is unknown or has already been consumed.
    pub fn take_pending_login(&self, ticket: &str) -> Option<PendingLogin> {
        self.pending_logins.lock().ok()?.remove(ticket)
    }

    /// Return a cloned `Arc` pointing to the pending-logins map.
    ///
    /// Used by the orchestrator to wire the shared map into `BaseService`
    /// before starting services, so the BaseService can validate Phase 3 tickets.
    pub fn pending_logins_arc(&self) -> Arc<Mutex<HashMap<String, PendingLogin>>> {
        Arc::clone(&self.pending_logins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = AuthService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 13001);
        assert_eq!(svc.logon_addr.port(), 8081);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let mut config = ServerConfig::default();
        config.logon_port = 0; // OS-assigned port to avoid conflicts in tests
        let mut svc = AuthService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }
}
