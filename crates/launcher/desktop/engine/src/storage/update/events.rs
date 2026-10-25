//! Engine update events (launcher-telemetry LT-13; C5, C6). One info row per staged,
//! committed, recovered, discarded or abandoned update, and a warn row when an
//! operation cannot be marked uncertain. Only closed codes and counts are written (R5).
use super::preparation::Failure;
use crate::failure::FailureCode;
use crate::operation_numbers;
use crate::operations::ContractError;
use crate::storage::{IntentError, StorageError};
use serde::Serialize;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

/// The serde code of `value` (snake_case), or `"other"` when it has none.
fn code<T: Serialize>(value: T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "other".to_owned(),
    }
}

/// The closed code of an intent error. Payload variants name their inner code.
fn intent_code(error: &IntentError) -> String {
    match error {
        IntentError::LauncherTooOld => "launcher_too_old".to_owned(),
        IntentError::Storage(storage) => code(storage),
        IntentError::Operation(contract) => code(contract),
    }
}

/// The closed code of a preparation failure.
fn failure_code(failure: &Failure) -> &'static str {
    match failure {
        Failure::Cancelled => "cancelled",
        Failure::Failed => "failed",
        Failure::ReconciliationRequired => "reconciliation_required",
    }
}

/// `ok`, or the closed code of the error.
fn outcome_of<E>(result: &Result<(), E>, code_of: impl Fn(&E) -> String) -> String {
    match result {
        Ok(()) => "ok".to_owned(),
        Err(error) => code_of(error),
    }
}

/// Regular files and their total length under `root`. Symlinks are not followed,
/// and the depth bound keeps a malformed tree from recursing without end.
pub(super) fn tree_size(root: &Path, depth: usize) -> (u64, u64) {
    if depth > 64 {
        return (0, 0);
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return (0, 0);
    };
    entries
        .flatten()
        .fold((0, 0), |(files, bytes), entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => {
                let (inner_files, inner_bytes) = tree_size(&entry.path(), depth + 1);
                (files + inner_files, bytes + inner_bytes)
            }
            Ok(meta) if meta.is_file() => (files + 1, bytes + meta.len()),
            _ => (files, bytes),
        })
}

/// How the replacement tree was built (LA-13). Counts and a closed mode only (R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Staging {
    /// `incremental` (seedless, from the live tree) or `full` (rebuilt from the seed).
    pub mode: &'static str,
    /// Patch ops left incomplete in the staged ledger.
    pub ops_skipped: u64,
    /// Files the stage shares with the live tree through a hard link.
    pub linked: u64,
    /// Files the stage copied from the live tree.
    pub copied: u64,
}

impl Staging {
    /// The seed-rebuild path: nothing is taken from the live tree.
    pub(super) const FULL: Self = Self {
        mode: "full",
        ops_skipped: 0,
        linked: 0,
        copied: 0,
    };
}

/// The replacement tree was staged and its digest was recorded. `files` and `bytes`
/// count the regular files of the staged tree; `elapsed` covers the staging.
pub(super) fn staged(files: u64, bytes: u64, elapsed: Duration, staging: Staging) {
    let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_staged",
        files,
        bytes,
        duration_ms,
        mode = staging.mode,
        ops_skipped = staging.ops_skipped,
        stage_linked = staging.linked,
        stage_copied = staging.copied,
        "game update: the replacement tree was staged; expected a digest-checked tree; commit may begin"
    );
}

/// The failure code of an intent error, when it maps to one (C-PD1).
fn intent_failure(error: &IntentError) -> Option<FailureCode> {
    match error {
        IntentError::Storage(StorageError::Io) => Some(FailureCode::LocalIo),
        IntentError::Storage(StorageError::InvalidDirectory) => {
            Some(FailureCode::GameFolderInvalid)
        }
        IntentError::Storage(StorageError::Corrupt) => Some(FailureCode::StateCorrupt),
        _ => None,
    }
}

/// Update preparation failed at `stage`, a closed name. `error` is the intent
/// error when the stage has one; `error` and `failure_code` are omitted otherwise.
pub(super) fn prepare_failed(id: Uuid, stage: &'static str, error: Option<&IntentError>) {
    let operation = operation_numbers::number(id);
    let failure_code = error.and_then(intent_failure).map(FailureCode::code);
    let error = error.map(intent_code);
    tracing::warn!(
        target: "launcher.game_update",
        event = "update_prepare_failed",
        operation,
        stage,
        error = error.as_deref(),
        failure_code,
        "game update: preparation failed; expected a staged replacement tree; the update stops and the installed game is unchanged"
    );
}

