//! Launcher journey summaries: one terminal row per install, runtime-setup,
//! repair, uninstall or launch attempt, kept in a small local queue until the
//! exporter delivers it. Independent of game and DLL telemetry.
//!
//! Nothing is tracked, recorded or sent unless an endpoint is configured, the
//! user opted in, no opt-out is in doubt and the state needs no reopen. An
//! attempt is reported only when that held at its admission; consent given
//! later never reaches back.
//!
//! Coverage. Admitted attempts are observed at the operation journal, so every
//! admission and terminal is seen without a hook in each workflow. `phases`
//! holds only what this process timed: `starting` and `running` for every kind,
//! plus `download` and `extraction` for installs. An install moves between
//! those two with every blob, and each entry is the total over all of them.
//! Whatever follows the last unpack (content verification, promotion) has no
//! progress of its own and is counted in `extraction`.
//!
//! What is left out on purpose (`attempt.rs`):
//!
//! - A launch row carries `starting` only: no `running` entry and no total
//!   duration. Both would end when the game process exits, so they would be the
//!   length of the play session, which is game activity and outside this
//!   consent. Its `phase` still says where the attempt ended.
//! - A row whose end was not observed is `unknown`, never a success. It has no
//!   total duration and no entry for the phase that was open.
//! - After a restart a row carries no timing and no phase at all.
//!
//! Failures before admission arrive through `summary_pre_admission_failure`.
//!
//! No entry point here can fail or change the result of the launcher's own work:
//! each returns `()`, contains its own panics and counts what it swallowed.
mod attempt;
mod batch;
mod consent;
mod endpoint;
mod exchange;
mod export;
mod queue;
mod schema;
mod tracker;

#[cfg(test)]
pub(in crate::storage) mod tests;

pub use endpoint::SummaryEndpoint;
pub use export::start;
pub use schema::{
    DroppedCounts, LauncherVersion, Millis, MintRequest, PhaseDuration, RetryCount, Summary,
    SummaryArch, SummaryErrorCode, SummaryOperation, SummaryOs, SummaryOutcome, SummaryPhase,
    SummaryRequest, SummaryResponse, SummaryResult, TimedPhase, MAX_BATCH, MAX_BODY_BYTES,
    MAX_PHASES, SCHEMA_VERSION, SESSION_KIND,
};
pub(super) use tracker::Shared;

use super::{install_worker, launch, DesktopState};
use crate::{OperationState, Snapshot};
use attempt::Pending;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};
use tracker::{lock, Gate, Tracker};
use uuid::Uuid;

/// Native composition input. `endpoint: None` keeps the whole component inert.
pub struct SummaryConfig {
    /// The shell's product version.
    pub launcher_version: (u16, u16, u16),
    pub endpoint: Option<SummaryEndpoint>,
}

/// What the component swallowed instead of failing the launcher. Local only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SummaryFaults {
    pub queue_writes: u32,
    pub panics: u32,
}

pub(super) fn shared(root: &Path, consent: bool) -> Shared {
    Tracker::new(root, consent)
}

/// The single hook: called by `FileJournal::commit` after a confirmed write.
pub(super) fn observe_commit(shared: &Shared, snapshot: &Snapshot) {
    let mut tracker = lock(shared);
    if catch_unwind(AssertUnwindSafe(|| tracker.observe(snapshot))).is_err() {
        tracker.faults.panics = tracker.faults.panics.saturating_add(1);
    }
}

impl DesktopState {
    /// Without an endpoint this removes any queue file and leaves the component
    /// inert. With one it loads the queue and settles what a previous process
    /// left tracked: a terminal attempt becomes its row, a lost one `unknown`.
    pub fn configure_summaries(&mut self, config: SummaryConfig) {
        self.summary_guarded(|state| {
            state.summary_sync();
            let current = state.operations.snapshot().operation.clone();
            let pending = lock(&state.summaries).configure(config, current.as_ref());
            if let Some(pending) = pending {
                state.summary_finalize(pending);
            }
        });
    }

    /// Enters a timed phase of the tracked attempt. Anything else is a no-op.
    pub fn summary_phase(&mut self, operation_id: Uuid, phase: TimedPhase) {
        self.summary_guarded(|state| {
            state.summary_sync();
            lock(&state.summaries).enter_phase(operation_id, phase);
        });
    }

