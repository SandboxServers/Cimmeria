//! Path-free Play admission and durable lifecycle observation.
use super::*;
use cimmeria_launcher_engine::launch;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod ledger;
mod resources;
pub(crate) use resources::dll_bundled;
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

impl LaunchCommand {
    /// Start at IPC receipt, before waiting for a blocking worker. Inspect polls
    /// produce no timing records and cannot reset the startup clock.
    pub(crate) fn queue_timer(&self) -> Option<cimmeria_launcher_engine::phase_timing::Timer> {
        match self {
            Self::Play { operation_id, .. } => Some(
                cimmeria_launcher_engine::phase_timing::Context::for_operation(*operation_id)
                    .start(cimmeria_launcher_engine::phase_timing::Phase::PlayDispatchQueue),
            ),
            Self::Inspect { .. } => None,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct LaunchStatus {
    schema_version: u32,
    native: NativeSnapshot,
    installation_id: Option<Uuid>,
    /// Verified game setup, independent of temporary Play admission/update gates.
    setup_complete: bool,
    resources_available: bool,
    launcher_update_required: bool,
    /// Patches the installed copy applied with skipped ops; 0 when Play is not offered.
    patches_incomplete: u32,
    /// The ops those patches skipped.
    ops_skipped: u32,
    observation: Option<launch::Observation>,
}
impl NativeHost {
    /// Packaging probe uses the production resolver, including canonical paths
    /// and player-only telemetry checks, without opening state or launching.
    #[cfg(windows)]
    pub(crate) fn verify_embedded_launch_bundle(directory: &std::path::Path) -> bool {
        resources::bundled(directory).is_some_and(|resources| {
            resources.client_patches.is_some()
                && resources.client_telemetry.is_some()
                && resources.verify().is_ok()
        })
    }
    pub fn with_launch_resources(mut self, directory: PathBuf) -> Self {
        self.launch_resources = resources::bundled(&directory);
        self
    }
    pub fn launch_command(&self, request: LaunchCommand) -> Result<LaunchStatus, JobError> {
        match &request {
            LaunchCommand::Play { operation_id, .. } => {
                cimmeria_launcher_engine::phase_timing::Context::for_operation(*operation_id)
                    .measure(
                        cimmeria_launcher_engine::phase_timing::Phase::PlayRequest,
                        || self.launch_command_timed(request),
                    )
            }
            _ => self.launch_command_timed(request),
        }
    }

    fn launch_command_timed(&self, request: LaunchCommand) -> Result<LaunchStatus, JobError> {
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
                owner.admit_launch_with_updater(
                    operation_id,
                    operation_revision,
                    installation_id,
                    resources,
                    self.updater_config.as_ref(),
                )?
            };
            if admission.dispatch {
                match launch::dispatch(store.clone(), operation_id) {
                    Ok(value) => *worker = Some(value),
                    Err(error) => {
                        if let Ok(mut state) = store.lock() {
                            if let Ok(operations) = state.operations_mut() {
                                if let Err(error) = operations.mark_uncertain(operation_id) {
                                    cimmeria_launcher_engine::launch::events::mark_uncertain_failed(
                                        error,
                                    );
                                }
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
        let launcher_update_required = state
            .launcher_update_blocks_play(self.updater_config.as_ref())?
            || (idle
                && state
                    .installed_launcher_minimum()?
                    .is_some_and(|minimum| minimum.blocks()));
        let known_launch = native.operation.operation.as_ref().is_some_and(|op| {
            op.kind == cimmeria_launcher_engine::OperationKind::Launch
                && op.state != cimmeria_launcher_engine::OperationState::ReconciliationRequired
        }) && observation != Some(launch::Observation::Unknown);
        // prepared_runtime deliberately withholds admission evidence while work
        // is active. A verified admitted launch already captured that evidence;
        // use it only to keep setup controls hidden for the same installation.
        let running_runtime = if !idle && known_launch {
            state.launch_plan()?.and_then(|plan| plan.runtime)
        } else {
            None
        };
        let content = if !native.requires_reopen
            && observation != Some(launch::Observation::Unknown)
            && (idle || known_launch)
            && playable
        {
            state.installed_content()?.filter(|installed| {
                if installed.intent.backend.is_native() {
                    cfg!(windows)
                } else {
                    cfg!(target_os = "macos")
                        && (state.prepared_runtime().ok().flatten().is_some()
                            || running_runtime
                                .as_ref()
                                .is_some_and(|runtime| runtime.installation == installed.intent))
                }
            })
        } else {
            None
        };
        let setup_complete = content.is_some() && state.has_valid_install()?;
        let installed = if idle && !launcher_update_required {
            content
        } else {
            None
        };
        let installation_id = installed
            .as_ref()
            .map(|installed| installed.intent.operation_id);
        let game = installed
            .as_ref()
            .map(|installed| installed.intent.content_root());
        Ok(LaunchStatus::assemble(
            native,
            installation_id,
            setup_complete,
            (available, launcher_update_required),
            game.as_deref(),
            observation,
        ))
    }
}
impl LaunchStatus {
    /// The incomplete-patch counts come from the admitted copy's own ledger (`game`),
    /// so a status with no admitted copy reports 0, 0.
    fn assemble(
        native: NativeSnapshot,
        installation_id: Option<Uuid>,
        setup_complete: bool,
        (resources_available, launcher_update_required): (bool, bool),
        game: Option<&std::path::Path>,
        observation: Option<launch::Observation>,
    ) -> Self {
        let (patches_incomplete, ops_skipped) = ledger::incomplete_counts(game);
        Self {
            schema_version: 1,
            native,
            installation_id,
            setup_complete,
            resources_available,
            launcher_update_required,
            patches_incomplete,
            ops_skipped,
            observation,
        }
    }
}
#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "macos"))]
mod adopted_tests;
#[cfg(all(test, any(windows, target_os = "macos")))]
mod fixture;
#[cfg(all(test, any(windows, target_os = "macos")))]
mod setup_tests;
