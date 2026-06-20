use std::path::{Path, PathBuf};

/// Server configuration loaded from XML config files.
///
/// Mirrors the configuration values spread across the three C++ config files:
/// - `config/AuthenticationService.config` (auth ports, protocol digest)
/// - `config/BaseService.config` (shard settings, grid, tick rates, console)
/// - `config/CellService.config` (cell id, baseapp connection, caching)
///
/// Default values match the existing C++ config file defaults.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Authentication server bind address.
    pub auth_host: String,

    /// Port the AuthenticationServer listens on for BaseApp connections.
    /// Default: 13001 (from `AuthenticationService.config:base_service_port`)
    pub auth_port: u16,

    /// HTTP port the auth server listens on for client SOAP login requests.
    /// Default: 8081 (from `AuthenticationService.config:logon_service_port`)
    pub logon_port: u16,

    /// HTTPS port the auth server listens on for TLS-terminated SOAP login
    /// requests. The TLS listener runs **in parallel** with the
    /// plain-HTTP listener on `logon_port` during the transition window, serving
    /// the same axum `Router`. It only starts when **both** `auth_tls_cert_path`
    /// and `auth_tls_key_path` are set; otherwise the server is HTTP-only.
    /// Default: 13443 (TLS off by default, no cert/key configured).
    pub auth_tls_port: u16,

    /// Path to the PEM-encoded TLS certificate chain (leaf first) for the auth
    /// HTTPS listener. `None` disables TLS. Both this and `auth_tls_key_path`
    /// must be set for the TLS listener to start.
    pub auth_tls_cert_path: Option<PathBuf>,

    /// Path to the PEM-encoded TLS private key (PKCS#8, PKCS#1/RSA, or SEC1/EC)
    /// for the auth HTTPS listener. `None` disables TLS.
    pub auth_tls_key_path: Option<PathBuf>,

    /// BaseApp bind address (the interface the UDP socket is bound to).
    /// Use `0.0.0.0` to listen on all interfaces.
    /// Default: `0.0.0.0`
    pub base_host: String,

    /// External BaseApp address advertised to game clients in the SOAP login
    /// response. Clients connect to this address, not the bind address.
    /// Maps to `BaseService.config:shard_external_address`.
    /// Default: `127.0.0.1` (suitable for local dev)
    pub base_external_host: String,

    /// Port the BaseApp shard listens on for client connections.
    /// Default: 32832 (from `BaseService.config:shard_port`)
    pub base_port: u16,

    /// CellApp bind address.
    pub cell_host: String,

    /// Port the CellApp uses for BaseApp communication.
    /// Default: 50000
    pub cell_port: u16,

    /// Administrative API port (REST/WebSocket).
    /// Default: 8443
    pub admin_port: u16,

    /// PostgreSQL connection string.
    /// Default matches the test credentials in the existing config files.
    pub db_connection_string: String,

    /// Protocol digest sent in the auth login response.
    /// Must match the value compiled into the game client.
    /// Maps to `AuthenticationService.config:protocol_digest`.
    pub protocol_digest: String,

    /// Enable developer mode (relaxed auth, elevated logging, multi-login).
    /// Default: true in dev builds (from `BaseService.config:developer_mode`)
    pub developer_mode: bool,

    /// Port the minigame SmartFoxServer TCP listener binds to.
    /// Default: 30000
    pub minigame_port: u16,

    /// Mercury packet-encryption wire version applied to every session.
    ///
    /// - `1` = legacy v1 (AES-256-CBC + HMAC-MD5, byte-identical to the
    ///   unmodified game client).
    /// - `2` = modernized v2 (per-packet random IV, HKDF-split keys,
    ///   truncated HMAC-SHA256).
    ///
    /// Default `1` for the duration of the client-transition window: the
    /// stock client only understands v1, so producing v2 frames would break
    /// every unpatched connection. Selection is **server-wide** — there is no
    /// per-client negotiation yet; that arrives with the client patch. Any
    /// value other than `1` or `2` falls back to v1 with a warning (see
    /// [`mercury_encryption_version`](ServerConfig::mercury_encryption_version)).
    pub mercury_encryption_version: u8,

    /// Cadence, in seconds, for server-initiated Mercury session-key rotation.
    ///
    /// When non-zero, a v2 session periodically mints a fresh 32-byte session
    /// key, hands it to the peer encrypted under the current key, and both
    /// sides switch. `0` disables rotation. Default `3600` (one hour).
    ///
    /// Rotation is **only** performed for v2 sessions: the stock client speaks
    /// v1 and cannot process a rotation control message, so the rotation
    /// scheduler is hard-gated to v2 regardless of this value. With the
    /// default v1 server-wide version, this setting has no effect until v2 is
    /// selected.
    pub mercury_key_rotation_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            auth_host: "0.0.0.0".to_string(),
            auth_port: 13001,
            logon_port: 8081,
            // TLS is opt-in: the listener stays down until an operator points
            // both cert and key paths at real PEM files. 13443 is a stable
            // default so the dual-stack window has a known HTTPS port.
            auth_tls_port: 13443,
            auth_tls_cert_path: None,
            auth_tls_key_path: None,
            base_host: "0.0.0.0".to_string(),
            base_external_host: "127.0.0.1".to_string(),
            base_port: 32832,
            cell_host: "0.0.0.0".to_string(),
            cell_port: 50000,
            admin_port: 8443,
            db_connection_string:
                "host=localhost port=5433 user=w-testing password=w-testing dbname=sgw".to_string(),
            protocol_digest: "58AFA196AD3AC4F65CADD99BFF23B799".to_string(),
            developer_mode: false,
            minigame_port: 30000,
            // v1 is the only version unpatched clients understand; the default
            // stays v1 until the client patch lands. Operators opt into v2
            // explicitly (and only for patched clients / test harnesses).
            mercury_encryption_version: 1,
            // One hour. Only takes effect on v2 sessions; v1 sessions never
            // rotate regardless of this value.
            mercury_key_rotation_secs: 3600,
        }
    }
}