/// A replacement commit finished. `outcome` is `ok` or the failure's code; a
/// failed commit logs at WARN.
pub(super) fn committed(result: &Result<(), Failure>) {
    let outcome = outcome_of(result, |failure| failure_code(failure).to_owned());
    if result.is_ok() {
        tracing::info!(
            target: "launcher.game_update",
            event = "update_committed",
            outcome,
            "game update: the replacement commit finished; expected the install to name the target release"
        );
    } else {
        tracing::warn!(
            target: "launcher.game_update",
            event = "update_committed",
            outcome,
            "game update: the replacement commit failed; expected the install to name the target release; the update stops"
        );
    }
}

/// An interrupted checkpointed commit was reconciled. `outcome` is `ok` or the code.
pub(super) fn recovered(result: &Result<(), IntentError>) {
    let outcome = outcome_of(result, intent_code);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_recovered",
        outcome,
        "game update: the recovery finished; expected the install to match one recorded release"
    );
}

/// A terminal preparation was discarded. `outcome` is `ok` or the code.
pub(super) fn discarded(result: &Result<(), IntentError>) {
    let outcome = outcome_of(result, intent_code);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_discarded",
        outcome,
        "game update: the discard finished; expected only staged content to be removed"
    );
}

/// An interrupted preparation was abandoned. `reason` is `ok` or the code.
pub(super) fn abandoned(result: &Result<(), IntentError>) {
    let reason = outcome_of(result, intent_code);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_abandoned",
        reason,
        "game update: the preparation was abandoned; expected both trees to stay in place"
    );
}

/// An operation could not be marked uncertain, so a restart must reconcile it.
pub(super) fn mark_uncertain_failed(error: ContractError) {
    let error_kind = code(error);
    tracing::warn!(
        target: "launcher.game_update",
        event = "mark_uncertain_failed",
        error_kind,
        "game update: the operation could not be marked uncertain; expected a recovery record; the next start must reconcile it"
    );
}

/// A light-adopted install fell back to the seed rebuild. `reason` is closed:
/// `previous_release_unverified` or `ledger_mismatch`. No path, URL or host.
pub(super) fn seedless_declined(reason: &'static str) {
    tracing::info!(
        target: "launcher.game_update",
        event = "update_seedless_declined",
        reason,
        "game update: the seedless update was declined; expected a rebuild from the seed"
    );
}

