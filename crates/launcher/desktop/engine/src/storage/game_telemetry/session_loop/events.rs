//! Game-telemetry session-loop and queue-write events (launcher-telemetry
//! LT-09b; C5, C6). The session loop, the install result and the
//! client-patches events log through these, one target and one shape each.
use super::super::transport::{events::reason, TransportError};
use super::SessionEnd;
use std::time::Duration;

/// The previous session was still sending past its bound; it was stopped.
pub(crate) fn previous_session_stopped(bound: Duration) {
    let duration_ms = u64::try_from(bound.as_millis()).unwrap_or(u64::MAX);
    tracing::warn!(
        target: "launcher.telemetry",
        event = "previous_session_stopped",
        duration_ms,
        "game telemetry: the previous session was still sending past its bound; stopped it; what it had queued goes with this session"
    );
}

/// Not sent; local only (LT-09a forwards `bundle_upload_failed`).
pub(crate) fn bundle_not_sent(code: TransportError) {
    tracing::debug!(
        target: "launcher.telemetry",
        event = "bundle_not_sent",
        reason = reason(code),
        "game telemetry: session log bundle not sent"
    );
}

/// The last flush at game exit did not finish; `why` is its closed code.
pub(crate) fn final_flush_failed(why: &'static str) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "final_flush_failed",
        reason = why,
        "game telemetry: the final flush at game exit did not finish; expected the whole queue sent; the rest goes with the next session"
    );
}

/// Events the queue could not take this session.
pub(crate) fn session_events_lost(count: u64) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "session_events_lost",
        count,
        "game telemetry: events could not be queued this session; expected every event on disk; those events were lost"
    );
}

/// The player opted out while the session ran.
pub(crate) fn game_session_withdrawn(count: u64) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "game_session_withdrawn",
        count,
        "game telemetry: turned off during the session; stopped sending and left no queue"
    );
}

/// The session's end: the events it queued, the bundle, and the exit when observed.
pub(crate) fn game_session_finished(end: &SessionEnd, duration_ms: u64) {
    let exit_code = end.exit.map(|exit| i64::from(exit.code));
    tracing::info!(
        target: "launcher.telemetry",
        event = "game_session_finished",
        count = end.events,
        bundle_sent = end.bundle_sent,
        exit_observed = end.exit.is_some(),
        exit_code,
        duration_ms,
        "game telemetry: session finished"
    );
}

/// The client-patches log could not be read; its events are not sent.
pub(crate) fn client_patches_read_failed() {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "client_patches_read_failed",
        reason = "task_failed",
        "game telemetry: client-patches log read failed; expected its events; they are not sent"
    );
}

/// One event could not be queued: warn for the first of a session, debug after.
pub(crate) fn event_queue_failed(code: TransportError, n: u64) {
    let why = reason(code);
    if n == 1 {
        tracing::warn!(
            target: "launcher.telemetry",
            event = "event_queue_failed",
            reason = why,
            count = n,
            "game telemetry: event not queued; expected it written to the disk queue; it is lost"
        );
    } else {
        tracing::debug!(
            target: "launcher.telemetry",
            event = "event_queue_failed",
            reason = why,
            count = n,
            "game telemetry: event not queued; expected it written to the disk queue; it is lost"
        );
    }
}

/// An install, repair or update result went to the disk queue.
pub(crate) fn install_result_queued(action: &'static str, outcome: &'static str, count: u64) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "install_result_queued",
        action,
        outcome,
        count,
        "game telemetry: install result queued for the next telemetry session"
    );
}

/// An install result could not be queued; `why` is its closed code.
pub(crate) fn install_result_not_queued(action: &'static str, why: &'static str) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "install_result_not_queued",
        action,
        reason = why,
        "game telemetry: install result not queued; expected it on disk for the next session; it is not sent"
    );
}

/// Client-patches events the queue refused at game exit.
pub(crate) fn client_patch_events_not_queued(count: u64) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "client_patch_events_not_queued",
        count,
        "game telemetry: client-patches events not queued; expected them on disk for the next session; they are not sent"
    );
}

#[cfg(test)]
mod tests {
    use super::super::{previous_sessions, settle_previous, GameExit};
    use super::*;
    use crate::test_capture::{capture, Captured};

    /// Every row the session events forward: the `launcher.telemetry` target.
    fn forwarded(captured: &[Captured]) -> Vec<&Captured> {
        captured
            .iter()
            .filter(|c| c.target == "launcher.telemetry")
            .collect()
    }

