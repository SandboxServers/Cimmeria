//! Repair and uninstall events (launcher-telemetry LT-16a; C5, C6, C9). Target
//! `launcher.repair`. Each helper takes the operation id and logs its number.
use super::preparation::Failure;
use crate::install::InstallError;
use crate::{ContractError, StorageError};
use uuid::Uuid;

/// A repair run started on its worker.
pub(crate) fn repair_started(id: Uuid) {
    tracing::info!(
        target: "launcher.repair",
        event = "repair_started",
        operation = crate::operation_numbers::number(id),
        "launcher repair: run started; expected a verified install to rebuild; the rebuild stages next"
    );
}

/// The journal claim for the repair failed; the old tree is untouched.
pub(crate) fn repair_claim_failed(id: Uuid, error: StorageError) {
    tracing::warn!(
        target: "launcher.repair",
        event = "repair_claim_failed",
        operation = crate::operation_numbers::number(id),
        reason = error.code(),
        "launcher repair: claim refused; expected a free install folder; the run stops and the install is unchanged"
    );
}

/// Staging the rebuilt content failed; `reason` is the closed install code.
pub(crate) fn repair_stage_failed(id: Uuid, error: &InstallError) {
    tracing::warn!(
        target: "launcher.repair",
        event = "repair_stage_failed",
        operation = crate::operation_numbers::number(id),
        reason = error.code(),
        http_status = error.http_status(),
        "launcher repair: staging failed; expected the release content over https; the run stops and the install is unchanged"
    );
}

/// The staged content did not match the release layout.
pub(crate) fn repair_content_invalid(id: Uuid) {
    tracing::warn!(
        target: "launcher.repair",
        event = "repair_content_invalid",
        operation = crate::operation_numbers::number(id),
        "launcher repair: staged content refused; expected every release file in place; the run stops and the install is unchanged"
    );
}

/// The repair committed its staged content.
pub(crate) fn repair_finished(id: Uuid) {
    tracing::info!(
        target: "launcher.repair",
        event = "repair_finished",
        operation = crate::operation_numbers::number(id),
        outcome = "ok",
        duration_ms = crate::operation_numbers::elapsed_ms(id),
        "launcher repair: run finished; expected the staged content to replace the install; the install is rebuilt"
    );
}

/// A run ended without commit. A cancel is an info finish; anything else is a
/// warn `repair_failed` with a closed reason.
pub(crate) fn repair_ended(id: Uuid, failure: &Failure) {
    let operation = crate::operation_numbers::number(id);
    let duration_ms = crate::operation_numbers::elapsed_ms(id);
    match failure {
        Failure::Cancelled => tracing::info!(
            target: "launcher.repair",
            event = "repair_finished",
            operation,
            outcome = "cancelled",
            duration_ms,
            "launcher repair: run cancelled; expected the owner to stop it; the install is unchanged"
        ),
        Failure::Failed => tracing::warn!(
            target: "launcher.repair",
            event = "repair_failed",
            operation,
            reason = "failed",
            duration_ms,
            "launcher repair: run failed; expected a rebuild from the release; the install is unchanged"
        ),
        Failure::ReconciliationRequired => tracing::warn!(
            target: "launcher.repair",
            event = "repair_failed",
            operation,
            reason = "reconciliation_required",
            duration_ms,
            "launcher repair: run needs reconciliation; expected an explicit recovery; the outcome is kept for it"
        ),
    }
}

/// A repair dispatch failed while it was still `Starting`; it ends failed.
/// `reason` is the shell's closed `JobError` code.
pub(crate) fn repair_not_started(id: Uuid, reason: &'static str) {
    tracing::warn!(
        target: "launcher.repair",
        event = "repair_not_started",
        operation = crate::operation_numbers::number(id),
        reason,
        failure_code = "repair_not_started",
        "launcher repair: run stopped before it started; expected the installed copy and plan to verify; nothing on disk changed and the operation is failed"
    );
}

/// Start-up found an interrupted repair that had not started and marked it
/// failed: no repair files were on disk (D-PD3).
pub(crate) fn repair_reconciled_unstarted(id: Uuid) {
    tracing::info!(
        target: "launcher.repair",
        event = "repair_reconciled_unstarted",
        operation = crate::operation_numbers::number(id),
        outcome = "failed",
        "launcher repair: an interrupted repair had not started; expected no repair files on disk and found none; marked failed"
    );
}

