//! Confirmed maintenance uses durable native Update identity, never renderer paths.
use super::*;
use cimmeria_launcher_engine::{update, OperationState};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceAction {
    Recover,
    Abandon,
    Discard,
    Cleanup,
}
#[derive(Debug, Serialize)]
pub struct Maintenance {
    pub operation_id: Uuid,
    pub directory: PathBuf,
    pub previous_digest: String,
    pub target_digest: String,
    pub recovery: bool,
    pub discard: bool,
    pub rollback: bool,
    pub backup: update::cleanup::BackupStatus,
}
pub(super) fn status(state: &mut DesktopState) -> Result<Option<Maintenance>, JobError> {
    if state.requires_reopen() {
        return Ok(None);
    }
    let Some(plan) = state.update_plan()? else {
        return Ok(None);
    };
    let operation = state
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .ok_or(JobError::UnknownOperation)?;
    let succeeded = operation.state == OperationState::Succeeded;
    let recovery = operation.state == OperationState::ReconciliationRequired;
    let discard = matches!(
        operation.state,
        OperationState::Failed | OperationState::Cancelled
    );
    let backup = if succeeded {
        update::cleanup::status(state)?
    } else {
        update::cleanup::BackupStatus::Unavailable
    };
    Ok(Some(Maintenance {
        operation_id: plan.id,
        directory: plan.owner.destination,
        previous_digest: digest(plan.previous.manifest_digest),
        target_digest: digest(plan.target.manifest_digest),
        recovery,
        discard,
        rollback: succeeded,
        backup,
    }))
}
impl NativeHost {
    pub fn maintain_game_update(
        &self,
        request: GameUpdateCommand,
    ) -> Result<GameUpdateStatus, JobError> {
        request.validate()?;
        let (id, revision, confirmed) = match request {
            GameUpdateCommand::Maintain {
                operation_id,
                operation_revision,
                confirmed,
                ..
            }
            | GameUpdateCommand::Rollback {
                operation_id,
                operation_revision,
                confirmed,
                ..
            } => (operation_id, operation_revision, confirmed),
            _ => return Err(JobError::UnsupportedSchema),
        };
        if !confirmed {
            return Err(JobError::RecoveryRequired);
        }
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| JobError::Io)?;
        let mut worker = self.game_update_worker.lock().map_err(|_| JobError::Io)?;
        let state = self.store()?;
        let plan = state
            .lock()
            .map_err(|_| JobError::Io)?
            .update_plan()?
            .ok_or(JobError::IdentityConflict)?;
        let native = plan.owner.backend.is_native();
        if !self.game_update_backend_supported(native) {
            return Err(JobError::PlatformUnavailable);
        }
        if let GameUpdateCommand::Rollback {
            completed_update, ..
        } = request
        {
            // A duplicate rollback uses its recorded previous identity and remains
            // subject to the engine's target/owner/idempotency checks.
            let expected = if plan.id == id {
                plan.previous
            } else {
                if plan.id != completed_update {
                    return Err(JobError::IdentityConflict);
                }
                plan.target
            };
            let admission = state
                .lock()
                .map_err(|_| JobError::Io)?
                .admit_update_rollback(completed_update, id, revision, expected, confirmed)?;
            if admission.dispatch {
                self.retain_game_update(state, id, native, runtime, &mut worker)?;
            }
        } else if let GameUpdateCommand::Maintain { action, .. } = request {
            if plan.id != id {
                return Err(JobError::IdentityConflict);
            }
            #[cfg(test)]
            let result = if self.game_update_fixture.is_some() {
                match action {
                    MaintenanceAction::Recover => {
                        update::test_support::recover(state, id, revision)
                    }
                    MaintenanceAction::Cleanup => {
                        update::test_support::cleanup(state, id, revision)
                    }
                    MaintenanceAction::Abandon => {
                        update::test_support::abandon(state, id, revision)
                    }
                    MaintenanceAction::Discard => {
                        update::test_support::discard(state, id, revision)
                    }
                }
                .map_err(JobError::from)?
            } else {
                maintenance(state, action, native, id, revision)?
            };
            #[cfg(not(test))]
            let result = maintenance(state, action, native, id, revision)?;
            result
                .blocking_recv()
                .map_err(|_| JobError::PersistenceUncertain)??;
            *worker = None;
        }
        drop(worker);
        self.game_update_status()
    }
}
fn maintenance(
    state: Arc<Mutex<DesktopState>>,
    action: MaintenanceAction,
    native: bool,
    id: Uuid,
    revision: u64,
) -> Result<
    tokio::sync::oneshot::Receiver<Result<(), cimmeria_launcher_engine::IntentError>>,
    JobError,
> {
    if native {
        return match action {
            MaintenanceAction::Recover => update::recovery::recover_native(state, id, revision),
            MaintenanceAction::Abandon => {
                update::abandon::abandon_native(state, id, revision, true)
            }
            MaintenanceAction::Discard => {
                update::discard::discard_native(state, id, revision, true)
            }
            MaintenanceAction::Cleanup => update::cleanup::cleanup_native(state, id, revision),
        }
        .map_err(Into::into);
    }
    #[cfg(target_os = "macos")]
    {
        match action {
            MaintenanceAction::Recover => update::recovery::recover_wine(state, id, revision),
            MaintenanceAction::Abandon => update::abandon::abandon_wine(state, id, revision, true),
            MaintenanceAction::Discard => update::discard::discard_wine(state, id, revision, true),
            MaintenanceAction::Cleanup => update::cleanup::cleanup_wine(state, id, revision),
        }
        .map_err(Into::into)
    }
    #[cfg(not(target_os = "macos"))]
    Err(JobError::PlatformUnavailable)
}
