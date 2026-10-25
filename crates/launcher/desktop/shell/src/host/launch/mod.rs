//! Path-free Play admission and durable lifecycle observation.
use super::*;
use cimmeria_launcher_engine::failure::FailureCode;
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
    prerequisites_missing: Vec<&'static str>,
    prerequisites_installing: bool,
    observation: Option<launch::Observation>,
    /// Why the current launch did not start (C-PD3); `null` when none is known.
    failure: Option<FailureCode>,
}
impl NativeHost {
    /// Packaging probe uses the production resolver, including canonical paths
    /// and player-only telemetry checks, without opening state or launching.
    #[cfg(any(windows, target_os = "linux"))]
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
    #[cfg(not(test))]
    fn proton(&self) -> Option<launch::ProtonSelection> {
        resources::proton(None)
    }
    /// Tests never read the player's Steam: no fixture, no Proton (R3).
    #[cfg(test)]
    fn proton(&self) -> Option<launch::ProtonSelection> {
        let (home, tools) = self.proton_fixture.as_ref()?;
        resources::proton(Some((home, tools)))
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
            let proton = self.proton();
            let Some(mut bundled) = self.launch_resources.clone() else {
                launch::events::launch_refused_before_operation("resources_unavailable");
                return Err(JobError::PlatformUnavailable);
            };
            bundled.proton = proton.clone();
            let admission = {
                // Dispatch and its failure path lock the store themselves, so this
                // guard must not outlive admission.
                let mut owner = store.lock().map_err(|_| JobError::Io)?;
                let game = owner
                    .installed_content()?
                    .map(|installed| installed.intent.content_root());
                let mut missing = self.missing_prerequisites(game.as_deref());
                missing.extend(resources::missing_proton(proton.as_ref()));
                if !missing.is_empty() {
                    tracing::warn!(target:"launcher.launch",event="prerequisites_missing",directx=missing.contains(&"directx"),physx=missing.contains(&"physx"),proton=missing.contains(&"proton"),"Play refused: required game components are missing");
                    return Err(JobError::PrerequisitesMissing);
                }
                let Some(resources) = owner.resolve_play_resources(bundled)? else {
                    launch::events::launch_refused_before_operation("resources_unresolved");
                    return Err(JobError::PlatformUnavailable);
                };
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
                    // A dispatch that failed before anything was recorded ends
                    // Failed with its reason; anything else stays uncertain.
                    Err(error)
                        if launch::dispatch_failed(
                            &store,
                            operation_id,
                            &error,
                            JobError::from(error).failure(),
                        ) =>
                    {
                        return Err(error.into());
                    }
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
        let failure = state.launch_failure()?;
        // The build is at fault only when the patch policy needs an artifact it
        // lacks, or a shipped artifact no longer verifies. A state that cannot
        // answer (busy content, refused imported settings) withholds Play without
        // failing Inspect or blaming the bundle.
        let proton = self.proton();
        let bundled = self.launch_resources.clone().map(|mut bundle| {
            bundle.proton = proton.clone();
            bundle
        });
        let (available, playable) = match &bundled {
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
                    // Linux Native is playable once a Proton is chosen (C-1); the
                    // prefix is prepared by the first Play (D-PP7), not here.
                    cfg!(windows)
                        || (cfg!(target_os = "linux")
                            && bundled
                                .as_ref()
                                .is_some_and(|bundle| bundle.native_admitted()))
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
        let mut missing = self.missing_prerequisites(game.as_deref());
        missing.extend(resources::missing_proton(proton.as_ref()));
        let mut status = LaunchStatus::assemble(
            native,
            installation_id,
            setup_complete,
            (available, launcher_update_required),
            game.as_deref(),
            observation,
            failure,
        );
        status.prerequisites_missing = missing;
        status.prerequisites_installing = state.prerequisite_setup_active();
        Ok(status)
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
        failure: Option<FailureCode>,
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
            prerequisites_missing: Vec::new(),
            prerequisites_installing: false,
            observation,
            failure,
        }
    }
}
#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "macos"))]
mod adopted_tests;
#[cfg(all(test, any(windows, target_os = "macos")))]
mod fixture;
#[cfg(all(test, target_os = "linux"))]
mod proton_status_tests;
#[cfg(all(test, any(windows, target_os = "macos")))]
mod setup_tests;
