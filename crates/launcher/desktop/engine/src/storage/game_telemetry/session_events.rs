//! Launcher events for the game-session mint (launcher-telemetry C8 and C10).
//! Each helper is one `launcher.telemetry` tracing call, forwarded as C1 allows.

/// The mint failed: the Play session goes on without diagnostics. `reason` is a
/// C8 code and `http_status` is set only when the login server replied.
pub(super) fn mint_failed(reason: &'static str, http_status: Option<u16>, duration_ms: u64) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "game_session_mint_failed",
        reason,
        http_status,
        duration_ms,
        "game telemetry: expected a dev-session token from the login server, got {reason}; this Play session sends no diagnostics"
    );
}

/// The login server handed out a loopback upload address and the session uploads
/// to the login server's own telemetry route instead (C10).
pub(super) fn endpoint_overridden() {
    tracing::info!(
        target: "launcher.telemetry",
        event = "upload_endpoint_overridden",
        kind = "game",
        "game telemetry: the login server gave a loopback upload address; uploads go to the login server instead"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;

    #[test]
    fn a_mint_failure_is_one_warning_with_its_reason_and_status() {
        let ((), events) = capture(|| mint_failed("http_status", Some(503), 12));
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.target, "launcher.telemetry");
        assert_eq!(event.level, tracing::Level::WARN);
        assert_eq!(event.field("event"), Some("game_session_mint_failed"));
        assert_eq!(event.field("reason"), Some("http_status"));
        assert_eq!(event.field("http_status"), Some("503"));
        assert_eq!(event.field("duration_ms"), Some("12"));
    }

    #[test]
    fn a_mint_failure_without_a_reply_has_no_status_field() {
        let ((), events) = capture(|| mint_failed("timeout", None, 8_000));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].field("reason"), Some("timeout"));
        assert_eq!(events[0].field("http_status"), None);
    }

    #[test]
    fn an_overridden_upload_is_one_info_for_the_game_kind() {
        let ((), events) = capture(endpoint_overridden);
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.target, "launcher.telemetry");
        assert_eq!(event.level, tracing::Level::INFO);
        assert_eq!(event.field("event"), Some("upload_endpoint_overridden"));
        assert_eq!(event.field("kind"), Some("game"));
    }
}
