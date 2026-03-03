//! Authentication service.
//!
//! Handles player login, account validation, and session creation. Mirrors the
//! C++ AuthenticationServer that listens on port 13001 and mediates between
//! incoming client connections and the BaseApp shard.

use std::net::SocketAddr;

use cimmeria_common::{ServerConfig, SessionId};

/// Errors specific to the authentication service.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials for user '{0}'")]
    InvalidCredentials(String),

    #[error("Account '{0}' is locked")]
    AccountLocked(String),

    #[error("Session limit exceeded")]
    SessionLimitExceeded,

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

/// Authentication service managing player login and session creation.
///
/// In the original C++ architecture, this was the `AuthenticationServer`
/// process listening on `base_service_port` (default 13001). It validated
/// credentials against the `sgw.account` table and issued session keys
/// for BaseApp connections.
pub struct AuthService {
    /// Address the authentication listener binds to.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently accepting connections.
    pub is_running: bool,
}

impl AuthService {
    /// Create a new authentication service from server configuration.
    ///
    /// Reads `auth_host` and `auth_port` from the config to determine the
    /// listener bind address.
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = format!("{}:{}", config.auth_host, config.auth_port)
            .parse()
            .unwrap_or_else(|_| {
                SocketAddr::from(([127, 0, 0, 1], config.auth_port))
            });

        Self {
            listener_addr,
            is_running: false,
        }
    }

    /// Start the authentication service.
    ///
    /// Binds the TCP listener and begins accepting client connections.
    /// Each connection goes through the login handshake documented in
    /// `docs/protocol/login-handshake.md`.
    pub async fn start(&mut self) -> Result<(), AuthError> {
        tracing::info!(addr = %self.listener_addr, "Starting authentication service");
        // TODO: Bind TCP listener, start accept loop
        self.is_running = true;
        Ok(())
    }

    /// Stop the authentication service gracefully.
    ///
    /// Closes the listener and drains active connections.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping authentication service");
        self.is_running = false;
        // TODO: Close listener, drain connections
    }

    /// Authenticate a player and return a session identifier.
    ///
    /// In the original protocol, this validates credentials against the
    /// `sgw.account` table (MD5-hashed passwords) and creates a session
    /// entry for the BaseApp to look up on subsequent connection.
    pub async fn handle_login(
        &self,
        username: &str,
        _password: &str,
    ) -> Result<SessionId, AuthError> {
        if !self.is_running {
            return Err(AuthError::NotRunning);
        }

        tracing::debug!(username, "Processing login request");

        // TODO: Validate credentials against database
        // TODO: Create session entry
        // TODO: Return session ID for BaseApp handoff

        // Stub: return a placeholder session ID
        let _ = username;
        Ok(SessionId(1))
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
    }

    #[tokio::test]
    async fn start_sets_running() {
        let config = ServerConfig::default();
        let mut svc = AuthService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }

    #[tokio::test]
    async fn login_fails_when_not_running() {
        let config = ServerConfig::default();
        let svc = AuthService::new(&config);
        let result = svc.handle_login("test", "pass").await;
        assert!(result.is_err());
    }
}
