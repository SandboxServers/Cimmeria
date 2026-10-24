//! Exclusive state-directory ownership and bounded, crash-aware persistence.
pub mod adoption;
mod atomic;
pub mod client_changes;
pub mod client_maintenance;
mod content_identity;
pub mod display_options;
pub mod effective_settings;
pub(crate) mod extraction_work;
mod failed_cleanup;
pub mod game_telemetry;
mod helper_journal;
pub mod launch;
pub mod launcher_summary;
pub mod log_upload;
pub mod migration;
pub mod repair;
pub mod runtime_setup;
pub mod seed_archive;
pub(crate) mod settings_events;
pub mod uninstall;
pub mod update;
pub mod updater;
pub use helper_journal::{HelperPhase, HelperRecord, HelperResult};
mod release_evidence;
pub use release_evidence::{EvidenceError, ReleaseIdentity};
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
mod admission_tests;
#[cfg(test)]
mod tests;

use crate::{ContractError, Journal, Operations, Snapshot};
use cimmeria_launcher_core::client_setup::login_servers::LoginServer;
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// A login-server list `login_servers::validate` refuses: saved, or about
    /// to be written into `LoginInternal.lua` by Play.
    InvalidServerList,
    /// A display mode or size `DisplayOptions::valid` refuses.
    InvalidDisplayOptions,
}

/// The preferences schema this build writes. Version 1 files still read.
const PREFERENCES_SCHEMA: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredPreferences")]
pub struct Preferences {
    pub schema_version: u32,
    pub revision: u64,
    pub install_directory: Option<PathBuf>,
    /// Launcher journey summaries only, never game/DLL telemetry.
    pub launcher_summary_consent: bool,
    /// Empty means `client_setup::login_servers::default_servers()`.
    pub login_servers: Vec<LoginServer>,
    pub client_patches_enabled: bool,
    /// A default display is stored as no key, so files and records written
    /// before the option keep their exact bytes (R10).
    #[serde(skip_serializing_if = "display_options::DisplayOptions::is_default")]
    pub display: display_options::DisplayOptions,
    /// The monitor per display setup (C-PD8), oldest first, at most 8. Empty is
    /// stored as no key (R10).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub display_placements: Vec<display_options::Placement>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: PREFERENCES_SCHEMA,
            revision: 0,
            install_directory: None,
            launcher_summary_consent: false,
            login_servers: Vec::new(),
            client_patches_enabled: true,
            display: Default::default(),
            display_placements: Vec::new(),
        }
    }
}

/// Every `preferences.json` shape this build reads. Migration happens on
/// deserialization, so records that embed preferences (legacy import,
/// adoption) upgrade the same way as the file itself.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPreferences {
    schema_version: u32,
    revision: u64,
    install_directory: Option<PathBuf>,
    launcher_summary_consent: bool,
    login_servers: Option<Vec<LoginServer>>,
    client_patches_enabled: Option<bool>,
    #[serde(default)]
    display: display_options::DisplayOptions,
    #[serde(default)]
    display_placements: Vec<display_options::Placement>,
}

impl TryFrom<StoredPreferences> for Preferences {
    type Error = &'static str;
    fn try_from(stored: StoredPreferences) -> Result<Self, Self::Error> {
        let (login_servers, client_patches_enabled) = match (
            stored.schema_version,
            stored.login_servers,
            stored.client_patches_enabled,
        ) {
            // v1 predates the client options: it reads as v2 with their
            // defaults and is written back as v2 by the next save.
            (1, None, None) => (Vec::new(), true),
            (PREFERENCES_SCHEMA, Some(servers), Some(enabled)) => (servers, enabled),
            (1 | PREFERENCES_SCHEMA, _, _) => {
                return Err("preferences fields do not match their schema version")
            }
            // A newer schema keeps its number so validation reports
            // UnsupportedSchema, not Corrupt, and the file is left alone.
            (_, servers, enabled) => (servers.unwrap_or_default(), enabled.unwrap_or(true)),
        };
        Ok(Self {
            schema_version: if stored.schema_version == 1 {
                PREFERENCES_SCHEMA
            } else {
                stored.schema_version
            },
            revision: stored.revision,
            install_directory: stored.install_directory,
            launcher_summary_consent: stored.launcher_summary_consent,
            login_servers,
            client_patches_enabled,
            display: stored.display,
            display_placements: stored.display_placements,
        })
    }
}

