//! Setup-area events (LT-15) for the seed archive, PhysX, migration and
//! prerequisite commands, on the `launcher.install` target. Each command logs one
//! `<area>_started`, then one `<area>_finished` or `<area>_failed`. Every field
//! is a closed code or a number: no path, URL, host, IP or error text (R5).

use serde::Serialize;

const TARGET: &str = "launcher.install";

// The closed code of an I/O error kind (C6), or `"other"`: the engine's one
// vocabulary, so a field means the same in both halves.
pub(crate) use cimmeria_launcher_engine::game_telemetry::launcher_event::kind as io_kind;

/// The serde code of `value` (snake_case), or `"unknown"` when it has none.
pub(crate) fn code<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "unknown".to_owned(),
    }
}

/// A seed archive command started.
pub(crate) fn seed_archive_started(action: &str) {
    tracing::info!(target: TARGET, event = "seed_archive_started", action, "seed archive command started");
}

/// A seed archive command finished with `outcome` (`ok` or a closed code).
pub(crate) fn seed_archive_finished(action: &str, outcome: &str, duration_ms: u64) {
    tracing::info!(target: TARGET, event = "seed_archive_finished", action, outcome, duration_ms, "seed archive command finished");
}

/// A seed archive command failed; `reason` is its closed error code.
pub(crate) fn seed_archive_failed(action: &str, reason: &str, error_kind: &str) {
    tracing::warn!(target: TARGET, event = "seed_archive_failed", action, reason, error_kind, "seed archive command failed");
}

/// A signed seed release fetch failed, so the next one waits before retrying.
pub(crate) fn seed_archive_release_failed(reason: &str, http_status: Option<u16>) {
    tracing::warn!(target: TARGET, event = "seed_archive_release_failed", reason, http_status, "seed release fetch failed");
}

/// An archive was picked and its check starts; `bytes` is the archive's size.
pub(crate) fn seed_archive_picked(bytes: u64) {
    tracing::info!(target: TARGET, event = "seed_archive_picked", bytes, "seed archive picked");
}

/// A pick was refused; `reason` is a closed code (`signing_key_unavailable`,
/// `release_unavailable`, `verify_failed` or `busy`).
pub(crate) fn seed_archive_pick_refused(reason: &str) {
    tracing::warn!(target: TARGET, event = "seed_archive_pick_refused", reason, "seed archive pick refused");
}

// PhysX install start and outcome are logged by the engine's `install()`; the shell
// logs only an Inspect failure and the install refusal that never reaches it.
/// A PhysX command failed; `reason` is its closed error code.
pub(crate) fn physx_failed(action: &str, reason: &str, error_kind: &str) {
    tracing::warn!(target: TARGET, event = "physx_failed", action, reason, error_kind, "PhysX command failed");
}

/// A migration command started.
pub(crate) fn migration_started(action: &str) {
    tracing::info!(target: TARGET, event = "migration_started", action, "migration command started");
}

/// A migration command finished with `outcome` (`ok` or a closed code).
pub(crate) fn migration_finished(action: &str, outcome: &str, duration_ms: u64) {
    tracing::info!(target: TARGET, event = "migration_finished", action, outcome, duration_ms, "migration command finished");
}

/// A migration command failed; `reason` is its closed error code.
pub(crate) fn migration_failed(action: &str, reason: &str, error_kind: &str) {
    tracing::warn!(target: TARGET, event = "migration_failed", action, reason, error_kind, "migration command failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
    use std::io;

    /// Each helper logs one record on `launcher.install` with its `event` field.
    #[test]
    fn every_helper_logs_on_the_install_target() {
        let ((), records) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            seed_archive_started("inspect");
            seed_archive_finished("inspect", "ok", 3);
            seed_archive_failed("inspect", "io", "not_found");
            seed_archive_release_failed("fetch_failed", Some(503));
            physx_failed("install", "package_mismatch", "other");
            migration_started("confirm");
            migration_finished("confirm", "ok", 5);
            migration_failed("confirm", "source_changed", "other");
        });
        let events: Vec<(&str, tracing::Level, &str)> = records
            .iter()
            .map(|r| (r.target.as_str(), r.level, r.field("event").unwrap_or("")))
            .collect();
        assert_eq!(events.len(), 8);
        assert!(events.iter().all(|(target, _, _)| *target == TARGET));
        assert_eq!(
            events[2],
            (TARGET, tracing::Level::WARN, "seed_archive_failed")
        );
        assert_eq!(
            events[3],
            (TARGET, tracing::Level::WARN, "seed_archive_release_failed")
        );
        assert_eq!(events[0].1, tracing::Level::INFO);
        assert_eq!(records[2].field("reason"), Some("io"));
        assert_eq!(records[3].field("http_status"), Some("503"));
        assert_eq!(records[1].field("duration_ms"), Some("3"));
    }

    #[test]
    fn io_kind_and_code_are_closed_snake_case() {
        let missing = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(io_kind(&missing), "not_found");
        assert_eq!(io_kind(&io::Error::other("x")), "other");
        assert_eq!(code(&"busy"), "busy");
    }
}
