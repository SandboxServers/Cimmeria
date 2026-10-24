//! Strict webview command/status schema and safe native error projection.
//! The renderer supplies identities, revisions and closed choices only.
use super::*;
use cimmeria_launcher_engine::{ContractError, StorageError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdoptionCommand {
    Inspect {
        schema_version: u32,
    },
    /// Discard the reviewed preview, or stop a preparation that has no review yet.
    Dismiss {
        schema_version: u32,
    },
    /// Ask the retained preparation or copy to stop. A copy honours this only
    /// until publication begins.
    Cancel {
        schema_version: u32,
    },
    Confirm {
        schema_version: u32,
        work_id: Uuid,
        preview_handle: Uuid,
        operation_revision: u64,
        preferences_revision: u64,
        normalize_managed_files: bool,
        accept_unavailable_game_telemetry: bool,
        old_game_closed: bool,
        /// Replace the files the reviewed folder has modified or lost. Absent
        /// means no replacement, so an older renderer cannot opt in by accident.
        #[serde(default)]
        replace_modified_files: bool,
        confirmed: bool,
    },
    Recover {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    Abandon {
        schema_version: u32,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    AbandonPreparation {
        schema_version: u32,
        preparation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
}
impl AdoptionCommand {
    pub fn validate(&self) -> Result<(), AdoptionError> {
        let version = match self {
            Self::Inspect { schema_version }
            | Self::Dismiss { schema_version }
            | Self::Cancel { schema_version }
            | Self::Confirm { schema_version, .. }
            | Self::Recover { schema_version, .. }
            | Self::Abandon { schema_version, .. }
            | Self::AbandonPreparation { schema_version, .. } => *schema_version,
        };
        if version == 1 {
            Ok(())
        } else {
            Err(AdoptionError::UnsupportedSchema)
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionError {
    UnsupportedSchema,
    PlatformUnavailable,
    // Keep this IPC error code shared even when legacy admission is test-only.
    #[cfg_attr(all(target_os = "macos", not(test)), expect(dead_code))]
    ImportRequired,
    UnsupportedCatalog,
    UnsupportedConfiguration,
    ReviewUnavailable,
    SourceChanged,
    /// A lock, prefix or destination this step needs is held or already exists.
    InUse,
    ConsentRequired,
    NoReusableFiles,
    Busy,
    StaleRevision,
    IdentityConflict,
    RecoveryRequired,
    PersistenceUncertain,
    InvalidDirectory,
    UnsafeFile,
    CorruptState,
    Io,
    Cancelled,
    LauncherTooOld,
    InvalidArtifact,
    UnsupportedArchive,
    Network,
    ReleaseUnavailable,
    /// In place: release files in the folder do not match the signed release.
    VerificationFailed,
    /// In place: the folder is not in the installer's full layout.
    UnsupportedLayout,
    /// In place: the game is running from the folder.
    GameRunning,
    /// In place: a release file is in both places and neither copy verifies.
    DuplicateReleaseFile,
    /// In place: launcher-owned entries with content are already in the folder.
    LeftoverLauncherFiles,
    /// The folder cannot be written, so the adoption cannot change it.
    FolderNotWritable,
    /// The selected folder is neither a client root nor its `Working` folder.
    NotAGameFolder,
    /// The backup folder for replaced files already exists.
    BackupConflict,
    /// This adoption mode is not available in this build.
    UnsupportedMode,
}
impl From<StorageError> for AdoptionError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::InUse => Self::InUse,
            StorageError::Busy => Self::Busy,
            StorageError::Io => Self::Io,
            // Only client options and Play check a server list; an adoption
            // never meets one, so it can only mean a damaged state.
            StorageError::Corrupt
            | StorageError::TooLarge
            | StorageError::InvalidServerList
            | StorageError::InvalidDisplayOptions => Self::CorruptState,
            StorageError::UnsafeFile => Self::UnsafeFile,
            StorageError::UnsupportedSchema => Self::UnsupportedSchema,
            StorageError::InvalidDirectory => Self::InvalidDirectory,
            StorageError::StaleRevision => Self::StaleRevision,
            StorageError::PersistenceUncertain => Self::PersistenceUncertain,
        }
    }
}
impl From<ContractError> for AdoptionError {
    fn from(error: ContractError) -> Self {
        match error {
            ContractError::UnsupportedSchema => Self::UnsupportedSchema,
            ContractError::InvalidRevision | ContractError::StaleRevision => Self::StaleRevision,
            ContractError::Busy => Self::Busy,
            ContractError::IdentityConflict | ContractError::UnknownOperation => {
                Self::IdentityConflict
            }
            ContractError::InvalidTransition => Self::RecoveryRequired,
            ContractError::PersistenceFailed => Self::Io,
            ContractError::PersistenceUncertain => Self::PersistenceUncertain,
        }
    }
}
impl From<adoption::Error> for AdoptionError {
    fn from(error: adoption::Error) -> Self {
        match error {
            adoption::Error::Storage(error) => error.into(),
            adoption::Error::Operation(error) => error.into(),
            adoption::Error::SourceChanged => Self::SourceChanged,
            adoption::Error::ConsentRequired => Self::ConsentRequired,
            adoption::Error::NoReusableFiles => Self::NoReusableFiles,
            adoption::Error::UnsupportedCatalog => Self::UnsupportedCatalog,
            adoption::Error::UnsupportedArchive => Self::UnsupportedArchive,
            adoption::Error::UnsupportedConfiguration => Self::UnsupportedConfiguration,
            adoption::Error::InvalidArtifact => Self::InvalidArtifact,
            adoption::Error::Network => Self::Network,
            adoption::Error::Cancelled => Self::Cancelled,
            adoption::Error::LauncherTooOld => Self::LauncherTooOld,
            adoption::Error::VerificationFailed => Self::VerificationFailed,
            adoption::Error::UnsupportedLayout => Self::UnsupportedLayout,
            adoption::Error::GameRunning => Self::GameRunning,
            adoption::Error::DuplicateReleaseFile => Self::DuplicateReleaseFile,
            adoption::Error::LeftoverLauncherFiles => Self::LeftoverLauncherFiles,
            adoption::Error::FolderNotWritable => Self::FolderNotWritable,
            adoption::Error::NotAGameFolder => Self::NotAGameFolder,
            adoption::Error::BackupConflict => Self::BackupConflict,
            adoption::Error::UnsupportedMode => Self::UnsupportedMode,
            adoption::Error::Unsafe(_) => Self::UnsafeFile,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    /// Retained for decoding old verified-copy status fixtures.
    #[allow(dead_code)]
    Available,
    /// The imported folder itself is verified and adopted (D-LX12).
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    InPlace,
    /// This build carries no archive helper matching its pinned identity.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    HelperUnavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Idle,
    Preparing,
    Review,
    Copying,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Imported {
    pub launcher_directory: PathBuf,
    pub game_directory: PathBuf,
    /// Imported configuration this copy cannot honour; shown, never rewritten.
    pub blocker: Option<AdoptionError>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Download,
    Extraction,
    Copy,
}
/// The step a download, extraction or folder scan is on, from the closed table in C16.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressItem {
    Seed,
    Patch,
    FileIndex,
    IndexBuild,
    Reference,
    /// Hashing the picked folder (light adoption); current and total are bytes.
    Scan,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub current: u64,
    pub total: u64,
    /// Absent for copy progress and for labels outside the table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<ProgressItem>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reconciliation {
    /// Interrupted reference preparation. It is never replayed; its private
    /// files can only be removed.
    Preparation { preparation_id: Uuid },
    /// Interrupted copy. Recovery finishes a verified staged copy; abandonment
    /// ends the operation and leaves the copied bytes where they are.
    Copy {
        operation_id: Uuid,
        directory: PathBuf,
        can_recover: bool,
        can_abandon: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Completed {
    pub directory: PathBuf,
    /// A light adoption: every game file was kept and moved into `game`.
    /// Left out of the JSON for a strict in-place adoption or a copy.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub light: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct AdoptionStatus {
    pub schema_version: u32,
    pub native: NativeSnapshot,
    pub backend: Backend,
    pub imported: Option<Imported>,
    /// Adoption modes this build can offer for the selected folder.
    pub modes: Vec<adoption::AdoptionMode>,
    pub activity: Activity,
    pub review: Option<Review>,
    pub progress: Option<Progress>,
    pub cancellable: bool,
    pub reconciliation: Option<Reconciliation>,
    /// Retained preparation files no operation owns any more.
    pub preparations: Vec<Uuid>,
    /// A desktop-owned installation exists, so nothing further can be adopted.
    pub owned: bool,
    /// Authenticated content exists, independent of runtime and Play availability.
    pub has_valid_install: bool,
    pub completed: Option<Completed>,
    pub last_error: Option<AdoptionError>,
    /// The check behind `last_error` when the engine recorded one.
    pub last_rejection: Option<super::rejection_dto::AdoptionRejection>,
    /// The entries a `unsafe_file` refusal names (launcher-adopt-ux D-AU5);
    /// `None` for every other state. Paths are relative to the chosen folder.
    pub unsafe_entries: Option<adoption::UnsafeReport>,
}
