//! AuthService lifecycle — construction, shard registration, start/stop.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::serve::{Listener as _, ListenerExt as _};
use axum::{routing::post, Router};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use cimmeria_common::ServerConfig;

use crate::audit::{LoginEvent, LoginEventBuffer};

use super::handlers::{handle_server_selection, handle_user_auth};
use super::tls::{hsts_layer, TlsCertStore, TlsListener};
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
    /// HTTPS port for TLS-terminated SOAP login requests. The TLS listener runs
    /// in parallel with the HTTP listener during the transition window. Only
    /// started when `tls_cert_path` + `tls_key_path` are both set.
    pub tls_addr: SocketAddr,
    /// PEM cert chain path; `None` disables the TLS listener.
    tls_cert_path: Option<PathBuf>,
    /// PEM private key path; `None` disables the TLS listener.
    tls_key_path: Option<PathBuf>,
    /// Live cert store (set once `start` loads the cert). Held so `reload_certs`
    /// can hot-swap the cert without restarting the listener.
    cert_store: Option<TlsCertStore>,
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
        let auth_ip = config
            .auth_host
            .parse()
            .unwrap_or_else(|_| [0, 0, 0, 0].into());
        let logon_addr = SocketAddr::new(auth_ip, config.logon_port);
        let tls_addr = SocketAddr::new(auth_ip, config.auth_tls_port);

        Self {
            listener_addr,
            logon_addr,
            tls_addr,
            tls_cert_path: config.auth_tls_cert_path.clone(),
            tls_key_path: config.auth_tls_key_path.clone(),
            cert_store: None,
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

        // HSTS is layered on the shared Router so *both* listeners stamp
        // `Strict-Transport-Security` on every response. (Per RFC 6797 a UA
        // only *honours* it when received over HTTPS; stamping it on the HTTP
        // path too is harmless and simplifies the shared-Router wiring.)
        let app = Router::new()
            .route("/SGWLogin/UserAuth", post(handle_user_auth))
            .route("/SGWLogin/ServerSelection", post(handle_server_selection))
            .layer(axum::middleware::from_fn(hsts_layer))
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

        // ── TLS listener (parallel HTTPS) ───────────────────────────────────
        // Start the HTTPS listener *before* moving `app` into the HTTP server
        // task — both listeners serve the same Router (cloned cheaply; a Router
        // clone shares the inner service). The TLS listener only starts when
        // both cert and key paths are configured; otherwise the server is
        // HTTP-only and this is a no-op.
        if let (Some(cert), Some(key)) = (&self.tls_cert_path, &self.tls_key_path) {
            tracing::info!(addr = %self.tls_addr, cert = %cert.display(), "Starting auth TLS listener");
            let store = TlsCertStore::load(cert, key).map_err(|e| {
                tracing::error!(error = %e, "Failed to load auth TLS certificate");
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            })?;
            // Stash the store so reload_certs() can hot-swap the cert later.
            self.cert_store = Some(store.clone());

            let tls_listener = TlsListener::bind(self.tls_addr, store).await.map_err(|e| {
                tracing::error!(addr = %self.tls_addr, error = %e, "Failed to bind auth TLS listener");
                e
            })?;
            // Fall back to the configured bind addr rather than panicking —
            // `local_addr()` can fail on an unusual socket state and we already
            // know where we bound.
            tracing::info!(addr = %tls_listener.local_addr().unwrap_or(self.tls_addr), "Auth TLS listener bound");

            // `tap_io` (a no-op here) routes the custom listener through axum's
            // blanket `Connected<IncomingStream<TapIo<L,F>>> for L::Addr` impl,
            // which is what makes `into_make_service_with_connect_info::
            // <SocketAddr>()` legal over a non-`TcpListener` listener. The auth
            // handlers extract `ConnectInfo<SocketAddr>`, so the HTTPS path must
            // surface the peer address exactly like the HTTP path.
            let tls_listener = tls_listener.tap_io(|_io| {});

            let tls_app = app.clone();
            tokio::spawn(async move {
                tracing::trace!("Auth TLS server task started");
                if let Err(e) = axum::serve(
                    tls_listener,
                    tls_app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                {
                    tracing::error!("Auth TLS server error: {e}");
                }
                tracing::trace!("Auth TLS server task exited");
            });
        } else {
            tracing::debug!("Auth TLS listener not configured (cert/key paths unset); HTTP-only");
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

    /// Hot-reload the auth TLS certificate from the configured cert/key paths.
    ///
    /// Re-reads the PEM files and atomically swaps the live `rustls::ServerConfig`
    /// so new connections pick up the rotated cert without restarting the
    /// listener. In-flight connections keep the config they handshook with.
    ///
    /// Returns `AuthError::NotRunning` if TLS was never configured/started. An
    /// mtime/SIGHUP watcher that calls this on a schedule is a documented
    /// follow-up — this is the reload *seam*.
    pub fn reload_certs(&self) -> Result<(), AuthError> {
        match &self.cert_store {
            Some(store) => store.reload().map_err(|e| {
                tracing::error!(error = %e, "auth TLS cert reload failed");
                AuthError::Network(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    e.to_string(),
                ))
            }),
            None => {
                tracing::warn!("reload_certs called but auth TLS is not configured");
                Err(AuthError::NotRunning)
            }
        }
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
        let config = ServerConfig {
            logon_port: 0, // OS-assigned port to avoid conflicts in tests
            ..ServerConfig::default()
        };
        let mut svc = AuthService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }
}
