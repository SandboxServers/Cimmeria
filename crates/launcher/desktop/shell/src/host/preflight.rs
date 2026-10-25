//! The `preflight_command` (PD-10, C-PD4): gathers what an operation needs,
//! evaluates it with the engine's pure rules, logs `preflight_checked` (C-PD5)
//! and returns the report. `Install` installs one dependency outside the
//! operation journal, refused while any operation is open (D-PD13), and then
//! checks again. Every `Err` logs `preflight_refused` once, in `run` (C-PD5).
use super::windows_prerequisites::SetupLease;
use super::{JobError, NativeHost};
use cimmeria_launcher_engine::catalog::{CatalogError, Purpose};
#[cfg(target_os = "macos")]
use cimmeria_launcher_engine::preflight::events::{Dependency, DependencyOutcome};
use cimmeria_launcher_engine::preflight::{
    self, events, rosetta_install, CheckId, Facts, ManifestProbe, PreflightOperation, Report,
    RosettaProbe,
};
use serde::Deserialize;
use std::sync::Arc;

/// A preflight request from the webview; it names no path.
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreflightRequest {
    Inspect {
        schema_version: u32,
        operation: PreflightOperation,
    },
    Install {
        schema_version: u32,
        operation: PreflightOperation,
        check: CheckId,
    },
}

impl PreflightRequest {
    fn schema_version(&self) -> u32 {
        match self {
            Self::Inspect { schema_version, .. } | Self::Install { schema_version, .. } => {
                *schema_version
            }
        }
    }

    fn operation(&self) -> PreflightOperation {
        match self {
            Self::Inspect { operation, .. } | Self::Install { operation, .. } => *operation,
        }
    }
}

/// The facts only an async probe gives, gathered before the blocking half.
/// `None` leaves the check out of the report.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct AsyncFacts {
    pub manifest: Option<ManifestProbe>,
    pub rosetta: Option<RosettaProbe>,
}

/// Checks what `operation` needs, or installs one dependency and checks again.
#[tauri::command]
pub(crate) async fn preflight_command(
    request: PreflightRequest,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<Report, JobError> {
    run(state.inner().clone(), request).await
}

pub(super) async fn run(
    host: Arc<NativeHost>,
    request: PreflightRequest,
) -> Result<Report, JobError> {
    let operation = request.operation();
    let (command, check) = match &request {
        PreflightRequest::Inspect { .. } => ("inspect", None),
        PreflightRequest::Install { check, .. } => ("install", Some(*check)),
    };
    let result = checked(host, request).await;
    if let Err(error) = &result {
        events::preflight_refused(operation, command, check, error.code(), error.failure());
    }
    result
}

async fn checked(host: Arc<NativeHost>, request: PreflightRequest) -> Result<Report, JobError> {
    if request.schema_version() != 1 {
        return Err(JobError::UnsupportedSchema);
    }
    let operation = request.operation();
    if let PreflightRequest::Install { check, .. } = request {
        install(host.clone(), check).await?;
    }
    inspect(host, operation).await
}

async fn inspect(host: Arc<NativeHost>, operation: PreflightOperation) -> Result<Report, JobError> {
    // The manifest matters only to the operations that download (D-PD7).
    let manifest = match operation {
        PreflightOperation::Play => None,
        PreflightOperation::Install | PreflightOperation::Repair => Some(manifest_probe(
            cimmeria_launcher_engine::catalog::fetch_release_for(Purpose::Preflight)
                .await
                .map(drop),
        )),
    };
    let rosetta = if host.preflight_wine_backend() {
        Some(preflight::mac::rosetta().await)
    } else {
        None
    };
    let facts = AsyncFacts { manifest, rosetta };
    tauri::async_runtime::spawn_blocking(move || host.preflight_inspect(operation, facts))
        .await
        .map_err(|_| JobError::Io)?
}

/// What an Install request may install: only Rosetta and the Wine runtime,
/// and only when the Wine backend applies (R-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Installable {
    Rosetta,
    WineRuntime,
}

pub(super) fn installable(check: CheckId, wine_backend: bool) -> Option<Installable> {
    match check {
        CheckId::Rosetta if wine_backend => Some(Installable::Rosetta),
        CheckId::WineRuntime if wine_backend => Some(Installable::WineRuntime),
        _ => None,
    }
}

/// Installs `check`'s dependency while holding the shared prerequisite lease,
/// which refuses with `Busy` while any operation, installer or the updater
/// runs (D-PD13). Only Rosetta and, on macOS, the Wine runtime install here.
async fn install(host: Arc<NativeHost>, check: CheckId) -> Result<(), JobError> {
    let leasing = host.clone();
    let lease = tauri::async_runtime::spawn_blocking(move || leasing.begin_dependency_install())
        .await
        .map_err(|_| JobError::Io)??;
    let result = match installable(check, host.preflight_wine_backend()) {
        Some(Installable::Rosetta) => {
            // The outcome shows in the re-run report; PD-09 logs both events.
            rosetta_install::install_rosetta().await;
            Ok(())
        }
        #[cfg(target_os = "macos")]
        Some(Installable::WineRuntime) => match host.root.canonicalize() {
            Ok(root) => {
                install_wine_runtime(root.join("runtimes")).await;
                Ok(())
            }
            Err(_) => Err(JobError::Io),
        },
        #[cfg(not(target_os = "macos"))]
        Some(Installable::WineRuntime) => Err(JobError::PlatformUnavailable),
        None => Err(JobError::PlatformUnavailable),
    };
    drop(lease);
    result
}

