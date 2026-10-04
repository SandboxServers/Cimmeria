//! Repair IPC retains the engine handoff across preparation and replacement.
use super::*;
use cimmeria_launcher_engine::{repair, IntentError, OperationKind, OperationState};
use serde::Serialize;
use uuid::Uuid;

pub(super) struct Worker {
    id: Uuid,
    preparation: repair::preparation::Preparation,
}
#[derive(Debug, Serialize)]
pub struct RepairStatus {
    pub target: Option<Target>,
    pub directory: Option<PathBuf>,
    pub recovery: bool,
    pub cleanup: bool,
}
#[derive(Debug, Serialize)]
pub struct Target {
    installation_id: Uuid,
    directory: PathBuf,
}
impl NativeHost {
    fn repair_backend_supported(&self, native: bool) -> bool {
        if native {
            return cfg!(windows);
        }
        #[cfg(target_os = "macos")]
        {
            self.helper
                .as_ref()
                .is_some_and(|helper| helper.verify().is_ok())
        }
        #[cfg(not(target_os = "macos"))]
        false
    }
    pub(super) fn repair_status(&self) -> Result<RepairStatus, JobError> {
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| JobError::Io)?;
        let op = state.operations().snapshot().operation.clone();
        let current_repair = op.as_ref().filter(|op| op.kind == OperationKind::Repair);
        let mut status = RepairStatus {
            target: None,
            directory: None,
            recovery: false,
            cleanup: false,
        };
        if op
            .as_ref()
            .is_some_and(|op| op.kind == OperationKind::Uninstall)
        {
            return Ok(status);
        }
        if state.requires_reopen() {
            return Ok(status);
        }
        if let Some(operation) = op.as_ref().filter(|op| !op.state.terminal()) {
            if operation.kind == OperationKind::Repair
                && operation.state == OperationState::ReconciliationRequired
            {
                let plan = state.repair_plan()?.ok_or(JobError::IdentityConflict)?;
                status.directory = Some(plan.installation.destination.clone());
                status.recovery = (plan.installation.backend.is_native() && cfg!(windows))
                    || (!plan.installation.backend.is_native() && cfg!(target_os = "macos"));
            }
            // A retained preparation owns the root file lock. Observation must
            // not reopen it on Windows, or interfere with the worker's handoff.
            return Ok(status);
        }
        // Identity survives damaged or missing game content. Never use preferences.
        let installed = match state.installed_content() {
            Ok(value) => value,
            Err(StorageError::Busy) => None,
            Err(error) => return Err(error.into()),
        };
        if let Some(installed) = installed {
            status.directory = Some(installed.intent.destination.clone());
            let supported = self.repair_backend_supported(installed.intent.backend.is_native());
            if supported && op.as_ref().is_none_or(|op| op.state.terminal()) {
                status.target = Some(Target {
                    installation_id: installed.intent.operation_id,
                    directory: installed.intent.destination,
                });
            }
            status.recovery = supported
                && current_repair
                    .is_some_and(|op| op.state == OperationState::ReconciliationRequired);
            status.cleanup =
                supported && current_repair.is_some_and(|op| op.state == OperationState::Succeeded);
        }
        Ok(status)
    }
    pub(super) fn repair_progress(
        &self,
    ) -> Result<Option<super::install::contract::JobProgress>, JobError> {
        let worker = self.repair_worker.lock().map_err(|_| JobError::Io)?;
        Ok(worker.as_ref().and_then(|worker| {
            worker
                .preparation
                .progress
                .borrow()
                .as_ref()
                .map(super::install::progress)
        }))
    }
    pub(super) fn cancel_repair(&self, id: Uuid) -> Result<bool, JobError> {
        let worker = self.repair_worker.lock().map_err(|_| JobError::Io)?;
        let Some(worker) = worker.as_ref().filter(|worker| worker.id == id) else {
            return Ok(false);
        };
        worker.preparation.request_cancel()?;
        Ok(true)
    }
    pub(super) fn repair_command(&self, request: &InstallCommand) -> Result<bool, JobError> {
        let (id, revision, confirmed) = match request {
            InstallCommand::Repair {
                operation_id,
                operation_revision,
                confirmed,
                ..
            }
            | InstallCommand::RecoverRepair {
                operation_id,
                operation_revision,
                confirmed,
                ..
            }
            | InstallCommand::AbandonRepair {
                operation_id,
                operation_revision,
                confirmed,
                ..
            }
            | InstallCommand::CleanupRepair {
                operation_id,
                operation_revision,
                confirmed,
                ..
            } => (*operation_id, *operation_revision, *confirmed),
            _ => return Ok(false),
        };
        if !confirmed {
            return Err(JobError::RecoveryRequired);
        }
        let state = self.store()?;
        let mut worker = self.repair_worker.lock().map_err(|_| JobError::Io)?;
        if let InstallCommand::Repair {
            installation_id, ..
        } = request
        {
            let mut owner = state.lock().map_err(|_| JobError::Io)?;
            let installed = owner
                .installed_content()?
                .ok_or(JobError::IdentityConflict)?;
            let native = installed.intent.backend.is_native();
            if !self.repair_backend_supported(native) {
                return Err(JobError::PlatformUnavailable);
            }
            let admission = owner.admit_repair(id, revision, *installation_id, confirmed)?;
            drop(owner);
            if !admission.dispatch {
                return Ok(true);
            }
            let result = self.prepare_repair(state.clone(), id, native);
            let mut preparation = match result {
                Ok(preparation) => preparation,
                Err(error) => {
                    mark_uncertain(&state, id);
                    return Err(error);
                }
            };
            // Keep cancellation/progress in the host, and move only the response
            // receiver into a retained task. No view owns the Prepared handoff.
            let (_, empty) = tokio::sync::oneshot::channel();
            let result = std::mem::replace(&mut preparation.result, empty);
            *worker = Some(Worker { id, preparation });
            tokio::runtime::Handle::try_current()
                .map_err(|_| JobError::Io)?
                .spawn(async move {
                    match result.await {
                        Ok(Ok(prepared)) => {
                            let commit = if native {
                                repair::commit::commit_native(state.clone(), prepared)
                            } else {
                                #[cfg(target_os = "macos")]
                                {
                                    repair::commit::commit_wine(state.clone(), prepared)
                                }
                                #[cfg(not(target_os = "macos"))]
                                {
                                    drop(prepared);
                                    Err(IntentError::from(StorageError::Corrupt))
                                }
                            };
                            match commit {
                                Ok(result) => {
                                    if result.await.is_err() {
                                        mark_uncertain(&state, id);
                                    }
                                }
                                Err(_) => mark_uncertain(&state, id),
                            }
                        }
                        Ok(Err(_)) => (), // Engine persisted the authoritative outcome.
                        Err(_) => mark_uncertain(&state, id),
                    }
                });
        } else {
            let native = state
                .lock()
                .map_err(|_| JobError::Io)?
                .repair_plan()?
                .ok_or(JobError::IdentityConflict)?
                .installation
                .backend
                .is_native();
            let result = recovery(state, request, native, id, revision)?;
            // Called by spawn_blocking. Engine worker retains mutation if IPC is lost.
            result
                .blocking_recv()
                .map_err(|_| JobError::PersistenceUncertain)??;
            *worker = None;
        }
        Ok(true)
    }
    fn prepare_repair(
        &self,
        state: Arc<Mutex<DesktopState>>,
        id: Uuid,
        native: bool,
    ) -> Result<repair::preparation::Preparation, JobError> {
        if native {
            return repair::preparation::prepare_native(state, id).map_err(Into::into);
        }
        #[cfg(target_os = "macos")]
        {
            repair::preparation::prepare_wine(
                state,
                id,
                self.helper.clone().ok_or(JobError::PlatformUnavailable)?,
            )
            .map_err(Into::into)
        }
        #[cfg(not(target_os = "macos"))]
        Err(JobError::PlatformUnavailable)
    }
}
fn mark_uncertain(state: &Mutex<DesktopState>, id: Uuid) {
    if let Ok(mut state) = state.lock() {
        if let Ok(operations) = state.operations_mut() {
            let _ = operations.mark_uncertain(id);
        }
    }
}
fn recovery(
    state: Arc<Mutex<DesktopState>>,
    request: &InstallCommand,
    native: bool,
    id: Uuid,
    revision: u64,
) -> Result<tokio::sync::oneshot::Receiver<Result<(), IntentError>>, JobError> {
    if native {
        return match request {
            InstallCommand::RecoverRepair { .. } => {
                repair::recovery::recover_native(state, id, revision)
            }
            InstallCommand::AbandonRepair { .. } => {
                repair::abandon::abandon_native(state, id, revision, true)
            }
            InstallCommand::CleanupRepair { .. } => {
                repair::cleanup::cleanup_native(state, id, revision)
            }
            _ => unreachable!(),
        }
        .map_err(Into::into);
    }
    #[cfg(target_os = "macos")]
    {
        match request {
            InstallCommand::RecoverRepair { .. } => {
                repair::recovery::recover_wine(state, id, revision)
            }
            InstallCommand::AbandonRepair { .. } => {
                repair::abandon::abandon_wine(state, id, revision, true)
            }
            InstallCommand::CleanupRepair { .. } => {
                repair::cleanup::cleanup_wine(state, id, revision)
            }
            _ => unreachable!(),
        }
        .map_err(Into::into)
    }
    #[cfg(not(target_os = "macos"))]
    Err(JobError::PlatformUnavailable)
}

#[cfg(test)]
mod tests;
