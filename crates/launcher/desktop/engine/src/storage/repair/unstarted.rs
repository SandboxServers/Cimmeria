//! A repair left in `ReconciliationRequired` that never started its work
//! (D-PD3). `claim()` creates the work directory before any process or
//! download runs, so a repair with no work directory, no prepared record, no
//! commit record and no backup touched nothing: start-up ends it `Failed`
//! without asking the player to Recover or Abandon.
use super::*;
use crate::game_telemetry::install_event::{Ending, Run};
use crate::OperationState;

impl DesktopState {
    /// Marks the current repair `Failed` when it waits for reconciliation and
    /// left nothing on disk. Follows `abandon()`'s precondition order, without
    /// its confirmation and without stopping a Wine prefix. Returns the id it
    /// reconciled; `Ok(None)` when any precondition fails, with nothing changed.
    pub fn reconcile_unstarted_repair(&mut self) -> Result<Option<Uuid>, IntentError> {
        if self.requires_reopen() {
            return Ok(None);
        }
        let waiting = self
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .and_then(|op| {
                (op.kind == OperationKind::Repair
                    && op.state == OperationState::ReconciliationRequired)
                    .then_some(op.id)
            });
        let Some(id) = waiting else {
            return Ok(None);
        };
        let Some(plan) = self.repair_plan()? else {
            return Ok(None);
        };
        #[cfg(not(target_os = "macos"))]
        if !plan.installation.backend.is_native() {
            return Ok(None);
        }
        let root = &plan.installation.destination;
        let _root_owner = match lock_owner(&plan.installation) {
            Err(StorageError::InUse) => return Ok(None),
            result => result?,
        };
        let state_root = &self.directory.root;
        if !missing(&state_root.join(format!("repair-commit-{id}.json")))?
            || !missing(&plan.backup())?
            || root.canonicalize().map_err(|_| StorageError::Io)? != *root
            || directory_or_absent(&plan.installation.content_root())? != plan.original_present
            || !missing(&state_root.join(format!("repair-prepared-{id}.json")))?
            || !missing(&plan.work_directory())?
        {
            return Ok(None);
        }
        self.operations_mut()?
            .reconcile(id, OperationState::Failed)?;
        events::repair_reconciled_unstarted(id);
        self.queue_install_result(id, Run::Repair, Ending::Failed("repair_not_started"));
        Ok(Some(id))
    }
}

/// Whether nothing exists at `path`. Something there is `false`; a failed
/// lookup is an error, as in `abandon()`.
fn missing(path: &Path) -> Result<bool, StorageError> {
    match abandon::absent(path) {
        Ok(()) => Ok(true),
        Err(StorageError::UnsafeFile) => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "unstarted_tests.rs"]
mod tests;