/// The bytes a record digest covers for one `Preferences` value. A schema-1
/// build digested the four v1 fields only, and an adoption plan it wrote can
/// still be in flight after the upgrade. Preferences whose client options,
/// display and display placements are
/// still the defaults therefore digest in exactly that v1 shape; any other
/// value digests in full, so a change to a v2 field still changes the digest.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum PreferencesDigest<'a> {
    V1 {
        schema_version: u32,
        revision: u64,
        install_directory: &'a Option<PathBuf>,
        launcher_summary_consent: bool,
    },
    Current(&'a Preferences),
}

impl Preferences {
    /// Use this, never the value itself, inside anything that is digested.
    pub(crate) fn digest_form(&self) -> PreferencesDigest<'_> {
        if self.schema_version == PREFERENCES_SCHEMA
            && self.login_servers.is_empty()
            && self.client_patches_enabled
            && self.display.is_default()
            && self.display_placements.is_empty()
        {
            PreferencesDigest::V1 {
                schema_version: 1,
                revision: self.revision,
                install_directory: &self.install_directory,
                launcher_summary_consent: self.launcher_summary_consent,
            }
        } else {
            PreferencesDigest::Current(self)
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
    summaries: launcher_summary::Shared,
}
impl Journal for FileJournal {
    fn commit(&mut self, snapshot: &Snapshot) -> Result<(), ContractError> {
        atomic::write(&self.directory.root, "operation.json", snapshot).map_err(|error| {
            if error == StorageError::PersistenceUncertain {
                ContractError::PersistenceUncertain
            } else {
                ContractError::PersistenceFailed
            }
        })?;
        // Confirmed commits only. The observer cannot fail or alter this result.
        launcher_summary::observe_commit(&self.summaries, snapshot);
        Ok(())
    }
}

/// Own this object for the application's lifetime, behind one command mutex.
/// The root is selected by native app-data resolution, never by an IPC caller.
pub struct DesktopState {
    compatibility: crate::launcher_compatibility::CompatibilityPolicy,
    operations: Operations<FileJournal>,
    preferences: Preferences,
    directory: Arc<Directory>,
    preferences_uncertain: bool,
    summaries: launcher_summary::Shared,
    install_telemetry: game_telemetry::install_event::Pending,
    /// The one seed source every seed step of this process shares.
    seed_source: SeedSourceHandle,
    /// The stored choice, not yet checked against a release. See `seed_archive`.
    seed_archive: Option<seed_archive::SeedArchiveRecord>,
}

impl DesktopState {
    pub fn open(root: &Path) -> Result<Self, StorageError> {
        Self::open_with_compatibility(root, Default::default())
    }

    /// Native composition root only. Identity/order are not renderer input.
    pub fn open_with_compatibility(
        root: &Path,
        compatibility: crate::launcher_compatibility::CompatibilityPolicy,
    ) -> Result<Self, StorageError> {
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
        // Inert until configure_summaries; the summary queue is not read here.
        let summaries =
            launcher_summary::shared(&directory.root, preferences.launcher_summary_consent);
        // Do not overwrite bad/newer state with defaults, including on restore.
        let operations = Operations::restore(
            snapshot,
            FileJournal {
                directory: directory.clone(),
                summaries: summaries.clone(),
            },
        )
        .map_err(|error| match error {
            ContractError::UnsupportedSchema => StorageError::UnsupportedSchema,
            ContractError::InvalidRevision => StorageError::Corrupt,
            ContractError::PersistenceUncertain => StorageError::PersistenceUncertain,
            _ => StorageError::Io,
        })?;
        let seed_archive = seed_archive::load(&directory.root);
        let mut state = Self {
            compatibility,
            operations,
            preferences,
            directory,
            preferences_uncertain: false,
            summaries,
            install_telemetry: Default::default(),
            seed_source: SeedSourceHandle::download(),
            seed_archive,
        };
        state.recover_legacy_import()?;
        state.recover_launcher_update()?;
        Ok(state)
    }

    pub fn launcher_compatibility(&self) -> &crate::launcher_compatibility::CompatibilityPolicy {
        &self.compatibility
    }

    pub(crate) fn state_root(&self) -> &Path {
        &self.directory.root
    }

