/// Top-level error type for the Cimmeria server emulator.
///
/// Provides a unified error type that can represent failures from any
/// subsystem. Individual crates may define more specific error types
/// internally but should convert to `CimmeriaError` at API boundaries.
#[derive(Debug, thiserror::Error)]
pub enum CimmeriaError {
    /// Configuration file loading or parsing failure.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network transport or connection failure.
    #[error("Network error: {0}")]
    Network(String),

    /// Mercury protocol encoding/decoding or message handling failure.
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Database query or connection failure.
    #[error("Database error: {0}")]
    Database(String),

    /// Entity system error (creation, lookup, property access, lifecycle).
    #[error("Entity error: {0}")]
    Entity(String),

    /// Authentication or authorization failure.
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Standard I/O error (file access, stream operations).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Mercury encryption/decryption failure (AES-256-CBC + HMAC-MD5).
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// Mercury packet decode failure (malformed header, truncated body).
    #[error("Packet decode error: {0}")]
    PacketDecode(String),

    /// Mercury fragment reassembly failure (missing fragments, index mismatch).
    #[error("Fragment reassembly error: {0}")]
    FragmentReassembly(String),

    /// Mercury channel error (state machine violation, window overflow).
    #[error("Channel error: {0}")]
    Channel(String),

    /// Codec layer error (encode/decode pipeline failure).
    #[error("Codec error: {0}")]
    Codec(String),

    /// Buffer did not contain enough bytes for the expected read.
    #[error("Buffer underflow: needed {needed} bytes, had {available}")]
    BufferUnderflow { needed: usize, available: usize },

    /// Unknown or unsupported Mercury message type identifier.
    #[error("Invalid message ID: {0}")]
    InvalidMessageId(u8),

    /// Operation timed out.
    #[error("Timeout")]
    Timeout,

    /// Catch-all for errors that do not fit another category.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias for `std::result::Result<T, CimmeriaError>`.
pub type Result<T> = std::result::Result<T, CimmeriaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_display() {
        let err = CimmeriaError::Config("missing field".to_string());
        assert_eq!(format!("{}", err), "Configuration error: missing field");
    }

    #[test]
    fn network_error_display() {
        let err = CimmeriaError::Network("connection refused".to_string());
        assert_eq!(format!("{}", err), "Network error: connection refused");
    }

    #[test]
    fn protocol_error_display() {
        let err = CimmeriaError::Protocol("invalid message id".to_string());
        assert_eq!(format!("{}", err), "Protocol error: invalid message id");
    }

    #[test]
    fn database_error_display() {
        let err = CimmeriaError::Database("connection timeout".to_string());
        assert_eq!(format!("{}", err), "Database error: connection timeout");
    }

    #[test]
    fn entity_error_display() {
        let err = CimmeriaError::Entity("entity not found".to_string());
        assert_eq!(format!("{}", err), "Entity error: entity not found");
    }

    #[test]
    fn auth_error_display() {
        let err = CimmeriaError::Auth("invalid credentials".to_string());
        assert_eq!(
            format!("{}", err),
            "Authentication error: invalid credentials"
        );
    }

    #[test]
    fn io_error_from_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: CimmeriaError = io_err.into();
        assert!(format!("{}", err).contains("file not found"));
    }
}
