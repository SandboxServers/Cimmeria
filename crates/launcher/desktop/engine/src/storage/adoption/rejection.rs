//! Why an adoption was refused (launcher-telemetry C14). A refusal records one `Rejection` in the scope it runs in and logs it on the `adoption` target;
//! only closed codes, sizes, digests and the folder-relative entry are kept.
use super::*;
use std::cell::RefCell;

/// Closed list; the frontend text table (C17) and the shell keep the same codes.
#[rustfmt::skip]
pub const REJECTION_CHECKS: &[&str] = &[
    // source and folder (LT-28, LT-29)
    "not_a_game_folder", "folder_not_writable", "folder_in_use", "folder_overlaps_launcher",
    "folder_unreadable", "import_changed", "folder_changed_during_check", "folder_too_large",
    "folder_too_deep", "path_not_canonical", "unsupported_catalog", "unsupported_configuration",
    "stale_revision", "busy", "destination_exists", "destination_overlaps", "launcher_too_old",
    "identity_conflict", "comparison_failed", "exe_at_root",
    // verification and artifacts (LT-30, LT-31)
    "no_reusable_files", "seed_mismatch", "setup_rule_invalid", "file_unclassified",
    "artifact_missing", "artifact_hash_mismatch", "artifact_size_mismatch",
    "download_http_status", "download_failed", "local_seed_invalid", "local_artifact_mismatch",
    "manifest_digest_malformed", "patch_replay_mismatch", "patch_archive_unsupported",
    "patch_input_unexpected", "patch_layout_unsupported",
    // in place, reference, worker, consent (LT-32)
    "leftover_launcher_files", "unsupported_layout", "invalid_directory", "reference_zip_invalid",
    "worker_stopped", "blocking_files",
    // shell (LT-33)
    "import_required", "state_corrupt", "release_unavailable", "platform_unavailable",
    "recovery_required",
];

/// Longest kept value of one field, in bytes.
const MAX_FIELD: usize = 120;