/// Errors that can occur during configuration loading.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse config XML: {0}")]
    Parse(String),

    #[error("Missing required config field: {0}")]
    MissingField(String),

    #[error("Invalid config value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },
}

/// Load server configuration from an XML config file.
///
/// # Arguments
///
/// * `path` - Path to the XML configuration file (e.g., `config/BaseService.config`)
///
/// # Returns
///
/// A `ServerConfig` populated from the file, or a `ConfigError` if loading fails.
///
/// # Current Status
///
/// This is a stub that returns defaults. Full XML parsing will be implemented
/// when the quick-xml integration is wired up.
pub fn load_config(_path: &Path) -> Result<ServerConfig, ConfigError> {
    // TODO: Implement XML config parsing with quick-xml.
    // The config files use a flat <config> root element with child elements
    // for each setting (see config/BaseService.config for the full schema).
    tracing::warn!("load_config is a stub; returning default configuration");
    Ok(ServerConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_ports() {
        let config = ServerConfig::default();
        assert_eq!(config.auth_port, 13001);
        assert_eq!(config.logon_port, 8081);
        assert_eq!(config.base_port, 32832);
        assert_eq!(config.cell_port, 50000);
        assert_eq!(config.admin_port, 8443);
    }

    #[test]
    fn default_config_developer_mode_off() {
        let config = ServerConfig::default();
        assert!(!config.developer_mode);
    }

    #[test]
    fn default_config_mercury_encryption_is_v1() {
        // The default MUST stay v1 — the stock client only understands v1, so
        // flipping this default would break every unpatched connection. This
        // guard fails loudly if the default is ever changed to 2.
        let config = ServerConfig::default();
        assert_eq!(config.mercury_encryption_version, 1);
    }

    #[test]
    fn default_config_tls_off_with_stable_port() {
        // TLS must be disabled by default — both paths None means the auth
        // server stays HTTP-only. The port is set regardless so enabling TLS
        // is a two-path change, not a port-hunt.
        let config = ServerConfig::default();
        assert_eq!(config.auth_tls_port, 13443);
        assert!(config.auth_tls_cert_path.is_none());
        assert!(config.auth_tls_key_path.is_none());
    }

    #[test]
    fn default_config_key_rotation_is_one_hour() {
        // Default cadence is one hour. It only engages on v2 sessions; with the
        // default v1 version this value is inert, but it must default to the
        // documented hour so flipping to v2 gives the intended cadence.
        let config = ServerConfig::default();
        assert_eq!(config.mercury_key_rotation_secs, 3600);
    }

    #[test]
    fn load_config_returns_default_for_now() {
        let path = Path::new("nonexistent.config");
        let config = load_config(path).unwrap();
        assert_eq!(config.auth_port, 13001);
    }
}
