//! Log upload events (launcher-telemetry LT-21a; C6, C9). Target
//! `launcher.telemetry`. No field holds a blob name (a store path), the store
//! URL, a local path or an error's `Display`: reasons are closed codes.
use cimmeria_launcher_core::logs;

/// The snake_case name of a [`logs::LogError`] variant. No wildcard arm.
pub(super) fn log_error_reason(error: &logs::LogError) -> &'static str {
    match error {
        logs::LogError::Io(_) => "io",
        logs::LogError::Zip(_) => "zip",
        logs::LogError::Http(_) => "http",
        logs::LogError::BadResponse { .. } => "bad_response",
        logs::LogError::InsecureUrl => "insecure_url",
        logs::LogError::Walk(_) => "walk",
    }
}

/// The ledger was not updated because an operation started during the PUT.
pub(super) fn ledger_skipped(reason: &'static str) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "log_upload_ledger_skipped",
        reason,
        "launcher log upload: ledger not saved; an operation started during the upload; the upload is reported as sent and the same logs may be sent again"
    );
}

/// Writing the ledger failed after a stored upload.
pub(super) fn ledger_save_failed() {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_ledger_save_failed",
        reason = "ledger_save_failed",
        "launcher log upload: ledger not saved; the upload is stored; the same logs may be sent again"
    );
}

/// The offsets file could not be written after a chunk was accepted: the next
/// press resends from the last saved point, a repeat of lines and never a loss.
pub(super) fn offsets_save_failed() {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_offsets_save_failed",
        reason = "offsets_save_failed",
        "launcher log upload: offsets not saved; the lines were accepted; the same lines may be sent again"
    );
}

/// Every log is over the cap, so nothing is sent.
pub(super) fn logs_too_large(left_out: usize, max_bytes: u64) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_too_large",
        count = left_out,
        bytes = max_bytes,
        "launcher log upload: refused; every log is over the size cap; expected one log to fit; nothing is sent"
    );
}

/// The oldest logs are left out to fit the cap.
pub(super) fn logs_trimmed(left_out: usize, max_bytes: u64) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "log_upload_trimmed",
        count = left_out,
        bytes = max_bytes,
        "launcher log upload: the oldest logs are left out to fit the size cap; the newest logs are sent"
    );
}

/// The logs match a past upload, so nothing is sent.
pub(super) fn nothing_new() {
    tracing::debug!(
        target: "launcher.telemetry",
        event = "log_upload_nothing_new",
        "launcher log upload: skipped; the same logs were sent before; nothing new to send"
    );
}

/// Reading or zipping the logs failed. `reason` comes from [`log_error_reason`].
pub(super) fn collect_failed(reason: &'static str) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_collect_failed",
        reason,
        "launcher log upload: could not read or zip the logs; expected the game logs; nothing is sent"
    );
}

/// A game log file was not read this press. `reason` is a closed word
/// (`read_failed` today); the path is never logged (R5).
pub(super) fn game_file_skipped(reason: &'static str) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_game_file_skipped",
        reason,
        "launcher log upload: a game log file was not read; its lines wait for the next press"
    );
}

/// The store refused the PUT with a non-success status.
pub(super) fn rejected(http_status: u16) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_rejected",
        http_status,
        "launcher log upload: the store refused the upload; expected a stored blob; nothing is stored"
    );
}

/// The PUT completed; `bytes` is the size of the zip.
pub(super) fn stored(bytes: usize) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "log_upload_stored",
        bytes = u64::try_from(bytes).unwrap_or(u64::MAX),
        "launcher log upload: stored; the logs are with the store"
    );
}

/// The PUT did not complete. `reason` is a closed transport code.
pub(super) fn failed(reason: &'static str) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_failed",
        reason,
        "launcher log upload: the upload did not reach the store; expected a stored blob; nothing is stored"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::{capture, Captured};
    use std::sync::{Mutex, MutexGuard};
    use tracing::Level;

    static SERIAL: Mutex<()> = Mutex::new(());
    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The one event a helper logged: `launcher.telemetry`, `level`, `event`.
    fn one(events: &[Captured], level: Level, event: &str) -> Captured {
        assert_eq!(events.len(), 1, "{events:?}");
        let found = events[0].clone();
        assert_eq!(
            (found.target.as_str(), found.level, found.field("event")),
            ("launcher.telemetry", level, Some(event))
        );
        found
    }

    #[test]
    fn ledger_skipped_logs_the_storage_code() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            ledger_skipped("busy");
        });
        let e = one(&events, Level::INFO, "log_upload_ledger_skipped");
        assert_eq!(e.field("reason"), Some("busy"));
    }

    #[test]
    fn ledger_save_failed_logs_the_reason() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            ledger_save_failed();
        });
        let e = one(&events, Level::WARN, "log_upload_ledger_save_failed");
        assert_eq!(e.field("reason"), Some("ledger_save_failed"));
    }

    #[test]
    fn offsets_save_failed_logs_the_reason() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            offsets_save_failed();
        });
        let e = one(&events, Level::WARN, "log_upload_offsets_save_failed");
        assert_eq!(e.field("reason"), Some("offsets_save_failed"));
    }

    #[test]
    fn logs_too_large_logs_the_count_and_the_cap() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            logs_too_large(3, 4);
        });
        let e = one(&events, Level::WARN, "log_upload_too_large");
        assert_eq!((e.field("count"), e.field("bytes")), (Some("3"), Some("4")));
    }

    #[test]
    fn logs_trimmed_logs_the_count_and_the_cap() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            logs_trimmed(1, 1000);
        });
        let e = one(&events, Level::INFO, "log_upload_trimmed");
        assert_eq!(
            (e.field("count"), e.field("bytes")),
            (Some("1"), Some("1000"))
        );
    }

    #[test]
    fn nothing_new_is_a_debug_event() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            nothing_new();
        });
        one(&events, Level::DEBUG, "log_upload_nothing_new");
    }

    #[test]
    fn collect_failed_logs_the_log_error_variant() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            collect_failed(log_error_reason(&logs::LogError::InsecureUrl));
        });
        let e = one(&events, Level::WARN, "log_upload_collect_failed");
        assert_eq!(e.field("reason"), Some("insecure_url"));
    }

    #[test]
    fn rejected_logs_the_http_status() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            rejected(403);
        });
        let e = one(&events, Level::WARN, "log_upload_rejected");
        assert_eq!(e.field("http_status"), Some("403"));
    }

    #[test]
    fn stored_logs_the_size_and_no_blob_name() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            stored(42);
        });
        let e = one(&events, Level::INFO, "log_upload_stored");
        assert_eq!((e.field("bytes"), e.field("blob")), (Some("42"), None));
    }

    #[test]
    fn failed_logs_the_transport_reason() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            failed("timeout");
        });
        let e = one(&events, Level::WARN, "log_upload_failed");
        assert_eq!(e.field("reason"), Some("timeout"));
    }
}