    /// Runs `future` on a fresh current-thread runtime, so the thread-local capture sees it.
    fn block_on<F: std::future::Future<Output = ()>>(future: F) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future);
    }

    #[test]
    fn each_helper_logs_its_level_event_and_codes() {
        // Callsite interest: see `transport/events_tests.rs`.
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), captured) = capture(|| {
            previous_session_stopped(Duration::from_secs(35));
            bundle_not_sent(TransportError::Upload);
            final_flush_failed("timeout");
            session_events_lost(3);
            game_session_withdrawn(4);
            client_patches_read_failed();
            install_result_queued("install", "ok", 2);
            install_result_not_queued("repair", "queue_write_failed");
            client_patch_events_not_queued(5);
        });
        let rows = forwarded(&captured);
        let got: Vec<_> = rows
            .iter()
            .map(|r| (r.level, r.field("event").unwrap()))
            .collect();
        #[rustfmt::skip]
        assert_eq!(got, [
            (tracing::Level::WARN, "previous_session_stopped"), (tracing::Level::DEBUG, "bundle_not_sent"), (tracing::Level::WARN, "final_flush_failed"),
            (tracing::Level::WARN, "session_events_lost"), (tracing::Level::INFO, "game_session_withdrawn"), (tracing::Level::WARN, "client_patches_read_failed"),
            (tracing::Level::INFO, "install_result_queued"), (tracing::Level::WARN, "install_result_not_queued"), (tracing::Level::WARN, "client_patch_events_not_queued"),
        ]);
        assert_eq!(rows[0].field("duration_ms"), Some("35000"));
        assert_eq!(rows[1].field("reason"), Some("upload"));
        assert_eq!(rows[2].field("reason"), Some("timeout"));
        assert_eq!(rows[3].field("count"), Some("3"));
        assert_eq!(rows[4].field("count"), Some("4"));
        assert_eq!(rows[5].field("reason"), Some("task_failed"));
        assert_eq!(rows[6].field("action"), Some("install"));
        assert_eq!(rows[6].field("outcome"), Some("ok"));
        assert_eq!(rows[6].field("count"), Some("2"));
        assert_eq!(rows[7].field("action"), Some("repair"));
        assert_eq!(rows[7].field("reason"), Some("queue_write_failed"));
        assert_eq!(rows[8].field("count"), Some("5"));
    }

    #[test]
    fn game_session_finished_says_whether_the_game_exited() {
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        #[rustfmt::skip]
        let exited = SessionEnd { events: 7, exit: Some(GameExit { host_pid: 1, guest_pid: 2, code: 7, early: false }), bundle_sent: true, ..Default::default() };
        let no_exit = SessionEnd::default();
        let ((), captured) = capture(|| {
            game_session_finished(&exited, 1500);
            game_session_finished(&no_exit, 10);
        });
        let rows = forwarded(&captured);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].level, tracing::Level::INFO);
        assert_eq!(rows[0].field("count"), Some("7"));
        assert_eq!(rows[0].field("bundle_sent"), Some("true"));
        assert_eq!(rows[0].field("exit_observed"), Some("true"));
        assert_eq!(rows[0].field("exit_code"), Some("7"));
        assert_eq!(rows[0].field("duration_ms"), Some("1500"));
        assert_eq!(rows[1].field("exit_observed"), Some("false"));
        assert_eq!(rows[1].field("exit_code"), None);
    }

    #[test]
    fn a_queue_failure_warns_once_then_debugs() {
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), captured) = capture(|| {
            event_queue_failed(TransportError::Queue, 1);
            event_queue_failed(TransportError::Queue, 2);
        });
        let rows = forwarded(&captured);
        assert_eq!(rows[0].level, tracing::Level::WARN);
        assert_eq!(rows[0].field("reason"), Some("queue"));
        assert_eq!(rows[0].field("count"), Some("1"));
        assert_eq!(rows[1].level, tracing::Level::DEBUG);
        assert_eq!(rows[1].field("count"), Some("2"));
    }

    #[test]
    fn a_stuck_previous_session_is_stopped_on_the_real_bound() {
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let dir = tempfile::tempdir().unwrap();
        let ((), captured) = capture(|| {
            block_on(async {
                let stuck = tokio::spawn(std::future::pending::<()>());
                previous_sessions()
                    .lock()
                    .unwrap()
                    .insert(dir.path().to_path_buf(), stuck.abort_handle());
                settle_previous(dir.path(), Duration::from_millis(20)).await;
            });
        });
        let rows = forwarded(&captured);
        assert_eq!(rows.len(), 1, "{captured:?}");
        assert_eq!(rows[0].level, tracing::Level::WARN);
        assert_eq!(rows[0].field("event"), Some("previous_session_stopped"));
        assert_eq!(rows[0].field("duration_ms"), Some("20"));
        assert!(captured.iter().all(|c| c.field("bound_s").is_none()));
    }
}