    /// A command that failed before the journal admitted it. Every argument is a
    /// closed value. Identical failures share one row and raise its retry count.
    pub fn summary_pre_admission_failure(
        &mut self,
        request_operation_id: Option<Uuid>,
        operation: SummaryOperation,
        phase: SummaryPhase,
        code: SummaryErrorCode,
    ) {
        self.summary_guarded(|state| {
            state.summary_sync();
            let current = state
                .operations
                .snapshot()
                .operation
                .as_ref()
                .map(|operation| operation.id);
            lock(&state.summaries).pre_admission_failure(
                request_operation_id,
                current,
                operation,
                phase,
                code,
            );
        });
    }

    pub fn summary_faults(&self) -> SummaryFaults {
        lock(&self.summaries).faults
    }

    /// Lazy and idempotent: turns a committed end into its queue entry. Runs at
    /// the start of `operations_mut`, a preferences save and every entry point,
    /// and right after the install worker's terminal commit, while the install
    /// result it reads is still this attempt's.
    pub(super) fn finalize_summaries(&mut self) {
        self.summary_guarded(|state| {
            state.summary_sync();
        });
    }

    // Taken while the caller's state guard is held: a panic stops here, so it
    // can neither poison that mutex nor unwind into launcher work.
    fn summary_guarded<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> Option<R> {
        match catch_unwind(AssertUnwindSafe(|| body(self))) {
            Ok(value) => Some(value),
            Err(_) => {
                let mut tracker = lock(&self.summaries);
                tracker.faults.panics = tracker.faults.panics.saturating_add(1);
                None
            }
        }
    }

    // Refreshes the gate inputs the journal observer cannot see, then finalizes.
    fn summary_sync(&mut self) -> Gate {
        let consent = self.preferences.launcher_summary_consent;
        let reopen = self.requires_reopen();
        let pending = {
            let mut tracker = lock(&self.summaries);
            tracker.test_panic();
            tracker.consent = consent;
            tracker.reopen = reopen;
            tracker.retry_store();
            tracker.take_pending()
        };
        if let Some(pending) = pending {
            self.summary_finalize(pending);
        }
        lock(&self.summaries).gate()
    }

    fn summary_finalize(&mut self, pending: Pending) {
        let code =
            (pending.outcome == SummaryOutcome::Failed).then(|| self.summary_error_code(&pending));
        lock(&self.summaries).enqueue_terminal(pending, code);
    }

    // Closed codes from the launcher's own result records, read through the
    // existing accessors. Any doubt is `unspecified`.
    fn summary_error_code(&self, pending: &Pending) -> SummaryErrorCode {
        use install_worker::Outcome;
        use launch::Observation;
        // Detail belongs to the journal's current operation; once another attempt
        // replaced this one, the record on disk is no longer its own.
        let current = self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|operation| {
                operation.id == pending.local_id && operation.state == OperationState::Failed
            });
        match pending.operation {
            SummaryOperation::PrepareRuntime => SummaryErrorCode::PrerequisiteFailed,
            SummaryOperation::Install if current => match self.install_outcome() {
                Ok(Some(Outcome::DestinationUnavailable)) => {
                    SummaryErrorCode::DestinationUnavailable
                }
                Ok(Some(Outcome::InstallFailed)) => SummaryErrorCode::InstallFailed,
                Ok(Some(Outcome::ContentInvalid)) => SummaryErrorCode::ContentInvalid,
                Ok(Some(Outcome::RosettaRequired)) => SummaryErrorCode::RosettaRequired,
                Ok(Some(Outcome::RuntimeUnavailable)) => SummaryErrorCode::RuntimeUnavailable,
                _ => SummaryErrorCode::Unspecified,
            },
            SummaryOperation::Launch if current => match self.launch_observation() {
                Ok(Some(Observation::NotStarted)) => SummaryErrorCode::LaunchNotStarted,
                Ok(Some(Observation::ProcessExited { code, early, .. })) if code != 0 => {
                    if early {
                        SummaryErrorCode::LaunchEarlyExit
                    } else {
                        SummaryErrorCode::LaunchExitNonzero
                    }
                }
                _ => SummaryErrorCode::Unspecified,
            },
            _ => SummaryErrorCode::Unspecified,
        }
    }
}