/// The journal could not record an uncertain outcome. `pub` for the shell.
pub fn mark_uncertain_failed(error: ContractError) {
    tracing::warn!(
        target: "launcher.repair",
        event = "mark_uncertain_failed",
        reason = error.code(),
        "launcher operation: uncertain outcome not recorded; expected the journal to accept the mark; the run is still reported as uncertain"
    );
}

/// An uninstall began removing files.
pub(crate) fn uninstall_started(id: Uuid) {
    tracing::info!(
        target: "launcher.repair",
        event = "uninstall_started",
        operation = crate::operation_numbers::number(id),
        "launcher uninstall: removal started; expected a confirmed intent; files the release lists are removed"
    );
}

/// An uninstall finished. `recovering` is true when it resumed a run.
pub(crate) fn uninstall_finished(id: Uuid, recovering: bool) {
    tracing::info!(
        target: "launcher.repair",
        event = "uninstall_finished",
        operation = crate::operation_numbers::number(id),
        outcome = "ok",
        recovering,
        duration_ms = crate::operation_numbers::elapsed_ms(id),
        "launcher uninstall: removal finished; expected the release files gone; unrelated files are kept"
    );
}

/// An uninstall failed; `reason` is a closed code.
pub(crate) fn uninstall_failed(id: Uuid, reason: &'static str) {
    tracing::warn!(
        target: "launcher.repair",
        event = "uninstall_failed",
        operation = crate::operation_numbers::number(id),
        reason,
        "launcher uninstall: removal failed; expected the install folder to be removable; the outcome is kept for reconciliation"
    );
}

/// The adopted folder was kept because it holds files the release does not list.
pub(crate) fn uninstall_kept_adopted_folder() {
    tracing::info!(
        target: "launcher.repair",
        event = "uninstall_kept_adopted_folder",
        "launcher uninstall: adopted folder kept; expected it to hold only release files; the unlisted files stay"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;

    fn id() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn repair_start_and_finish_share_one_operation() {
        let id = id();
        let ((), events) = capture(|| {
            repair_started(id);
            repair_finished(id);
        });
        let repair: Vec<_> = events
            .iter()
            .filter(|e| e.target == "launcher.repair")
            .collect();
        assert_eq!(repair.len(), 2);
        assert_eq!(repair[0].level, tracing::Level::INFO);
        assert_eq!(repair[0].field("event"), Some("repair_started"));
        assert_eq!(repair[1].field("event"), Some("repair_finished"));
        assert_eq!(repair[1].field("outcome"), Some("ok"));
        assert_eq!(repair[0].field("operation"), repair[1].field("operation"));
        assert!(repair[1].field("duration_ms").is_some());
    }

    #[test]
    fn a_cancelled_repair_is_an_info_finish() {
        let ((), events) = capture(|| repair_ended(id(), &Failure::Cancelled));
        assert_eq!(events[0].level, tracing::Level::INFO);
        assert_eq!(events[0].field("event"), Some("repair_finished"));
        assert_eq!(events[0].field("outcome"), Some("cancelled"));
        assert_eq!(events[0].field("reason"), None);
    }

    #[test]
    fn a_failed_repair_is_a_warn_with_its_reason() {
        let ((), events) = capture(|| repair_ended(id(), &Failure::Failed));
        assert_eq!(events[0].level, tracing::Level::WARN);
        assert_eq!(events[0].field("event"), Some("repair_failed"));
        assert_eq!(events[0].field("reason"), Some("failed"));
        let ((), events) = capture(|| repair_ended(id(), &Failure::ReconciliationRequired));
        assert_eq!(events[0].field("reason"), Some("reconciliation_required"));
    }

    #[test]
    fn a_stage_failure_carries_code_and_status() {
        let error = InstallError::UnexpectedStatus {
            status: 503,
            url: "https://example.invalid/x".into(),
        };
        let ((), events) = capture(|| repair_stage_failed(id(), &error));
        assert_eq!(events[0].field("event"), Some("repair_stage_failed"));
        assert_eq!(events[0].field("reason"), Some("unexpected_status"));
        assert_eq!(events[0].field("http_status"), Some("503"));
        assert!(events[0]
            .fields
            .values()
            .all(|value| !value.contains("example.invalid")));
    }

    #[test]
    fn uninstall_failed_carries_its_reason() {
        let ((), events) = capture(|| uninstall_failed(id(), "storage"));
        assert_eq!(events[0].level, tracing::Level::WARN);
        assert_eq!(events[0].target, "launcher.repair");
        assert_eq!(events[0].field("event"), Some("uninstall_failed"));
        assert_eq!(events[0].field("reason"), Some("storage"));
    }
}
