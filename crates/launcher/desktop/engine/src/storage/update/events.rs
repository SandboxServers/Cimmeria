//! Engine update events (launcher-telemetry LT-13; C5, C6). One info row per staged,
//! committed, recovered, discarded or abandoned update, and a warn row when an
//! operation cannot be marked uncertain. Only closed codes and counts are written (R5).
use std::fmt::Debug;
use std::path::Path;
use std::time::Duration;

/// `ok`, or the Debug name of the error. Engine errors are unit or tuple variants
/// with no free text, so the name is a closed code.
fn outcome_code<E: Debug>(result: &Result<(), E>) -> String {
    match result {
        Ok(()) => "ok".to_owned(),
        Err(error) => format!("{error:?}"),
    }
}

/// Regular files and their total length under `root`. Symlinks are not followed,
/// and the depth bound keeps a malformed tree from recursing without end.
fn tree_size(root: &Path, depth: usize) -> (u64, u64) {
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

/// The replacement tree was staged and its digest was recorded. `files` and `bytes`
/// count the regular files under `stage`.
pub(super) fn staged(stage: &Path, elapsed: Duration) {
    let (files, bytes) = tree_size(stage, 0);
    let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_staged",
        files,
        bytes,
        duration_ms,
        "game update: the replacement tree was staged; expected a digest-checked tree; commit may begin"
    );
}

/// A replacement commit finished. `outcome` is `ok` or the error's code.
pub(super) fn committed<E: Debug>(result: &Result<(), E>) {
    let outcome = outcome_code(result);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_committed",
        outcome,
        "game update: the replacement commit finished; expected the install to name the target release"
    );
}

/// An interrupted checkpointed commit was reconciled. `outcome` is `ok` or the code.
pub(super) fn recovered<E: Debug>(result: &Result<(), E>) {
    let outcome = outcome_code(result);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_recovered",
        outcome,
        "game update: the recovery finished; expected the install to match one recorded release"
    );
}

/// A terminal preparation was discarded. `outcome` is `ok` or the code.
pub(super) fn discarded<E: Debug>(result: &Result<(), E>) {
    let outcome = outcome_code(result);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_discarded",
        outcome,
        "game update: the discard finished; expected only staged content to be removed"
    );
}

/// An interrupted preparation was abandoned. `reason` is `ok` or the code.
pub(super) fn abandoned<E: Debug>(result: &Result<(), E>) {
    let reason = outcome_code(result);
    tracing::info!(
        target: "launcher.game_update",
        event = "update_abandoned",
        reason,
        "game update: the preparation was abandoned; expected both trees to stay in place"
    );
}

/// An operation could not be marked uncertain, so a restart must reconcile it.
pub(super) fn mark_uncertain_failed<E: Debug>(error: E) {
    let error_kind = format!("{error:?}");
    tracing::warn!(
        target: "launcher.game_update",
        event = "mark_uncertain_failed",
        error_kind,
        "game update: the operation could not be marked uncertain; expected a recovery record; the next start must reconcile it"
    );
}
