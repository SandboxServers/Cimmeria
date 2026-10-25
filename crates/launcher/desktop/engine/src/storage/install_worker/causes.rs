//! Closed causes for a destination the install could not use (PD-15). Each
//! maps an error to a snake_case code; no path or error text ever leaves here (R5).
use crate::storage::StorageError;
use std::io::ErrorKind;

/// The closed cause of a local I/O error, from its kind only.
pub(crate) fn io_cause(error: &std::io::Error) -> &'static str {
    match error.kind() {
        ErrorKind::PermissionDenied => "permission_denied",
        ErrorKind::NotFound => "not_found",
        ErrorKind::AlreadyExists => "already_exists",
        ErrorKind::DirectoryNotEmpty => "directory_not_empty",
        ErrorKind::ReadOnlyFilesystem => "read_only",
        ErrorKind::StorageFull => "storage_full",
        _ => "other",
    }
}

/// The closed cause of a failed ownership claim. The claim keeps no I/O error,
/// so this names the storage error instead.
pub(crate) fn storage_cause(error: &StorageError) -> &'static str {
    match error {
        StorageError::InvalidDirectory => "invalid_directory",
        StorageError::UnsafeFile => "unsafe_file",
        StorageError::InUse => "in_use",
        StorageError::Corrupt => "corrupt",
        StorageError::Io => "io",
        _ => "other",
    }
}

#[cfg(test)]
#[path = "causes_tests.rs"]
mod tests;
