//! Install lifecycle events (launcher-telemetry LT-14b; C5, C6). The install
//! worker logs through these, one target and one shape each. Logging never
//! changes the outcome: every helper returns nothing.
use super::Outcome;
use crate::install::InstallError;
use crate::launcher_summary::TimedPhase;
use crate::operation_numbers;
use crate::ContractError;
use uuid::Uuid;

/// The install began; `resumed` when it continues an earlier attempt.
pub(crate) fn install_started(id: Uuid, resumed: bool) {
    let operation = operation_numbers::number(id);
    let action = if resumed { "resume" } else { "install" };
    tracing::info!(
        target: "launcher.install",
        event = "install_started",
        operation,
        action,
        "install: started"
    );
}

/// The install ended. Success and cancel are infos; every other outcome is a
/// warn whose `reason` is its closed code.
pub(crate) fn install_finished(id: Uuid, outcome: Outcome) {
    let operation = operation_numbers::number(id);
    let duration_ms = operation_numbers::elapsed_ms(id);
    match outcome {
        Outcome::ContentPrepared => tracing::info!(
            target: "launcher.install",
            event = "install_finished",
            operation,
            duration_ms,
            outcome = "ok",
            "install: finished"
        ),
        Outcome::Cancelled => tracing::info!(
            target: "launcher.install",
            event = "install_finished",
            operation,
            duration_ms,
            outcome = "cancelled",
            "install: finished"
        ),
        failed => tracing::warn!(
            target: "launcher.install",
            event = "install_failed",
            operation,
            duration_ms,
            reason = outcome_code(failed),
            "install: failed"
        ),
    }
}

/// The closed code of a failed outcome. Success and cancel never reach it.
fn outcome_code(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::DestinationUnavailable => "destination_unavailable",
        Outcome::InstallFailed => "install_failed",
        Outcome::ContentInvalid => "content_invalid",
        Outcome::RosettaRequired => "rosetta_required",
        Outcome::RuntimeUnavailable => "runtime_unavailable",
        Outcome::ReconciliationRequired => "reconciliation_required",
        Outcome::ContentPrepared => "ok",
        Outcome::Cancelled => "cancelled",
    }
}

/// Logs each timed phase of one install once. A patch reopens `download`, and
/// that second entry is not logged again.
pub(crate) struct PhaseLog {
    id: Uuid,
    seen: [bool; 2],
}

impl PhaseLog {
    pub(crate) fn new(id: Uuid) -> Self {
        Self {
            id,
            seen: [false; 2],
        }
    }

    /// Logs `install_phase` the first time `phase` is entered; other phases are ignored.
    pub(crate) fn enter(&mut self, phase: TimedPhase) {
        let (slot, name) = match phase {
            TimedPhase::Download => (0, "download"),
            TimedPhase::Extraction => (1, "extraction"),
            _ => return,
        };
        if self.seen[slot] {
            return;
        }
        self.seen[slot] = true;
        let operation = operation_numbers::number(self.id);
        tracing::info!(
            target: "launcher.install",
            event = "install_phase",
            operation,
            phase = name,
            "install: phase"
        );
    }
}

/// The worker task panicked, so the install's outcome is unknown.
pub(crate) fn install_reconcile_unknown(id: Uuid) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.install",
        event = "install_reconcile_unknown",
        operation,
        "install: the worker panicked; the outcome is unknown"
    );
}

/// The unusable seed archive could not be forgotten; the next attempt may fail on it.
pub(crate) fn install_seed_forget_failed(id: Uuid) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.install",
        event = "install_seed_forget_failed",
        operation,
        "install: forgetting the unusable seed archive failed"
    );
}

/// A stage failed. Carries the closed code, the HTTP status and the unpack kind,
/// never the URL. The error text goes to the local log only, under its own target.
pub(crate) fn install_stage_failed(id: Uuid, error: &InstallError) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.install",
        event = "install_stage_failed",
        operation,
        reason = error.code(),
        http_status = error.http_status(),
        unpack_kind = error.unpack_code(),
        "install: a stage failed"
    );
    tracing::warn!(target: "launcher.install.detail", event = "install_stage_detail", operation,
        detail = %error, "install: stage failure detail (local log only)");
}

/// The destination could not be used. `stage` is `check` (the ownership claim),
/// `create` (the staging folder) or `promote` (moving the staged game into
/// place); `cause` is a closed code from `causes`.
pub(crate) fn destination_unavailable(id: Uuid, stage: &'static str, cause: &'static str) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.install",
        event = "destination_unavailable",
        operation,
        stage,
        cause,
        failure_code = "game_folder_not_writable",
        "install: the game folder could not be used; expected a writable folder the launcher owns; the install stops"
    );
}

