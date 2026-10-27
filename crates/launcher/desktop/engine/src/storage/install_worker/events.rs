//! Install lifecycle events (launcher-telemetry LT-14b; C5, C6). The install
//! worker logs through these, one target and one shape each. Logging never
//! changes the outcome: every helper returns nothing.
use super::Outcome;
use crate::install::{InstallError, Progress};
use crate::launcher_summary::TimedPhase;
use crate::operation_numbers;
use crate::ContractError;
use std::time::Instant;
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

/// Tracks the C-7 phase (`phase` on the wire) of one install. Each phase that
/// ends is logged once with its last counters, when the next phase begins and
/// when the job ends.
pub(crate) struct PhaseTally {
    id: Uuid,
    open: Option<OpenPhase>,
}

/// The phase in progress, with the last counters it reported.
struct OpenPhase {
    phase: &'static str,
    bytes: u64,
    files: u64,
    started: Instant,
}

impl PhaseTally {
    pub(crate) fn new(id: Uuid) -> Self {
        Self { id, open: None }
    }

    /// Records one progress event. A different phase closes the open one first.
    pub(crate) fn observe(&mut self, progress: &Progress) {
        let (phase, bytes, files) = wire_counts(progress);
        match &mut self.open {
            Some(open) if open.phase == phase => {
                // The RAR index pass shares the `check` phase with the archive
                // hash; it reports no bytes, so it keeps the hashed count.
                if !matches!(progress, Progress::Checking { total: 0, .. }) {
                    open.bytes = bytes;
                }
                open.files = files;
            }
            _ => {
                self.finish();
                self.open = Some(OpenPhase {
                    phase,
                    bytes,
                    files,
                    started: Instant::now(),
                });
            }
        }
    }

    /// Logs the open phase, if any, and leaves no phase open.
    pub(crate) fn finish(&mut self) {
        let Some(open) = self.open.take() else {
            return;
        };
        let elapsed_ms = u64::try_from(open.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        install_phase_finished(self.id, open.phase, open.bytes, open.files, elapsed_ms);
    }
}

/// The C-7 wire name of a progress event, with its byte count (`download`,
/// `copy`, and `check` with a known total) and its file count (`extraction`,
/// `expansion`). The RAR index pass (`check` with no total) is not a byte phase.
fn wire_counts(progress: &Progress) -> (&'static str, u64, u64) {
    match *progress {
        Progress::Downloading { downloaded, .. } => ("download", downloaded, 0),
        Progress::Copying { copied, .. } => ("copy", copied, 0),
        Progress::Checking { read, total } => ("check", if total > 0 { read } else { 0 }, 0),
        Progress::Extracting { current, .. } => ("extraction", 0, current as u64),
        Progress::Expanding { current, .. } => ("expansion", 0, current as u64),
    }
}

/// A C-7 install phase ended. `bytes` and `files` are its last counters, and
/// `elapsed_ms` is how long it lasted.
pub(crate) fn install_phase_finished(
    id: Uuid,
    phase: &str,
    bytes: u64,
    files: u64,
    elapsed_ms: u64,
) {
    let operation = operation_numbers::number(id);
    tracing::info!(
        target: "launcher.install",
        event = "install_phase_finished",
        operation,
        phase,
        bytes,
        files,
        elapsed_ms,
        "install: phase finished"
    );
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

    fn phase_finished(events: &[Captured]) -> Vec<&Captured> {
        event_named(events, "install_phase_finished")
    }

    #[test]
    fn a_copy_then_check_closes_the_copy_with_its_bytes() {
        let ((), events) = capture(|| {
            let mut tally = PhaseTally::new(Uuid::new_v4());
            tally.observe(&Progress::Copying {
                copied: 10,
                total: 100,
            });
            tally.observe(&Progress::Copying {
                copied: 100,
                total: 100,
            });
            tally.observe(&Progress::Checking {
                read: 0,
                total: 100,
            });
            tally.observe(&Progress::Checking {
                read: 40,
                total: 100,
            });
            tally.finish();
        });
        let finished = phase_finished(&events);
        assert_eq!(finished.len(), 2, "{events:?}");
        assert_eq!(finished[0].field("phase"), Some("copy"));
        assert_eq!(finished[0].field("bytes"), Some("100"));
        assert_eq!(finished[1].field("phase"), Some("check"));
        assert_eq!(finished[1].field("bytes"), Some("40"));
        assert!(finished[1].field("elapsed_ms").is_some());
    }

    #[test]
    fn the_rar_index_pass_is_a_check_with_no_bytes() {
        let ((), events) = capture(|| {
            let mut tally = PhaseTally::new(Uuid::new_v4());
            tally.observe(&Progress::Checking { read: 7, total: 0 });
            tally.observe(&Progress::Extracting {
                label: "RAR".into(),
                current: 3,
                total: 9,
                filename: "a.cab".into(),
            });
            tally.finish();
        });
        let finished = phase_finished(&events);
        assert_eq!(finished.len(), 2, "{events:?}");
        assert_eq!(finished[0].field("phase"), Some("check"));
        assert_eq!(finished[0].field("bytes"), Some("0"));
        assert_eq!(finished[1].field("phase"), Some("extraction"));
        assert_eq!(finished[1].field("files"), Some("3"));
        assert_eq!(finished[1].field("bytes"), Some("0"));
    }

    #[test]
    fn the_index_pass_keeps_the_hashed_bytes_of_its_check() {
        let ((), events) = capture(|| {
            let mut tally = PhaseTally::new(Uuid::new_v4());
            tally.observe(&Progress::Downloading {
                label: "archive".into(),
                downloaded: 50,
                total: 100,
            });
            tally.observe(&Progress::Checking {
                read: 100,
                total: 100,
            });
            tally.observe(&Progress::Checking { read: 7, total: 0 });
            tally.observe(&Progress::Extracting {
                label: "RAR".into(),
                current: 1,
                total: 2,
                filename: "a.cab".into(),
            });
            tally.finish();
        });
        let finished = phase_finished(&events);
        assert_eq!(finished.len(), 3, "{events:?}");
        assert_eq!(finished[0].field("phase"), Some("download"));
        assert_eq!(finished[1].field("phase"), Some("check"));
        assert_eq!(finished[1].field("bytes"), Some("100"));
        assert_eq!(finished[2].field("phase"), Some("extraction"));
    }

    #[test]
    fn expansion_counts_its_files_and_ends_with_the_job() {
        let ((), events) = capture(|| {
            let mut tally = PhaseTally::new(Uuid::new_v4());
            tally.observe(&Progress::Expanding {
                label: "cabinets".into(),
                current: 2,
                total: 5,
                filename: "b.cab".into(),
            });
            tally.observe(&Progress::Expanding {
                label: "cabinets".into(),
                current: 5,
                total: 5,
                filename: "e.cab".into(),
            });
            tally.finish();
            // Nothing is open after the job ends, so a second finish logs nothing.
            tally.finish();
        });
        let finished = phase_finished(&events);
        assert_eq!(finished.len(), 1, "{events:?}");
        assert_eq!(finished[0].field("phase"), Some("expansion"));
        assert_eq!(finished[0].field("files"), Some("5"));
    }
}