/// Downloads the pinned Wine runtime into the shared cache under the state
/// root, logging the dependency events. A failure is logged and shows in the
/// next report.
#[cfg(target_os = "macos")]
async fn install_wine_runtime(runtimes: std::path::PathBuf) {
    use cimmeria_launcher_engine::mac_runtime::{self, RuntimeError};
    events::dependency_install_started(
        Dependency::WineRuntime,
        Some(preflight::WINE_RUNTIME_BYTES),
    );
    let started = std::time::Instant::now();
    let result = mac_runtime::prepare(
        runtimes,
        Default::default(),
        cimmeria_launcher_engine::install_progress::ProgressSink::latest().0,
    )
    .await;
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let failure = result.err().map(|error| {
        use cimmeria_launcher_engine::failure::FailureCode;
        match error {
            RuntimeError::Network => FailureCode::WineRuntimeDownloadFailed,
            RuntimeError::Verification | RuntimeError::Extraction => {
                FailureCode::WineRuntimeInvalid
            }
            RuntimeError::Busy => FailureCode::WineRuntimeBusy,
            RuntimeError::Io | RuntimeError::Cancelled => FailureCode::WineRuntimeUnavailable,
        }
    });
    let outcome = if failure.is_none() {
        DependencyOutcome::Installed
    } else {
        DependencyOutcome::Failed
    };
    events::dependency_install_finished(
        Dependency::WineRuntime,
        outcome,
        failure,
        elapsed_ms,
        None,
    );
}

/// The release manifest fact from a catalog fetch.
pub(super) fn manifest_probe(result: Result<(), CatalogError>) -> ManifestProbe {
    match result {
        Ok(()) => ManifestProbe::Ok,
        Err(CatalogError::Network) => ManifestProbe::Unreachable,
        Err(CatalogError::SigningKeyUnavailable | CatalogError::DevelopmentSigningKey) => {
            ManifestProbe::SigningKeyUnavailable
        }
        Err(_) => ManifestProbe::Invalid,
    }
}

impl NativeHost {
    /// Whether the Wine backend applies: macOS with the bundled helper.
    fn preflight_wine_backend(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.helper.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// Takes the shared prerequisite lease; `Busy` while anything else runs.
    fn begin_dependency_install(&self) -> Result<SetupLease, JobError> {
        let store = self.store()?;
        store
            .lock()
            .map_err(|_| JobError::Io)?
            .begin_prerequisite_setup()?;
        Ok(SetupLease(store))
    }

    /// The blocking half: gathers the facts, evaluates them and logs the result.
    pub(super) fn preflight_inspect(
        &self,
        operation: PreflightOperation,
        early: AsyncFacts,
    ) -> Result<Report, JobError> {
        let facts = self.preflight_facts(operation, early)?;
        let report = preflight::evaluate(operation, &facts);
        events::preflight_checked(&report);
        Ok(report)
    }

    fn preflight_facts(
        &self,
        operation: PreflightOperation,
        early: AsyncFacts,
    ) -> Result<Facts, JobError> {
        let store = self.store()?;
        let (directory, helper_available) = {
            let state = store.lock().map_err(|_| JobError::Io)?;
            let directory = state.preferences().install_directory.clone();
            (directory, self.preflight_helper_available(&state)?)
        };
        // Play and Repair act on a folder that is already there; Install creates it.
        let must_exist = operation != PreflightOperation::Install;
        // Only the macOS and Windows branches below add facts.
        #[cfg_attr(not(any(target_os = "macos", windows)), allow(unused_mut))]
        let mut facts = Facts {
            rosetta: early.rosetta,
            helper_available,
            free_bytes: directory.as_deref().and_then(preflight::free_bytes),
            folder: Some(preflight::folder_facts(directory.as_deref(), must_exist)),
            manifest: early.manifest,
            ..Facts::default()
        };
        #[cfg(target_os = "macos")]
        {
            if self.preflight_wine_backend() {
                let root = self.root.canonicalize().map_err(|_| JobError::Io)?;
                facts.wine_runtime_cached = Some(preflight::mac::wine_runtime_cached(&root));
            }
            let exe = std::env::current_exe().map_err(|_| JobError::Io)?;
            facts.translocated = Some(preflight::mac::translocated(&exe));
        }
        #[cfg(windows)]
        {
            use cimmeria_launcher_engine::runtime_setup::windows_physx;
            facts.physx_installed =
                Some(windows_physx::detect(&windows_physx::native::System).core_present);
        }
        Ok(facts)
    }

    /// macOS: the bundled helper opened. Windows: the bundled Play resources
    /// resolve, the check Play makes before admission. Elsewhere: not checked.
    fn preflight_helper_available(
        &self,
        state: &cimmeria_launcher_engine::DesktopState,
    ) -> Result<Option<bool>, JobError> {
        #[cfg(target_os = "macos")]
        {
            let _ = state;
            Ok(Some(self.helper.is_some()))
        }
        #[cfg(windows)]
        {
            let resolved = match self.launch_resources.clone() {
                Some(bundled) => state.resolve_play_resources(bundled)?.is_some(),
                None => false,
            };
            Ok(Some(resolved))
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            let _ = state;
            Ok(None)
        }
    }
}

#[cfg(test)]
#[path = "preflight_tests.rs"]
mod tests;