thread_local! {
    /// The record of the innermost open scope on this thread.
    static SLOT: RefCell<Option<Rejection>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Rejection {
    /// One of `REJECTION_CHECKS` (debug_assert in new()).
    pub check: &'static str,
    /// R5a: folder-relative path or a manifest label.
    pub entry: Option<String>,
    /// `digest()` form, a count, or a short closed phrase.
    pub expected: Option<String>,
    pub found: Option<String>,
    pub http_status: Option<u16>,
}

impl Rejection {
    /// A rejection of `check`, which must be listed in `REJECTION_CHECKS`.
    #[rustfmt::skip]
    pub fn new(check: &'static str) -> Self {
        debug_assert!(REJECTION_CHECKS.contains(&check), "unlisted rejection check {check}");
        Self { check, entry: None, expected: None, found: None, http_status: None }
    }
    pub fn entry(mut self, e: impl Into<String>) -> Self {
        self.entry = Some(keep_tail(e.into()));
        self
    }
    pub fn expected(mut self, e: impl Into<String>) -> Self {
        self.expected = Some(keep_tail(e.into()));
        self
    }
    pub fn found(mut self, f: impl Into<String>) -> Self {
        self.found = Some(keep_tail(f.into()));
        self
    }
    pub fn http_status(mut self, s: u16) -> Self {
        self.http_status = Some(s);
        self
    }
}

/// Keeps at most `MAX_FIELD` bytes of `text`, from its end, on a char boundary.
fn keep_tail(text: String) -> String {
    if text.len() <= MAX_FIELD {
        return text;
    }
    let mut start = text.len() - MAX_FIELD;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_string()
}

/// "size 4096, sha256 0123456789ab" (the first 12 hex digits, lower case).
pub fn digest(size: u64, sha256_hex: &str) -> String {
    let prefix: String = sha256_hex.to_ascii_lowercase().chars().take(12).collect();
    format!("size {size}, sha256 {prefix}")
}

/// `path` relative to `root`, '/'-separated, "." for the root. A path outside
/// `root` yields only its last component, never the full path.
pub fn relative(root: &std::path::Path, path: &std::path::Path) -> String {
    let Ok(rest) = path.strip_prefix(root) else {
        return path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    };
    let parts: Vec<String> = rest
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
/// The snake_case code of `error`, identical to the shell's `AdoptionError` serde name.
#[rustfmt::skip]
pub fn code(error: &Error) -> &'static str {
    match error {
        Error::Storage(StorageError::InUse) => "in_use",
        Error::Storage(StorageError::Busy) => "busy",
        Error::Storage(StorageError::Io) => "io",
        Error::Storage(StorageError::Corrupt | StorageError::TooLarge | StorageError::InvalidServerList) => "corrupt_state",
        Error::Storage(StorageError::UnsafeFile) => "unsafe_file",
        Error::Storage(StorageError::UnsupportedSchema) => "unsupported_schema",
        Error::Storage(StorageError::InvalidDirectory) => "invalid_directory",
        Error::Storage(StorageError::StaleRevision) => "stale_revision",
        Error::Storage(StorageError::PersistenceUncertain) => "persistence_uncertain",
        Error::Operation(ContractError::UnsupportedSchema) => "unsupported_schema",
        Error::Operation(ContractError::InvalidRevision | ContractError::StaleRevision) => "stale_revision",
        Error::Operation(ContractError::Busy) => "busy",
        Error::Operation(ContractError::IdentityConflict | ContractError::UnknownOperation) => "identity_conflict",
        Error::Operation(ContractError::InvalidTransition) => "recovery_required",
        Error::Operation(ContractError::PersistenceFailed) => "io",
        Error::Operation(ContractError::PersistenceUncertain) => "persistence_uncertain",
        Error::SourceChanged => "source_changed",
        Error::ConsentRequired => "consent_required",
        Error::NoReusableFiles => "no_reusable_files",
        Error::UnsupportedCatalog => "unsupported_catalog",
        Error::UnsupportedArchive => "unsupported_archive",
        Error::UnsupportedConfiguration => "unsupported_configuration",
        Error::InvalidArtifact => "invalid_artifact",
        Error::Network => "network",
        Error::Cancelled => "cancelled",
        Error::LauncherTooOld => "launcher_too_old",
        Error::VerificationFailed => "verification_failed",
        Error::UnsupportedLayout => "unsupported_layout",
        Error::GameRunning => "game_running",
        Error::DuplicateReleaseFile => "duplicate_release_file",
        Error::LeftoverLauncherFiles => "leftover_launcher_files",
        Error::FolderNotWritable => "folder_not_writable",
        Error::NotAGameFolder => "not_a_game_folder",
        Error::BackupConflict => "backup_conflict",
        Error::UnsupportedMode => "unsupported_mode",
        Error::Unsafe(_) => "unsafe_file",
    }
}
/// Records `r` as the scope's rejection if none is recorded yet (the first is the
/// cause, later ones are consequences), WARN-logs it, and returns `error` unchanged.
pub fn reject(error: Error, r: Rejection) -> Error {
    tracing::warn!(
        target: "adoption",
        event = "adoption_rejected",
        check = r.check,
        code = code(&error),
        entry = r.entry.as_deref(),
        expected = r.expected.as_deref(),
        found = r.found.as_deref(),
        http_status = r.http_status,
        "adoption: check {} failed: expected {}, found {}; the folder was not adopted",
        r.check,
        r.expected.as_deref().unwrap_or("none"),
        r.found.as_deref().unwrap_or("none"),
    );
    SLOT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(r);
        }
    });
    error
}

/// Restores the enclosing scope's slot when the scope ends, also on a panic.
struct Restore(Option<Rejection>);

impl Drop for Restore {
    fn drop(&mut self) {
        let outer = self.0.take();
        SLOT.with(|slot| *slot.borrow_mut() = outer);
    }
}