    pub fn operations(&self) -> &Operations<FileJournal> {
        &self.operations
    }
    /// Native adapter only; IPC exposes specific validated commands.
    pub fn operations_mut(&mut self) -> Result<&mut Operations<FileJournal>, StorageError> {
        self.finalize_summaries();
        self.ensure_updater_idle()?;
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
        self.finalize_summaries();
        if install_directory != self.preferences.install_directory {
            self.ensure_updater_idle()?;
        }
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
            schema_version: PREFERENCES_SCHEMA,
            revision: self.next_preferences_revision()?,
            install_directory,
            launcher_summary_consent,
            login_servers: self.preferences.login_servers.clone(),
            client_patches_enabled: self.preferences.client_patches_enabled,
            display: self.preferences.display,
            display_placements: self.preferences.display_placements.clone(),
        };
        validate_preferences(&next)?;
        // Withdrawal closes the summary gate before the write; granting consent
        // first empties the summary queue, and fails here if it cannot.
        let consent = self.summary_consent_requested(launcher_summary_consent)?;
        let written = write(&self.directory.root, &next);
        match written {
            Ok(()) => self.preferences = next,
            Err(error) => {
                self.preferences_uncertain = error == StorageError::PersistenceUncertain;
            }
        }
        self.summary_consent_written(consent, written);
        written?;
        Ok(self.preferences.clone())
    }

    /// The login-server list and the client-patches switch. Both take effect
    /// at the next Play; neither touches the install path or consent.
    pub fn save_client_options(
        &mut self,
        login_servers: Vec<LoginServer>,
        client_patches_enabled: bool,
        expected_revision: u64,
    ) -> Result<Preferences, StorageError> {
        self.save_client_options_with(
            login_servers,
            client_patches_enabled,
            expected_revision,
            |root, value| atomic::write(root, "preferences.json", value),
        )
    }

    fn save_client_options_with(
        &mut self,
        login_servers: Vec<LoginServer>,
        client_patches_enabled: bool,
        expected_revision: u64,
        write: impl FnOnce(&Path, &Preferences) -> Result<(), StorageError>,
    ) -> Result<Preferences, StorageError> {
        self.finalize_summaries();
        // Like consent, the client options may change while the game runs:
        // Play reads them only at launch. Every other operation (an adoption
        // compares the whole preferences value at recovery) and the launcher
        // updater refuse the save.
        self.ensure_updater_idle()?;
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| !op.state.terminal() && op.kind != crate::OperationKind::Launch)
        {
            return Err(StorageError::Busy);
        }
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        if expected_revision != self.preferences.revision {
            return Err(StorageError::StaleRevision);
        }
        // Empty means the defaults; any other list must be one Play can write.
        if !login_servers.is_empty()
            && cimmeria_launcher_core::client_setup::login_servers::validate(&login_servers)
                .is_err()
        {
            return Err(StorageError::InvalidServerList);
        }
        let next = Preferences {
            schema_version: PREFERENCES_SCHEMA,
            revision: self.next_preferences_revision()?,
            login_servers,
            client_patches_enabled,
            ..self.preferences.clone()
        };
        validate_preferences(&next)?;
        let written = write(&self.directory.root, &next);
        match written {
            Ok(()) => self.preferences = next,
            Err(error) => {
                self.preferences_uncertain = error == StorageError::PersistenceUncertain;
            }
        }
        written?;
        Ok(self.preferences.clone())
    }

    /// Admission for new operations that write to disk (`docs/updater.md`,
    /// "New mutation paths must preserve this rule"): refuse while the
    /// launcher updater or an install, Play, Repair or update is nonterminal.
    pub(crate) fn ensure_no_active_operation(&self) -> Result<(), StorageError> {
        self.ensure_updater_idle()?;
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| !op.state.terminal())
        {
            return Err(StorageError::Busy);
        }
        Ok(())
    }

    fn next_preferences_revision(&self) -> Result<u64, StorageError> {
        self.preferences
            .revision
            .checked_add(1)
            .filter(|n| *n <= MAX_REVISION)
            .ok_or(StorageError::Corrupt)
    }
}

fn validate_preferences(value: &Preferences) -> Result<(), StorageError> {
    if value.schema_version != PREFERENCES_SCHEMA {
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

#[cfg(test)]
mod launcher_minimum_tests;
