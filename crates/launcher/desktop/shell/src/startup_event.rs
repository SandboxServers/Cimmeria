//! Launcher startup and update-reconcile events (LT-06, C5 and C6). Every field
//! here is a closed code, a count or a boolean: no path, host, account or error
//! text reaches a forwarded target (R5).
use cimmeria_launcher_engine::updater::Error as UpdateError;

/// What a start knows about the build and its settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupFacts {
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub release_build: bool,
    pub dll_bundled: bool,
    pub game_diagnostics: bool,
    pub setup_diagnostics: bool,
    pub login_servers: u64,
    pub log_file_ok: bool,
    pub previous_run_panicked: bool,
}

/// One info per start on `launcher.startup`: `event=launcher_started` with the facts.
pub(crate) fn emit_startup(facts: &StartupFacts) {
    tracing::info!(
        target: "launcher.startup",
        event = "launcher_started",
        version = facts.version,
        os = facts.os,
        arch = facts.arch,
        release_build = facts.release_build,
        dll_bundled = facts.dll_bundled,
        game_diagnostics = facts.game_diagnostics,
        setup_diagnostics = facts.setup_diagnostics,
        login_servers = facts.login_servers,
        log_file_ok = facts.log_file_ok,
        previous_run_panicked = facts.previous_run_panicked,
        "launcher startup: state loaded; telemetry follows the recorded consents"
    );
}

/// The previous launcher was still running at the restart deadline. This start
/// continues; the state lock keeps the two from sharing launcher state.
pub(crate) fn old_launcher_still_running() {
    tracing::warn!(
        target: "launcher.startup",
        event = "old_launcher_still_running",
        reason = "restart_wait_expired",
        "launcher startup: the previous launcher had not exited; expected it to exit before this start, this start continues under the state lock"
    );
}

/// The update handoff was not acknowledged at start. The update owner stays
/// visible and game work is held until a later start reconciles it.
pub(crate) fn update_reconcile_failed(reason: &'static str) {
    tracing::warn!(
        target: "launcher.update",
        event = "update_reconcile_failed",
        reason = reason,
        "launcher update: the handoff was not acknowledged; expected the update to reconcile at start, the update owner stays visible and game work is held"
    );
}

/// The telemetry queue check could not read the recorded choice. Nothing was
/// touched; the queue waits for the next start.
pub(crate) fn update_recovery_failed(reason: &'static str) {
    tracing::warn!(
        target: "launcher.update",
        event = "update_recovery_failed",
        reason = reason,
        "launcher update: the recorded telemetry choice could not be read; expected a readable choice, nothing was touched and the queue waits for the next start"
    );
}

/// Maps every updater error to a closed snake_case code (`storage` for a
/// storage failure). The error's own text is never used.
pub(crate) fn reconcile_reason(error: UpdateError) -> &'static str {
    match error {
        UpdateError::Disabled => "disabled",
        UpdateError::Busy => "busy",
        UpdateError::StaleRevision => "stale_revision",
        UpdateError::StaleOffer => "stale_offer",
        UpdateError::Policy => "policy",
        UpdateError::Feed => "feed",
        UpdateError::Platform => "platform",
        UpdateError::NotNewer => "not_newer",
        UpdateError::Signature => "signature",
        UpdateError::SignedVersion => "signed_version",
        UpdateError::Size => "size",
        UpdateError::Transport => "transport",
        UpdateError::Timeout => "timeout",
        UpdateError::Interrupted => "interrupted",
        UpdateError::Package => "package",
        UpdateError::Target => "target",
        UpdateError::Replace => "replace",
        UpdateError::Spawn => "spawn",
        UpdateError::Reconciliation => "reconciliation",
        UpdateError::Storage(_) => "storage",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
    use cimmeria_launcher_engine::StorageError;
    use tracing::Level;

    fn facts() -> StartupFacts {
        StartupFacts {
            version: "0.1.0",
            os: "windows",
            arch: "x86_64",
            release_build: true,
            dll_bundled: false,
            game_diagnostics: false,
            setup_diagnostics: true,
            login_servers: 1,
            log_file_ok: true,
            previous_run_panicked: true,
        }
    }

    #[test]
    fn emit_startup_pins_the_event_fields_and_leaks_no_path() {
        let ((), events) = capture(|| emit_startup(&facts()));
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.target, "launcher.startup");
        assert_eq!(event.level, Level::INFO);
        assert_eq!(event.field("event"), Some("launcher_started"));
        assert_eq!(event.field("version"), Some("0.1.0"));
        assert_eq!(event.field("release_build"), Some("true"));
        assert_eq!(event.field("dll_bundled"), Some("false"));
        assert_eq!(event.field("setup_diagnostics"), Some("true"));
        assert_eq!(event.field("login_servers"), Some("1"));
        assert_eq!(event.field("previous_run_panicked"), Some("true"));
        for value in event.fields.values() {
            assert!(!value.contains('\\') && !value.contains('/'), "{value}");
        }
    }

    #[test]
    fn old_launcher_still_running_warns_with_a_reason() {
        let ((), events) = capture(old_launcher_still_running);
        assert_eq!(events[0].target, "launcher.startup");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].field("event"), Some("old_launcher_still_running"));
        assert_eq!(events[0].field("reason"), Some("restart_wait_expired"));
    }

    #[test]
    fn update_reconcile_failure_warns_on_the_update_target() {
        let ((), events) = capture(|| {
            update_reconcile_failed(reconcile_reason(UpdateError::Feed));
        });
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, "launcher.update");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].field("event"), Some("update_reconcile_failed"));
        assert_eq!(events[0].field("reason"), Some("feed"));
    }

    #[test]
    fn update_recovery_failure_warns_on_the_update_target() {
        let ((), events) = capture(|| update_recovery_failed("choice_unreadable"));
        assert_eq!(events[0].target, "launcher.update");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].field("event"), Some("update_recovery_failed"));
        assert_eq!(events[0].field("reason"), Some("choice_unreadable"));
    }

    #[test]
    fn reconcile_reason_keeps_storage_failures_closed() {
        assert_eq!(reconcile_reason(UpdateError::Timeout), "timeout");
        assert_eq!(
            reconcile_reason(UpdateError::Storage(StorageError::Io)),
            "storage"
        );
    }
}
