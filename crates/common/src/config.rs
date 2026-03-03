use std::path::Path;

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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            auth_host: "0.0.0.0".to_string(),
            auth_port: 13001,
            logon_port: 8081,
            base_host: "0.0.0.0".to_string(),
            base_external_host: "127.0.0.1".to_string(),
            base_port: 32832,
            cell_host: "0.0.0.0".to_string(),
            cell_port: 50000,
            admin_port: 8443,
            db_connection_string:
                "host=localhost port=5433 user=w-testing password=w-testing dbname=sgw".to_string(),
            protocol_digest: "58AFA196AD3AC4F65CADD99BFF23B799".to_string(),
            developer_mode: true,
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
    fn default_config_developer_mode_on() {
        let config = ServerConfig::default();
        assert!(config.developer_mode);
    }

    #[test]
    fn load_config_returns_default_for_now() {
        let path = Path::new("nonexistent.config");
        let config = load_config(path).unwrap();
        assert_eq!(config.auth_port, 13001);
    }
}