/// Runs `f` with a fresh slot and returns what it recorded. Nested scopes get
/// their own slot and restore the outer one on exit (used by the index fallback).
pub fn scope<R>(f: impl FnOnce() -> R) -> (R, Option<Rejection>) {
    let _restore = Restore(SLOT.with(|slot| slot.borrow_mut().take()));
    let value = f();
    (value, SLOT.with(|slot| slot.borrow_mut().take()))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
    use tracing::Level;

    const SHA: &str = "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn digest_keeps_the_first_twelve_lower_case_digits() {
        assert_eq!(digest(4096, SHA), "size 4096, sha256 0123456789ab");
    }

    #[test]
    fn relative_is_slash_separated_and_never_the_full_path() {
        let root = std::path::Path::new("base");
        assert_eq!(relative(root, root), ".");
        let inside = std::path::Path::new("base/sub/file.bin");
        assert_eq!(relative(root, inside), "sub/file.bin");
        let outside = std::path::Path::new("elsewhere/file.bin");
        assert_eq!(relative(root, outside), "file.bin");
    }

    #[test]
    #[rustfmt::skip]
    fn code_is_the_shell_name_for_every_variant() {
        let cases: Vec<(Error, &str)> = vec![
            (StorageError::InUse.into(), "in_use"), (StorageError::Busy.into(), "busy"),
            (StorageError::Io.into(), "io"), (StorageError::Corrupt.into(), "corrupt_state"),
            (StorageError::TooLarge.into(), "corrupt_state"), (StorageError::InvalidServerList.into(), "corrupt_state"),
            (StorageError::UnsafeFile.into(), "unsafe_file"), (StorageError::UnsupportedSchema.into(), "unsupported_schema"),
            (StorageError::InvalidDirectory.into(), "invalid_directory"), (StorageError::StaleRevision.into(), "stale_revision"),
            (StorageError::PersistenceUncertain.into(), "persistence_uncertain"), (ContractError::UnsupportedSchema.into(), "unsupported_schema"),
            (ContractError::InvalidRevision.into(), "stale_revision"), (ContractError::StaleRevision.into(), "stale_revision"),
            (ContractError::Busy.into(), "busy"), (ContractError::IdentityConflict.into(), "identity_conflict"),
            (ContractError::UnknownOperation.into(), "identity_conflict"), (ContractError::InvalidTransition.into(), "recovery_required"),
            (ContractError::PersistenceFailed.into(), "io"), (ContractError::PersistenceUncertain.into(), "persistence_uncertain"),
            (Error::SourceChanged, "source_changed"), (Error::ConsentRequired, "consent_required"),
            (Error::NoReusableFiles, "no_reusable_files"), (Error::UnsupportedCatalog, "unsupported_catalog"),
            (Error::UnsupportedArchive, "unsupported_archive"), (Error::UnsupportedConfiguration, "unsupported_configuration"),
            (Error::InvalidArtifact, "invalid_artifact"), (Error::Network, "network"),
            (Error::Cancelled, "cancelled"), (Error::LauncherTooOld, "launcher_too_old"),
            (Error::VerificationFailed, "verification_failed"), (Error::UnsupportedLayout, "unsupported_layout"),
            (Error::GameRunning, "game_running"), (Error::DuplicateReleaseFile, "duplicate_release_file"),
            (Error::LeftoverLauncherFiles, "leftover_launcher_files"), (Error::FolderNotWritable, "folder_not_writable"),
            (Error::NotAGameFolder, "not_a_game_folder"), (Error::BackupConflict, "backup_conflict"),
            (Error::UnsupportedMode, "unsupported_mode"), (Error::Unsafe(Box::new(UnsafeReport { entries: Vec::new(), total: 0 })), "unsafe_file"),
        ];        for (error, expected) in cases {
            assert_eq!(code(&error), expected);
        }
    }

    #[test]
    #[should_panic(expected = "unlisted rejection check")]
    fn an_unlisted_check_is_refused() {
        let _ = Rejection::new("not_in_the_list");
    }

    #[test]
    fn the_first_rejection_in_a_scope_wins() {
        let (_, first) = scope(|| {
            reject(Error::Network, Rejection::new("download_failed"));
            reject(Error::Cancelled, Rejection::new("busy"));
        });
        assert_eq!(first.map(|r| r.check), Some("download_failed"));
    }

    #[test]
    fn a_nested_scope_keeps_its_own_record() {
        let (outer_error, outer) = scope(|| {
            reject(Error::Network, Rejection::new("download_failed"));
            let (_, nested) = scope(|| reject(Error::Cancelled, Rejection::new("busy")));
            assert_eq!(nested.map(|r| r.check), Some("busy"));
            Error::Network
        });
        assert_eq!(outer_error, Error::Network);
        assert_eq!(outer.map(|r| r.check), Some("download_failed"));
    }

    #[test]
    #[rustfmt::skip]
    fn reject_logs_a_closed_warn_row() {
        let (_, logs) = capture(|| reject(Error::Network, Rejection::new("download_failed").entry("desktop-copy").found(digest(1, SHA))));
        let row = logs.iter().find(|row| row.field("event") == Some("adoption_rejected")).expect("an adoption_rejected row");
        assert_eq!(row.target, "adoption");
        assert_eq!(row.level, Level::WARN);
        assert_eq!(row.field("check"), Some("download_failed"));
        assert_eq!(row.field("code"), Some("network"));
        assert_eq!(row.field("entry"), Some("desktop-copy"));
        assert_eq!(row.field("expected"), None);
        assert_eq!(row.field("http_status"), None);
    }

    #[test]
    #[rustfmt::skip]
    fn a_copy_into_an_existing_destination_records_destination_exists() {
        let f = super::super::tests::Fixture::new();
        std::fs::create_dir(f.destination()).unwrap();
        let (result, record) = scope(|| super::super::preview_using(f.state.clone(), f.request(), CancellationToken::new(), crate::install_progress::ProgressSink::latest().0, Backend::Native, None));
        assert_eq!(result.err(), Some(Error::Storage(StorageError::InvalidDirectory)));
        let record = record.expect("a recorded rejection");
        assert_eq!(record.check, "destination_exists");
        assert_eq!(record.entry.as_deref(), Some("desktop-copy"));
    }
}