/// The seedless Update failed. `reason` is closed: `fetch_failed`,
/// `hash_mismatch`, `write_set_unreadable`, `seed_stage_failed` or `rename_failed`.
pub(super) fn incremental_failed(reason: &'static str) {
    tracing::warn!(
        target: "launcher.game_update",
        event = "update_incremental_failed",
        reason,
        "game update: the seedless update failed; expected a staged tree built from the live one"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::{capture, Captured};
    use tracing::Level;

    /// The single captured row, asserted to be an update row at `level`.
    fn only_row(captured: &[Captured], level: Level) -> &Captured {
        assert_eq!(captured.len(), 1, "expected exactly one row: {captured:?}");
        assert_eq!(captured[0].target, "launcher.game_update");
        assert_eq!(captured[0].level, level);
        &captured[0]
    }

    #[test]
    fn staged_row_carries_counts_and_duration() {
        let ((), captured) = capture(|| staged(2, 8, Duration::from_millis(7), Staging::FULL));
        let row = only_row(&captured, Level::INFO);
        assert_eq!(row.field("event"), Some("update_staged"));
        assert_eq!(row.field("files"), Some("2"));
        assert_eq!(row.field("bytes"), Some("8"));
        assert_eq!(row.field("duration_ms"), Some("7"));
        assert_eq!(row.field("mode"), Some("full"));
        assert_eq!(row.field("ops_skipped"), Some("0"));
        assert_eq!(row.field("stage_linked"), Some("0"));
        assert_eq!(row.field("stage_copied"), Some("0"));
    }

    #[test]
    fn incremental_failed_row_carries_the_closed_reason() {
        let ((), captured) = capture(|| incremental_failed("hash_mismatch"));
        let row = only_row(&captured, Level::WARN);
        assert_eq!(row.field("event"), Some("update_incremental_failed"));
        assert_eq!(row.field("reason"), Some("hash_mismatch"));
    }
    #[test]
    fn staged_row_carries_the_incremental_mode_and_stage_counts() {
        let staging = Staging {
            mode: "incremental",
            ops_skipped: 3,
            linked: 40,
            copied: 5,
        };
        let ((), captured) = capture(|| staged(45, 9, Duration::from_millis(1), staging));
        let row = only_row(&captured, Level::INFO);
        assert_eq!(row.field("mode"), Some("incremental"));
        assert_eq!(row.field("ops_skipped"), Some("3"));
        assert_eq!(row.field("stage_linked"), Some("40"));
        assert_eq!(row.field("stage_copied"), Some("5"));
    }

    #[test]
    fn tree_size_counts_regular_files_under_the_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("nested")).unwrap();
        std::fs::write(dir.path().join("a.bin"), [0_u8; 3]).unwrap();
        std::fs::write(dir.path().join("nested").join("b.bin"), [0_u8; 5]).unwrap();
        assert_eq!(tree_size(dir.path(), 0), (2, 8));
    }

    #[test]
    fn committed_row_names_each_failure_code() {
        let cases = [
            (Ok(()), "ok"),
            (Err(Failure::Cancelled), "cancelled"),
            (Err(Failure::Failed), "failed"),
            (
                Err(Failure::ReconciliationRequired),
                "reconciliation_required",
            ),
        ];
        for (result, expected) in cases {
            let ((), captured) = capture(|| committed(&result));
            let level = if result.is_ok() {
                Level::INFO
            } else {
                Level::WARN
            };
            let row = only_row(&captured, level);
            assert_eq!(row.field("event"), Some("update_committed"));
            assert_eq!(row.field("outcome"), Some(expected));
        }
    }

    #[test]
    fn intent_rows_name_payload_codes_in_snake_case() {
        let cases: [(Result<(), IntentError>, &str); 4] = [
            (Ok(()), "ok"),
            (Err(IntentError::LauncherTooOld), "launcher_too_old"),
            (Err(IntentError::Storage(StorageError::Io)), "io"),
            (
                Err(IntentError::Operation(ContractError::InvalidTransition)),
                "invalid_transition",
            ),
        ];
        for (result, expected) in cases {
            let ((), captured) = capture(|| recovered(&result));
            let row = only_row(&captured, Level::INFO);
            assert_eq!(row.field("event"), Some("update_recovered"));
            assert_eq!(row.field("outcome"), Some(expected));

            let ((), captured) = capture(|| discarded(&result));
            assert_eq!(
                only_row(&captured, Level::INFO).field("event"),
                Some("update_discarded")
            );

            let ((), captured) = capture(|| abandoned(&result));
            let row = only_row(&captured, Level::INFO);
            assert_eq!(row.field("event"), Some("update_abandoned"));
            assert_eq!(row.field("reason"), Some(expected));
        }
    }

    #[test]
    fn prepare_failed_row_names_stage_error_and_failure_code() {
        let id = Uuid::new_v4();
        let cases: [(Option<IntentError>, Option<&str>, Option<&str>); 5] = [
            (Some(StorageError::Io.into()), Some("io"), Some("local_io")),
            (
                Some(StorageError::InvalidDirectory.into()),
                Some("invalid_directory"),
                Some("game_folder_invalid"),
            ),
            (
                Some(StorageError::Corrupt.into()),
                Some("corrupt"),
                Some("state_corrupt"),
            ),
            (Some(StorageError::Busy.into()), Some("busy"), None),
            (None, None, None),
        ];
        for (error, expected_error, expected_failure) in cases {
            let ((), captured) = capture(|| prepare_failed(id, "digest", error.as_ref()));
            let row = only_row(&captured, Level::WARN);
            assert_eq!(row.field("event"), Some("update_prepare_failed"));
            assert_eq!(row.field("stage"), Some("digest"));
            assert!(row.field("operation").is_some());
            assert_eq!(row.field("error"), expected_error);
            assert_eq!(row.field("failure_code"), expected_failure);
        }
    }

    #[test]
    fn mark_uncertain_row_is_a_warning_with_the_contract_code() {
        let ((), captured) = capture(|| mark_uncertain_failed(ContractError::InvalidTransition));
        let row = only_row(&captured, Level::WARN);
        assert_eq!(row.field("event"), Some("mark_uncertain_failed"));
        assert_eq!(row.field("error_kind"), Some("invalid_transition"));
    }
}
