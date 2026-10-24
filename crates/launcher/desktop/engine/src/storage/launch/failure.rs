//! Why a launch did not start (C-PD3). The launch result record carries an
//! optional failure code next to its observation, so a `NotStarted` launch
//! says why: a spawn error, a failed verify or setup, a refused login server
//! list, or a dispatch that failed before anything was recorded.
use super::*;
use crate::failure::FailureCode;
use std::sync::Mutex;

/// The failure code for a game process that could not be created. EBADARCH
/// (86, "Bad CPU type in executable") on macOS means Rosetta 2 is missing.
pub(crate) fn classify_spawn(
    kind: std::io::ErrorKind,
    raw: Option<i32>,
    macos: bool,
) -> FailureCode {
    if macos && raw == Some(86) {
        return FailureCode::RosettaMissing;
    }
    match kind {
        std::io::ErrorKind::NotFound => FailureCode::SpawnNotFound,
        std::io::ErrorKind::PermissionDenied => FailureCode::SpawnPermissionDenied,
        _ => FailureCode::SpawnFailed,
    }
}

/// Notes `code` on the current launch record under the state lock. A lock or
/// write error leaves the record as it was; the launch still ends `NotStarted`.
pub(super) fn note(state: &Mutex<DesktopState>, plan: &Plan, code: FailureCode) {
    if let Ok(mut owner) = state.lock() {
        let _ = owner.note_launch_failure(plan, code);
    }
}

/// The game did not start for a known reason before the helper ran: logs
/// `launch_not_started` with `reason` and `code`, and notes `code` so the
/// worker's terminal `NotStarted` record carries it.
pub(super) fn not_started(
    state: &Mutex<DesktopState>,
    plan: &Plan,
    reason: &'static str,
    code: FailureCode,
) {
    events::launch_not_started(plan.id, reason, code);
    note(state, plan, code);
}

impl DesktopState {
    /// Sets `failure` on the current launch record without changing its
    /// observation. The record must exist and belong to `plan`.
    pub(crate) fn note_launch_failure(
        &mut self,
        plan: &Plan,
        code: FailureCode,
    ) -> Result<(), IntentError> {
        if self.launch_plan()?.as_ref() != Some(plan) {
            return Err(ContractError::IdentityConflict.into());
        }
        let mut record = self.launch_record(plan)?.ok_or(StorageError::Corrupt)?;
        record.failure = Some(code);
        self.write_launch(&format!("launch-result-{}.json", plan.id), &record)
    }

    /// The failure code of the current launch, when its record has one. A
    /// launch whose observation is unknown has no known failure.
    pub fn launch_failure(&self) -> Result<Option<FailureCode>, IntentError> {
        match (self.launch_observation()?, self.launch_plan()?) {
            (Some(observation), Some(plan)) if observation != Observation::Unknown => {
                Ok(self.launch_record(&plan)?.and_then(|record| record.failure))
            }
            _ => Ok(None),
        }
    }

    /// Writes one record for a launch that never started: `NotStarted` with
    /// `failure`, in a single write, after the same plan check `record_launch`
    /// makes.
    pub(crate) fn record_not_started(
        &mut self,
        plan: &Plan,
        code: FailureCode,
    ) -> Result<(), IntentError> {
        if self.launch_plan()?.as_ref() != Some(plan) {
            return Err(ContractError::IdentityConflict.into());
        }
        self.write_launch(
            &format!("launch-result-{}.json", plan.id),
            &Record {
                id: plan.id,
                digest: plan.digest()?,
                observation: Observation::NotStarted,
                failure: Some(code),
            },
        )
    }

    /// The launch result record of `plan`, checked against its id and digest.
    pub(super) fn launch_record(&self, plan: &Plan) -> Result<Option<Record>, IntentError> {
        let record: Option<Record> = read(
            &self
                .directory
                .root
                .join(format!("launch-result-{}.json", plan.id)),
        )?;
        match record {
            Some(record) if record.id != plan.id || record.digest != plan.digest()? => {
                Err(StorageError::Corrupt.into())
            }
            record => Ok(record),
        }
    }
}

#[cfg(test)]
#[path = "failure_tests.rs"]
mod tests;