/// The install could not record its uncertain state; the shell calls this too.
pub fn mark_uncertain_failed(error: ContractError) {
    tracing::warn!(
        target: "launcher.install",
        event = "mark_uncertain_failed",
        reason = error.code(),
        "install: recording the uncertain state failed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::{capture, Captured};
    use tracing::Level;

    fn event_named<'a>(events: &'a [Captured], name: &str) -> Vec<&'a Captured> {
        events
            .iter()
            .filter(|e| e.target == "launcher.install" && e.field("event") == Some(name))
            .collect()
    }

    #[test]
    fn started_and_finished_share_one_operation() {
        let id = Uuid::new_v4();
        let ((), events) = capture(|| {
            install_started(id, false);
            install_finished(id, Outcome::ContentPrepared);
        });
        let started = event_named(&events, "install_started");
        let finished = event_named(&events, "install_finished");
        assert_eq!(started.len(), 1);
        assert_eq!(finished.len(), 1);
        assert_eq!(started[0].level, Level::INFO);
        assert_eq!(started[0].field("action"), Some("install"));
        assert_eq!(finished[0].level, Level::INFO);
        assert_eq!(finished[0].field("outcome"), Some("ok"));
        assert!(finished[0].field("duration_ms").is_some());
        assert!(started[0].field("operation").is_some());
        assert_eq!(
            started[0].field("operation"),
            finished[0].field("operation")
        );
    }

    #[test]
    fn a_failed_outcome_is_a_warn_with_its_code() {
        let ((), events) = capture(|| install_finished(Uuid::new_v4(), Outcome::ContentInvalid));
        let failed = event_named(&events, "install_failed");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].level, Level::WARN);
        assert_eq!(failed[0].field("reason"), Some("content_invalid"));
    }

    #[test]
    fn a_cancel_is_an_info_finish() {
        let ((), events) = capture(|| install_finished(Uuid::new_v4(), Outcome::Cancelled));
        let finished = event_named(&events, "install_finished");
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].level, Level::INFO);
        assert_eq!(finished[0].field("outcome"), Some("cancelled"));
    }

    #[test]
    fn a_resume_sets_the_action() {
        let ((), events) = capture(|| install_started(Uuid::new_v4(), true));
        let started = event_named(&events, "install_started");
        assert_eq!(started[0].field("action"), Some("resume"));
    }

    #[test]
    fn each_phase_is_logged_once() {
        let id = Uuid::new_v4();
        let ((), events) = capture(|| {
            let mut phases = PhaseLog::new(id);
            phases.enter(TimedPhase::Download);
            phases.enter(TimedPhase::Download);
            phases.enter(TimedPhase::Extraction);
            phases.enter(TimedPhase::Download);
        });
        let logged = event_named(&events, "install_phase");
        assert_eq!(logged.len(), 2, "{events:?}");
        assert_eq!(logged[0].field("phase"), Some("download"));
        assert_eq!(logged[1].field("phase"), Some("extraction"));
    }

    #[test]
    fn a_stage_failure_carries_its_code_and_status() {
        let id = Uuid::new_v4();
        let error = InstallError::UnexpectedStatus {
            status: 503,
            url: "https://example.invalid/blob".into(),
        };
        let ((), events) = capture(|| install_stage_failed(id, &error));
        let failed = event_named(&events, "install_stage_failed");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].level, Level::WARN);
        assert_eq!(failed[0].field("reason"), Some("unexpected_status"));
        assert_eq!(failed[0].field("http_status"), Some("503"));
        assert!(failed[0]
            .fields
            .values()
            .all(|v| !v.contains("example.invalid")));
    }

    #[test]
    fn mark_uncertain_failed_carries_the_contract_code() {
        let ((), events) = capture(|| mark_uncertain_failed(ContractError::PersistenceFailed));
        let failed = event_named(&events, "mark_uncertain_failed");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].level, Level::WARN);
        assert_eq!(failed[0].field("reason"), Some("persistence_failed"));
        assert_eq!(failed[0].field("operation"), None);
    }

    fn unpack_failure() -> InstallError {
        InstallError::Unpack(crate::unpack::UnpackError::Cab(
            "expanding the cabinet set in /home/u/x/Data needs Windows".into(),
        ))
    }

    #[test]
    fn an_unpack_failure_logs_its_detail_locally() {
        let ((), events) = capture(|| install_stage_failed(Uuid::new_v4(), &unpack_failure()));
        let detail: Vec<_> = events
            .iter()
            .filter(|e| e.target == "launcher.install.detail")
            .collect();
        assert_eq!(detail.len(), 1);
        assert_eq!(detail[0].field("event"), Some("install_stage_detail"));
        assert!(detail[0].field("detail").unwrap().contains("needs Windows"));
        assert_eq!(
            crate::storage::game_telemetry::launcher_event::wire_target("launcher.install.detail"),
            None
        );
    }

    #[test]
    fn the_forwarded_failure_carries_a_closed_kind_and_no_text() {
        let ((), events) = capture(|| install_stage_failed(Uuid::new_v4(), &unpack_failure()));
        let failed = event_named(&events, "install_stage_failed");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].field("reason"), Some("unpack"));
        assert_eq!(failed[0].field("unpack_kind"), Some("cab"));
        assert!(failed[0]
            .fields
            .values()
            .all(|v| !v.contains("/home/") && !v.contains("needs Windows")));
    }

    #[test]
    fn a_non_unpack_failure_has_no_unpack_kind() {
        let ((), events) =
            capture(|| install_stage_failed(Uuid::new_v4(), &InstallError::Cancelled));
        let failed = event_named(&events, "install_stage_failed");
        assert_eq!(failed[0].field("unpack_kind"), None);
    }
}
