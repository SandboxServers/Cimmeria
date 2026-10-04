//! Native-only resource selection and retained prerequisite task ownership.
use super::*;
use uuid::Uuid;
impl NativeHost {
    pub(super) fn runtime_setup_target(
        &self,
        state: &mut DesktopState,
    ) -> Result<Option<Uuid>, StorageError> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = state;
            Ok(None)
        }
        #[cfg(target_os = "macos")]
        {
            use cimmeria_launcher_engine::{ExtractionBackend, OperationKind, OperationState};
            if state.requires_reopen()
                || self
                    .prerequisite_helper
                    .as_ref()
                    .is_none_or(|helper| helper.verify().is_err())
            {
                return Ok(None);
            }
            let eligible = state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| {
                    (op.kind == OperationKind::Install && op.state == OperationState::Succeeded)
                        || (op.kind == OperationKind::PrepareRuntime
                            && matches!(
                                op.state,
                                OperationState::Failed | OperationState::Cancelled
                            ))
                });
            if !eligible {
                return Ok(None);
            }
            let Some(installed) = state.installed_content()? else {
                return Ok(None);
            };
            Ok(
                matches!(installed.intent.backend, ExtractionBackend::Wine { .. })
                    .then_some(installed.intent.operation_id),
            )
        }
    }

    pub(super) fn prepare_runtime(
        &self,
        id: Uuid,
        revision: u64,
        installation: Uuid,
    ) -> Result<(), JobError> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, revision, installation);
            Err(JobError::PlatformUnavailable)
        }
        #[cfg(target_os = "macos")]
        {
            use cimmeria_launcher_engine::{mac_wine::prerequisites, ExtractionBackend};
            let helper = self
                .prerequisite_helper
                .as_ref()
                .ok_or(JobError::PlatformUnavailable)?;
            // Reject changed/missing resources before initializing state or admitting work.
            helper.verify().map_err(|_| JobError::PlatformUnavailable)?;
            let state = self.store()?;
            let mut worker = self.runtime_worker.lock().map_err(|_| JobError::Io)?;
            let admission = {
                let mut owner = state.lock().map_err(|_| JobError::Io)?;
                let installed = owner
                    .installed_content()?
                    .ok_or(JobError::IdentityConflict)?;
                let ExtractionBackend::Wine { runtime_sha256, .. } = installed.intent.backend
                else {
                    return Err(JobError::PlatformUnavailable);
                };
                owner.admit_runtime_setup(
                    id,
                    revision,
                    installation,
                    runtime_sha256,
                    helper.sha256(),
                )?
            };
            if admission.dispatch {
                match prerequisites::dispatch(state.clone(), id, helper.path().into()) {
                    Ok(active) => *worker = Some(active),
                    Err(error) => {
                        if let Ok(mut owner) = state.lock() {
                            if let Ok(operations) = owner.operations_mut() {
                                let _ = operations.mark_uncertain(id);
                            }
                        }
                        return Err(error.into());
                    }
                }
            }
            Ok(())
        }
    }
    pub(super) fn cancel_runtime(&self, id: Uuid) -> Result<bool, JobError> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Ok(false)
        }
        #[cfg(target_os = "macos")]
        {
            let worker = self.runtime_worker.lock().map_err(|_| JobError::Io)?;
            if let Some(active) = worker.as_ref().filter(|worker| worker.operation_id() == id) {
                active.request_cancel()?;
                return Ok(true);
            }
            Ok(false)
        }
    }
}
#[cfg(test)]
mod tests;
