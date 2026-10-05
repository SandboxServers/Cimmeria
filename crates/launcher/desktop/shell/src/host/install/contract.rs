//! Strict webview command/status schema and safe native error projection.
use super::*;
use cimmeria_launcher_engine::{
    catalog::CatalogError, install::Progress, install_worker::ResumeError, ContractError,
    EvidenceError,
};
use serde::{Deserialize, Serialize};
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
    PrepareRuntime {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        installation_id: Uuid,
    },
    Uninstall {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        installation_id: Uuid,
        confirmed: bool,
    },
    CleanFailed {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    Repair {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        installation_id: Uuid,
        confirmed: bool,
    },
    RecoverRepair {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    AbandonRepair {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    CleanupRepair {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
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
            | Self::PrepareRuntime { schema_version, .. }
            | Self::Uninstall { schema_version, .. }
            | Self::CleanFailed { schema_version, .. }
            | Self::Repair { schema_version, .. }
            | Self::RecoverRepair { schema_version, .. }
            | Self::AbandonRepair { schema_version, .. }
            | Self::CleanupRepair { schema_version, .. }
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
    LauncherTooOld,
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
            IntentError::LauncherTooOld => Self::LauncherTooOld,
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
    pub can_retry: bool,
    pub repair: super::super::repair::RepairStatus,
    pub runtime_setup: Option<Uuid>,
    pub uninstall: Option<cimmeria_launcher_engine::uninstall::Target>,
    pub progress: Option<JobProgress>,
    pub outcome: Option<Outcome>,
}
#[derive(Debug, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum JobProgress {
    Download { current: u64, total: u64 },
    Extraction { current: u64, total: u64 },
}
pub(crate) fn progress(value: &Progress) -> JobProgress {
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
