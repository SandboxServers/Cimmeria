//! Path-free Play admission and durable lifecycle observation.
use super::*;
use cimmeria_launcher_engine::launch;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod resources;
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum LaunchCommand {
    Inspect {
        schema_version: u32,
    },
    Play {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        installation_id: Uuid,
    },
}
#[derive(Debug, Serialize)]
pub struct LaunchStatus {
    schema_version: u32,
    native: NativeSnapshot,
    installation_id: Option<Uuid>,
    resources_available: bool,
    launcher_update_required: bool,
    observation: Option<launch::Observation>,
}
impl NativeHost {
    pub fn with_launch_resources(mut self, directory: PathBuf) -> Self {
        self.launch_resources = resources::bundled(&directory);
        self
    }
    pub fn launch_command(&self, request: LaunchCommand) -> Result<LaunchStatus, JobError> {
        let version = match &request {
            LaunchCommand::Inspect { schema_version }
            | LaunchCommand::Play { schema_version, .. } => *schema_version,
        };
        if version != 1 {
            return Err(JobError::UnsupportedSchema);
        }
        let store = self.store()?;
        if let LaunchCommand::Play {
            operation_id,
            operation_revision,
            installation_id,
            ..
        } = request
        {
            // Serialize admission and dispatch, including identical simultaneous retries.
            let mut worker = self.launch_worker.lock().map_err(|_| JobError::Io)?;
            let bundled = self
                .launch_resources
                .clone()
                .ok_or(JobError::PlatformUnavailable)?;
            let admission = {
                // Dispatch and its failure path lock the store themselves, so this
                // guard must not outlive admission.
                let mut owner = store.lock().map_err(|_| JobError::Io)?;
                let resources = owner
                    .resolve_play_resources(bundled)?
                    .ok_or(JobError::PlatformUnavailable)?;
                owner.admit_launch(operation_id, operation_revision, installation_id, resources)?
            };
            if admission.dispatch {
                match launch::dispatch(store.clone(), operation_id) {
                    Ok(value) => *worker = Some(value),
                    Err(error) => {
                        if let Ok(mut state) = store.lock() {
                            if let Ok(operations) = state.operations_mut() {
                                let _ = operations.mark_uncertain(operation_id);
                            }
                        }
                        return Err(error.into());
                    }
                }
            }
        }
        let mut state = store.lock().map_err(|_| JobError::Io)?;
        let native = state.dispatch(NativeCommand::Inspect { schema_version: 1 })?;
        let observation = state.launch_observation()?;
        // The build is at fault only when the patch policy needs an artifact it
        // lacks, or a shipped artifact no longer verifies. A state that cannot
        // answer (busy content, refused imported settings) withholds Play without
        // failing Inspect or blaming the bundle.
        let (available, playable) = match &self.launch_resources {
            None => (false, false),
            Some(bundled) => match state.resolve_play_resources(bundled.clone()) {
                Ok(Some(resources)) => {
                    let verified = resources.verify().is_ok();
                    (verified, verified)
                }
                Ok(None) => (false, false),
                Err(_) => (bundled.verify().is_ok(), false),
            },
        };
        let idle = !native.requires_reopen
            && native
                .operation
                .operation
                .as_ref()
                .is_none_or(|op| op.state.terminal());
        let launcher_update_required = idle
            && state
                .installed_launcher_minimum()?
                .is_some_and(|minimum| minimum.blocks());
        let installation_id = if idle && playable && !launcher_update_required {
            state
                .installed_content()?
                .filter(|installed| {
                    if installed.intent.backend.is_native() {
                        cfg!(windows)
                    } else {
                        cfg!(target_os = "macos")
                            && state.prepared_runtime().ok().flatten().is_some()
                    }
                })
                .map(|installed| installed.intent.operation_id)
        } else {
            None
        };
        Ok(LaunchStatus {
            schema_version: 1,
            native,
            installation_id,
            resources_available: available,
            launcher_update_required,
            observation,
        })
    }
}
#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "macos"))]
mod adopted_tests;
#[cfg(all(test, target_os = "macos"))]
mod fixture;
