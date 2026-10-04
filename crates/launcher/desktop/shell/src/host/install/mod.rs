//! Restricted operation IPC. Executables, URLs, manifests and paths stay native.
use super::*;
use cimmeria_launcher_engine::{
    catalog::{CatalogError, VerifiedRelease},
    install::Progress,
    install_recovery,
    install_worker::{self, Outcome, ResumeError},
    ContractError, EvidenceError, IntentError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstallCommand {
    Inspect {
        schema_version: u32,
    },
    Install {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        preferences_revision: u64,
    },
    Cancel {
        schema_version: u32,
        operation_id: Uuid,
    },
    Resume {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
    },
    Reconcile {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
    },
}
impl InstallCommand {
    pub fn validate(&self) -> Result<(), JobError> {
        let version = match self {
            Self::Inspect { schema_version }
            | Self::Install { schema_version, .. }
            | Self::Cancel { schema_version, .. }
            | Self::Resume { schema_version, .. }
            | Self::Reconcile { schema_version, .. } => *schema_version,
        };
        if version != 1 {
            return Err(JobError::UnsupportedSchema);
        }
        Ok(())
    }
    pub fn needs_release(&self) -> bool {
        matches!(self, Self::Install { .. })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobError {
    UnsupportedSchema,
    PlatformUnavailable,
    Io,
    CorruptState,
    InvalidDirectory,
    StaleRevision,
    Busy,
    UnknownOperation,
    IdentityConflict,
    RecoveryRequired,
    PersistenceUncertain,
    ManifestUnavailable,
    InvalidManifest,
    SigningKeyUnavailable,
}
impl From<StorageError> for JobError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::InUse | StorageError::Busy => Self::Busy,
            StorageError::Io => Self::Io,
            StorageError::Corrupt | StorageError::TooLarge | StorageError::UnsafeFile => {
                Self::CorruptState
            }
            StorageError::UnsupportedSchema => Self::UnsupportedSchema,
            StorageError::InvalidDirectory => Self::InvalidDirectory,
            StorageError::StaleRevision => Self::StaleRevision,
            StorageError::PersistenceUncertain => Self::PersistenceUncertain,
        }
    }
}
impl From<ContractError> for JobError {
    fn from(error: ContractError) -> Self {
        match error {
            ContractError::UnsupportedSchema => Self::UnsupportedSchema,
            ContractError::InvalidRevision | ContractError::StaleRevision => Self::StaleRevision,
            ContractError::Busy => Self::Busy,
            ContractError::IdentityConflict => Self::IdentityConflict,
            ContractError::UnknownOperation => Self::UnknownOperation,
            ContractError::InvalidTransition => Self::RecoveryRequired,
            ContractError::PersistenceFailed => Self::Io,
            ContractError::PersistenceUncertain => Self::PersistenceUncertain,
        }
    }
}
impl From<IntentError> for JobError {
    fn from(error: IntentError) -> Self {
        match error {
            IntentError::Storage(e) => e.into(),
            IntentError::Operation(e) => e.into(),
        }
    }
}
impl From<CatalogError> for JobError {
    fn from(error: CatalogError) -> Self {
        match error {
            CatalogError::Network => Self::ManifestUnavailable,
            CatalogError::SigningKeyUnavailable => Self::SigningKeyUnavailable,
            _ => Self::InvalidManifest,
        }
    }
}
impl From<EvidenceError> for JobError {
    fn from(error: EvidenceError) -> Self {
        match error {
            EvidenceError::Storage(e) => e.into(),
            EvidenceError::Verification(e) => e.into(),
            EvidenceError::IdentityMismatch => Self::IdentityConflict,
        }
    }
}
impl From<ResumeError> for JobError {
    fn from(error: ResumeError) -> Self {
        match error {
            ResumeError::Evidence(e) => e.into(),
            ResumeError::Intent(e) => e.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InstallStatus {
    pub schema_version: u32,
    pub native: NativeSnapshot,
    pub install_supported: bool,
    pub can_resume: bool,
    pub can_reconcile: bool,
    pub progress: Option<JobProgress>,
    pub outcome: Option<Outcome>,
}
#[derive(Debug, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum JobProgress {
    Download { current: u64, total: u64 },
    Extraction { current: u64, total: u64 },
}
fn progress(value: &Progress) -> JobProgress {
    const MAX: u64 = 9_007_199_254_740_991;
    match value {
        Progress::Downloading {
            downloaded, total, ..
        } => JobProgress::Download {
            current: (*downloaded).min(MAX),
            total: (*total).min(MAX),
        },
        Progress::Extracting { current, total, .. } => JobProgress::Extraction {
            current: (*current as u64).min(MAX),
            total: (*total as u64).min(MAX),
        },
    }
}

impl NativeHost {
    /// Missing or changed native resources reject before any release fetch.
    pub fn require_install_support(&self) -> Result<(), JobError> {
        self.platform_backend()?.verify()
    }
    fn platform_backend(&self) -> Result<PlatformBackend, JobError> {
        if cfg!(windows) {
            return Ok(PlatformBackend::Native);
        }
        #[cfg(target_os = "macos")]
        if let Some(helper) = &self.helper {
            return Ok(PlatformBackend::Wine(helper.clone()));
        }
        Err(JobError::PlatformUnavailable)
    }

    pub fn retry_release(
        &self,
        request: &InstallCommand,
    ) -> Result<Option<VerifiedRelease>, JobError> {
        request.validate()?;
        let InstallCommand::Install {
            operation_id,
            preferences_revision,
            ..
        } = request
        else {
            return Ok(None);
        };
        let state = self.store()?;
        let state = state.lock().map_err(|_| JobError::Io)?;
        if state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .is_none_or(|op| op.id != *operation_id)
        {
            return Ok(None);
        }
        let intent = state.install_intent()?.ok_or(JobError::IdentityConflict)?;
        if intent.preferences_revision != *preferences_revision
            || intent.login_servers
                != cimmeria_launcher_engine::client_setup::login_servers::default_servers()
        {
            return Err(JobError::IdentityConflict);
        }
        Ok(Some(state.cached_install_release()?))
    }

    pub fn install_status(&self) -> Result<InstallStatus, JobError> {
        let (native, native_backend, outcome) = self.with_state(|state| {
            Ok((
                state.inspect(),
                state
                    .install_intent()?
                    .is_some_and(|intent| intent.backend.is_native()),
                state.install_outcome()?,
            ))
        })?;
        let recovery = !native.requires_reopen
            && native.operation.operation.as_ref().is_some_and(|op| {
                op.state == cimmeria_launcher_engine::OperationState::ReconciliationRequired
            });
        let worker = self.worker.lock().map_err(|_| JobError::Io)?;
        let worker = worker.as_ref().filter(|worker| {
            native
                .operation
                .operation
                .as_ref()
                .is_some_and(|operation| operation.id == worker.operation_id())
        });
        let observed = worker.and_then(|worker| worker.progress.borrow().as_ref().map(progress));
        Ok(InstallStatus {
            schema_version: 1,
            native,
            install_supported: self.platform_backend().is_ok(),
            can_resume: recovery && native_backend && cfg!(windows),
            can_reconcile: recovery && (native_backend || cfg!(target_os = "macos")),
            progress: observed,
            outcome,
        })
    }

    /// Called on a native blocking worker. The async adapter fetches only the
    /// fixed signed release for Install; no webview-supplied release is accepted.
    pub fn install_command(
        &self,
        request: InstallCommand,
        release: Option<VerifiedRelease>,
    ) -> Result<InstallStatus, JobError> {
        request.validate()?;
        if matches!(request, InstallCommand::Inspect { .. }) {
            return self.install_status();
        }
        if matches!(
            request,
            InstallCommand::Install { .. } | InstallCommand::Resume { .. }
        ) {
            self.require_install_support()?;
        }
        let state = self.store()?;
        let mut worker = self.worker.lock().map_err(|_| JobError::Io)?;
        match request {
            InstallCommand::Install {
                operation_id,
                operation_revision,
                preferences_revision,
                ..
            } => {
                let release = release.ok_or(JobError::InvalidManifest)?;
                start_install_with(
                    state.clone(),
                    &mut worker,
                    operation_id,
                    operation_revision,
                    preferences_revision,
                    release,
                    self.platform_backend()?,
                )?;
            }
            InstallCommand::Cancel { operation_id, .. } => {
                let active = worker
                    .as_ref()
                    .filter(|worker| worker.operation_id() == operation_id)
                    .ok_or(JobError::UnknownOperation)?;
                active.request_cancel()?;
            }
            InstallCommand::Resume {
                operation_id,
                operation_revision,
                ..
            } => {
                if !cfg!(windows) {
                    return Err(JobError::PlatformUnavailable);
                }
                *worker = Some(install_worker::resume(
                    state.clone(),
                    operation_id,
                    operation_revision,
                )?);
            }
            InstallCommand::Reconcile {
                operation_id,
                operation_revision,
                ..
            } => {
                let mut owner = state.lock().map_err(|_| JobError::Io)?;
                if owner.operations().snapshot().revision != operation_revision {
                    return Err(JobError::StaleRevision);
                }
                if owner
                    .operations()
                    .snapshot()
                    .operation
                    .as_ref()
                    .is_none_or(|operation| operation.id != operation_id)
                {
                    return Err(JobError::UnknownOperation);
                }
                let release = owner.cached_install_release()?;
                install_recovery::reconcile(&mut owner, &release)?;
                // Recovery supersedes the completed worker's old observation.
                *worker = None;
            }
            InstallCommand::Inspect { .. } => unreachable!(),
        }
        drop(worker);
        self.install_status()
    }
}

fn start_install_with(
    state: Arc<Mutex<DesktopState>>,
    worker: &mut Option<install_worker::Worker>,
    id: Uuid,
    operation_revision: u64,
    preferences_revision: u64,
    release: VerifiedRelease,
    backend: PlatformBackend,
) -> Result<(), JobError> {
    backend.verify()?;
    let admission = state
        .lock()
        .map_err(|_| JobError::Io)?
        .admit_install_backend(cimmeria_launcher_engine::AdmissionRequest {
            id,
            operation_revision,
            preferences_revision,
            release: &release,
            login_servers: cimmeria_launcher_engine::client_setup::login_servers::default_servers(),
            backend: backend.identity(),
        })?;
    if admission.dispatch {
        match backend.dispatch(state.clone(), id, release) {
            Ok(active) => *worker = Some(active),
            Err(error) => {
                // Admission succeeded, but dispatch did not. Expose recovery now,
                // rather than leaving a Starting operation stranded until restart.
                if let Ok(mut state) = state.lock() {
                    if let Ok(operations) = state.operations_mut() {
                        let _ = operations.mark_uncertain(id);
                    }
                }
                return Err(error.into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
fn start_install(
    state: Arc<Mutex<DesktopState>>,
    worker: &mut Option<install_worker::Worker>,
    id: Uuid,
    operation_revision: u64,
    preferences_revision: u64,
    release: VerifiedRelease,
) -> Result<(), JobError> {
    start_install_with(
        state,
        worker,
        id,
        operation_revision,
        preferences_revision,
        release,
        PlatformBackend::Native,
    )
}

enum PlatformBackend {
    Native,
    #[cfg(target_os = "macos")]
    Wine(cimmeria_launcher_engine::mac_wine::HelperResource),
}
impl PlatformBackend {
    fn verify(&self) -> Result<(), JobError> {
        match self {
            Self::Native => Ok(()),
            #[cfg(target_os = "macos")]
            Self::Wine(helper) => helper.verify().map_err(|_| JobError::PlatformUnavailable),
        }
    }
    fn identity(&self) -> cimmeria_launcher_engine::ExtractionBackend {
        match self {
            Self::Native => cimmeria_launcher_engine::ExtractionBackend::Native,
            #[cfg(target_os = "macos")]
            Self::Wine(helper) => helper.backend(),
        }
    }
    fn dispatch(
        self,
        state: Arc<Mutex<DesktopState>>,
        id: Uuid,
        release: VerifiedRelease,
    ) -> Result<install_worker::Worker, IntentError> {
        match self {
            Self::Native => install_worker::dispatch(state, id, release),
            #[cfg(target_os = "macos")]
            Self::Wine(helper) => {
                install_worker::dispatch_wine(state, id, release, helper.path().into())
            }
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod resource_tests;
