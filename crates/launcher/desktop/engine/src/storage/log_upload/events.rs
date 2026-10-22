//! Log upload events (launcher-telemetry LT-21a; C6, C9). Target
//! `launcher.telemetry`. No field holds a local path, a server URL or an
//! error's `Display`: reasons are closed codes.

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

/// Reading the logs failed. `reason` is a closed word, never the error text.
pub(super) fn collect_failed(reason: &'static str) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "log_upload_collect_failed",
        reason,
        "launcher log upload: could not read the logs; expected the game logs; nothing is sent"
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

/// The telemetry request did not complete. `reason` is a closed transport code.
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
    fn collect_failed_logs_the_reason() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            collect_failed("lines_read");
        });
        let e = one(&events, Level::WARN, "log_upload_collect_failed");
        assert_eq!(e.field("reason"), Some("lines_read"));
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
