//! Exclusive state-directory ownership and bounded, crash-aware persistence.
mod atomic;
pub(crate) mod extraction_work;
mod failed_cleanup;
mod helper_journal;
pub mod launch;
pub mod migration;
pub mod repair;
pub mod runtime_setup;
pub mod uninstall;
pub use helper_journal::{HelperPhase, HelperRecord, HelperResult};
mod release_evidence;
pub use release_evidence::EvidenceError;
mod install_intent;
pub mod install_recovery;
mod install_result;
mod installed_content;
pub use installed_content::InstalledContent;
pub mod install_worker;
pub use install_intent::{
    AdmissionRequest, ExtractionBackend, InstallAdmission, InstallIntent, IntentError,
};
#[cfg(test)]
mod tests;

use crate::{ContractError, Journal, Operations, Snapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

const MAX_STATE_BYTES: u64 = 64 * 1024;
const MAX_REVISION: u64 = 9_007_199_254_740_991;

/// Safe codes for IPC. Detailed filesystem errors must remain local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageError {
    InUse,
    Io,
    Corrupt,
    UnsupportedSchema,
    TooLarge,
    UnsafeFile,
    InvalidDirectory,
    StaleRevision,
    Busy,
    PersistenceUncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u32,
    pub revision: u64,
    pub install_directory: Option<PathBuf>,
    /// Launcher journey summaries only, never game/DLL telemetry.
    pub launcher_summary_consent: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            install_directory: None,
            launcher_summary_consent: false,
        }
    }
}

struct Directory {
    root: PathBuf,
    lock: File,
}
impl Drop for Directory {
    fn drop(&mut self) {
        // Keep the lock file on disk: removing it would permit two inode owners.
        let _ = self.lock.unlock();
    }
}

pub struct FileJournal {
    directory: Arc<Directory>,
}
impl Journal for FileJournal {
    fn commit(&mut self, snapshot: &Snapshot) -> Result<(), ContractError> {
        atomic::write(&self.directory.root, "operation.json", snapshot).map_err(|error| {
            if error == StorageError::PersistenceUncertain {
                ContractError::PersistenceUncertain
            } else {
                ContractError::PersistenceFailed
            }
        })
    }
}

/// Own this object for the application's lifetime, behind one command mutex.
/// The root is selected by native app-data resolution, never by an IPC caller.
pub struct DesktopState {
    operations: Operations<FileJournal>,
    preferences: Preferences,
    directory: Arc<Directory>,
    preferences_uncertain: bool,
}

impl DesktopState {
    pub fn open(root: &Path) -> Result<Self, StorageError> {
        std::fs::create_dir_all(root).map_err(|_| StorageError::Io)?;
        if std::fs::symlink_metadata(root)
            .map_err(|_| StorageError::Io)?
            .file_type()
            .is_symlink()
        {
            return Err(StorageError::UnsafeFile);
        }
        let root = root.canonicalize().map_err(|_| StorageError::Io)?;
        ensure_regular_or_absent(&root.join("launcher.lock"))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("launcher.lock"))
            .map_err(|_| StorageError::Io)?;
        match lock.try_lock() {
            Ok(()) => (),
            Err(std::fs::TryLockError::WouldBlock) => return Err(StorageError::InUse),
            Err(std::fs::TryLockError::Error(_)) => return Err(StorageError::Io),
        }
        let directory = Arc::new(Directory { root, lock });
        let preferences: Preferences =
            read(&directory.root.join("preferences.json"))?.unwrap_or_default();
        validate_preferences(&preferences)?;
        let snapshot: Snapshot = read(&directory.root.join("operation.json"))?.unwrap_or_default();
        // Do not overwrite bad/newer state with defaults, including on restore.
        let operations = Operations::restore(
            snapshot,
            FileJournal {
                directory: directory.clone(),
            },
        )
        .map_err(|error| match error {
            ContractError::UnsupportedSchema => StorageError::UnsupportedSchema,
            ContractError::InvalidRevision => StorageError::Corrupt,
            ContractError::PersistenceUncertain => StorageError::PersistenceUncertain,
            _ => StorageError::Io,
        })?;
        let mut state = Self {
            operations,
            preferences,
            directory,
            preferences_uncertain: false,
        };
        state.recover_legacy_import()?;
        Ok(state)
    }

    pub(crate) fn state_root(&self) -> &Path {
        &self.directory.root
    }

    pub fn operations(&self) -> &Operations<FileJournal> {
        &self.operations
    }
    /// Native adapter only; IPC exposes specific validated commands.
    pub fn operations_mut(&mut self) -> Result<&mut Operations<FileJournal>, StorageError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        Ok(&mut self.operations)
    }
    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }
    pub fn requires_reopen(&self) -> bool {
        self.preferences_uncertain || self.operations.requires_reopen()
    }

    /// Save before acknowledgement. Consent may change during an operation;
    /// the install directory may not change while native ownership is active.
    pub fn save_preferences(
        &mut self,
        install_directory: Option<PathBuf>,
        launcher_summary_consent: bool,
        expected_revision: u64,
    ) -> Result<Preferences, StorageError> {
        self.save_preferences_with(
            install_directory,
            launcher_summary_consent,
            expected_revision,
            |root, value| atomic::write(root, "preferences.json", value),
        )
    }

    fn save_preferences_with(
        &mut self,
        install_directory: Option<PathBuf>,
        launcher_summary_consent: bool,
        expected_revision: u64,
        write: impl FnOnce(&Path, &Preferences) -> Result<(), StorageError>,
    ) -> Result<Preferences, StorageError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        if expected_revision != self.preferences.revision {
            return Err(StorageError::StaleRevision);
        }
        if install_directory != self.preferences.install_directory
            && self
                .operations
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| !op.state.terminal())
        {
            return Err(StorageError::Busy);
        }
        let next = Preferences {
            schema_version: 1,
            revision: self
                .preferences
                .revision
                .checked_add(1)
                .filter(|n| *n <= MAX_REVISION)
                .ok_or(StorageError::Corrupt)?,
            install_directory,
            launcher_summary_consent,
        };
        validate_preferences(&next)?;
        if let Err(error) = write(&self.directory.root, &next) {
            self.preferences_uncertain = error == StorageError::PersistenceUncertain;
            return Err(error);
        }
        self.preferences = next;
        Ok(self.preferences.clone())
    }
}

fn validate_preferences(value: &Preferences) -> Result<(), StorageError> {
    if value.schema_version != 1 {
        return Err(StorageError::UnsupportedSchema);
    }
    if value.revision > MAX_REVISION {
        return Err(StorageError::Corrupt);
    }
    if value
        .install_directory
        .as_ref()
        .is_some_and(|path| !path.is_absolute() || path.parent().is_none())
    {
        return Err(StorageError::InvalidDirectory);
    }
    Ok(())
}

fn ensure_regular_or_absent(path: &Path) -> Result<(), StorageError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => Err(StorageError::UnsafeFile),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StorageError::Io),
    }
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, StorageError> {
    ensure_regular_or_absent(path)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(StorageError::Io),
    };
    read_open(&file).map(Some)
}

// Read through an already-owned handle. Reopening an exclusively locked file
// would fail on Windows even from this same process.
fn read_open<T: serde::de::DeserializeOwned>(file: &File) -> Result<T, StorageError> {
    let mut bytes = Vec::new();
    file.take(MAX_STATE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StorageError::Io)?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(StorageError::TooLarge);
    }
    serde_json::from_slice(&bytes).map_err(|_| StorageError::Corrupt)
}
