//! One `client.launcher.install_result` event per Install, Repair or game
//! Update run, built by core's `install_result_event` so the fields match
//! what the Windows launcher sends. A run has no game session, so the event
//! goes to the disk queue and ships with the next session's first flush.
//! Nothing is queued unless the player has opted in to game telemetry.
//!
//! The per-patch detail comes from core's `install_all` report, stashed by
//! the run when it has one ([`DesktopState::stash_install_report`]); the
//! desktop's own terminal outcome decides `result` where the report cannot
//! (a failure after every patch applied, a lost commit).
//!
//! Known gap: only the terminal commit queues the event. A run finished by
//! reconciliation after a restart, a crash between that commit and the
//! queue append, and a preparation whose owner went away before the commit
//! handoff send nothing. Nothing records which runs were reported, so a
//! later report could not be exactly-once (see `docs/launch-telemetry.md`).
use super::session_loop::events;
use super::transport::{queue_now, Consent};
use super::DesktopState;
use crate::install_report::{InstallReport, RunResult};
use cimmeria_launcher_core::telemetry::install_result::install_result_event;
use uuid::Uuid;

/// Which operation ran. Only for the launcher's log: the event's fields are
/// the Windows launcher's, which has no such field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Run {
    Install,
    Repair,
    Update,
}

impl Run {
    /// The `action` the launcher's log gives the run.
    fn as_str(self) -> &'static str {
        match self {
            Run::Install => "install",
            Run::Repair => "repair",
            Run::Update => "update",
        }
    }
}

/// How the run ended, as the journal recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ending {
    Succeeded,
    Cancelled,
    /// The code becomes the event's `error` when the report has none.
    Failed(&'static str),
    /// Completion was lost: the journal waits for reconciliation.
    Uncertain,
}

/// `install_all`'s report for the run in progress, until the run ends.
#[derive(Debug, Default)]
pub(crate) struct Pending(Option<(Uuid, InstallReport)>);

impl DesktopState {
    /// Keep `install_all`'s report until operation `id` ends.
    pub(crate) fn stash_install_report(&mut self, id: Uuid, report: InstallReport) {
        self.install_telemetry.0 = Some((id, report));
    }

    /// Queue the result of operation `id`. Call it once per run, from the
    /// journal commit that ended it. Returns whether an event was queued.
    pub(crate) fn queue_install_result(&mut self, id: Uuid, run: Run, ending: Ending) -> bool {
        if ending == Ending::Succeeded {
            self.log_applied_content("applied");
        }
        let report = match self.install_telemetry.0.take() {
            Some((owner, report)) if owner == id => Some(report),
            _ => None,
        };
        let consent = match Consent::read(self) {
            Ok(Some(consent)) => consent,
            Ok(None) => return false,
            Err(_) => {
                events::install_result_not_queued(run.as_str(), "choice_unreadable");
                return false;
            }
        };
        let report = report_for(report, ending);
        let event = install_result_event(&report, chrono::Utc::now().timestamp_millis());
        match queue_now(&consent, &event) {
            Ok(()) => {
                events::install_result_queued(
                    run.as_str(),
                    report.result.as_str(),
                    report.patches.len() as u64,
                );
                true
            }
            Err(_) => {
                events::install_result_not_queued(run.as_str(), "queue_write_failed");
                false
            }
        }
    }
}

/// The report the event is built from. `install_all` saw only its own part
/// of the run: a stage that failed validation or promotion after every patch
/// applied is still an error, with the desktop's code as its text.
pub(crate) fn report_for(report: Option<InstallReport>, ending: Ending) -> InstallReport {
    let mut report = report.unwrap_or_default();
    let code = match ending {
        Ending::Succeeded => return report,
        Ending::Cancelled => {
            report.result = RunResult::Cancelled;
            return report;
        }
        Ending::Failed(code) => code,
        Ending::Uncertain => "reconciliation_required",
    };
    if matches!(report.result, RunResult::Ok | RunResult::Cancelled) {
        report.result = RunResult::Error;
        report.error = Some(code.into());
    }
    report
}
