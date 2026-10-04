//! Native-only resource selection and retained prerequisite task ownership.
use super::*;
use uuid::Uuid;
impl NativeHost {
    pub(super) fn runtime_can_reconcile(&self, state: &DesktopState) -> bool {
        #[cfg(target_os = "macos")]
        {
            cimmeria_launcher_engine::mac_wine::prerequisites::can_reconcile(state)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = state;
            false
        }
    }

    /// The Tauri adapter retains this future independently of the webview reply.
    /// None delegates content-install reconciliation to its existing coordinator.
    pub async fn reconcile_runtime(
        self: Arc<Self>,
        id: Uuid,
        revision: u64,
    ) -> Result<Option<InstallStatus>, JobError> {
        let host = self.clone();
        let state = tauri::async_runtime::spawn_blocking(move || {
            let state = host.store()?;
            let is_runtime = state
                .lock()
                .map_err(|_| JobError::Io)?
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| {
                    op.kind == cimmeria_launcher_engine::OperationKind::PrepareRuntime
                });
            Ok::<_, JobError>(is_runtime.then_some(state))
        })
        .await
        .map_err(|_| JobError::Io)??;
        let Some(state) = state else {
            return Ok(None);
        };
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (state, id, revision);
            Err(JobError::PlatformUnavailable)
        }
        #[cfg(target_os = "macos")]
        {
            cimmeria_launcher_engine::mac_wine::prerequisites::reconcile(state, id, revision)
                .await?;
            tauri::async_runtime::spawn_blocking(move || self.install_status().map(Some))
                .await
                .map_err(|_| JobError::Io)?
        }
    }

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
