//! Closed snake_case codes for engine errors in launcher events (C6). Each code
//! is the serde name of its variant, so events and IPC agree on one spelling.
use crate::{ContractError, IntentError, StorageError};

impl ContractError {
    /// The closed code for this error; the same text serde writes.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnsupportedSchema => "unsupported_schema",
            Self::InvalidRevision => "invalid_revision",
            Self::StaleRevision => "stale_revision",
            Self::Busy => "busy",
            Self::IdentityConflict => "identity_conflict",
            Self::UnknownOperation => "unknown_operation",
            Self::InvalidTransition => "invalid_transition",
            Self::PersistenceFailed => "persistence_failed",
            Self::PersistenceUncertain => "persistence_uncertain",
        }
    }
}

impl StorageError {
    /// The closed code for this error; the same text serde writes.
    pub fn code(self) -> &'static str {
        match self {
            Self::InUse => "in_use",
            Self::Io => "io",
            Self::Corrupt => "corrupt",
            Self::UnsupportedSchema => "unsupported_schema",
            Self::TooLarge => "too_large",
            Self::UnsafeFile => "unsafe_file",
            Self::InvalidDirectory => "invalid_directory",
            Self::StaleRevision => "stale_revision",
            Self::Busy => "busy",
            Self::PersistenceUncertain => "persistence_uncertain",
            Self::InvalidServerList => "invalid_server_list",
            Self::InvalidDisplayOptions => "invalid_display_options",
        }
    }
}

impl IntentError {
    /// The code of the error inside; `launcher_too_old` for a version refusal.
    pub fn code(self) -> &'static str {
        match self {
            Self::LauncherTooOld => "launcher_too_old",
            Self::Storage(error) => error.code(),
            Self::Operation(error) => error.code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_the_serde_names() {
        let contract = [
            ContractError::UnsupportedSchema,
            ContractError::InvalidRevision,
            ContractError::StaleRevision,
            ContractError::Busy,
            ContractError::IdentityConflict,
            ContractError::UnknownOperation,
            ContractError::InvalidTransition,
            ContractError::PersistenceFailed,
            ContractError::PersistenceUncertain,
        ];
        for error in contract {
            assert_eq!(
                serde_json::to_value(error).unwrap(),
                serde_json::Value::from(error.code())
            );
        }
        let storage = [
            StorageError::InUse,
            StorageError::Io,
            StorageError::Corrupt,
            StorageError::UnsupportedSchema,
            StorageError::TooLarge,
            StorageError::UnsafeFile,
            StorageError::InvalidDirectory,
            StorageError::StaleRevision,
            StorageError::Busy,
            StorageError::PersistenceUncertain,
            StorageError::InvalidServerList,
            StorageError::InvalidDisplayOptions,
        ];
        for error in storage {
            assert_eq!(
                serde_json::to_value(error).unwrap(),
                serde_json::Value::from(error.code())
            );
        }
    }

    #[test]
    fn intent_errors_use_the_inner_code() {
        assert_eq!(IntentError::LauncherTooOld.code(), "launcher_too_old");
        assert_eq!(
            IntentError::Storage(StorageError::TooLarge).code(),
            "too_large"
        );
        assert_eq!(IntentError::Operation(ContractError::Busy).code(), "busy");
    }
}
