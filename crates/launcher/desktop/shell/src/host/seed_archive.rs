//! Native-chosen seed archive: the webview names commands only, never a path.
use super::NativeHost;
use cimmeria_launcher_engine::{
    catalog::VerifiedRelease,
    seed_archive::{self, LocalSeedError, VerifyWorker},
    NativeCommand, NativeSnapshot, StorageError,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The webview's seed-archive commands. Each carries only the schema version.
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum SeedArchiveCommand {
    Inspect {
        schema_version: u32,
    },
    /// Ask the running check to stop; nothing is remembered from a cancelled check.
    Cancel {
        schema_version: u32,
    },
    /// Back to download mode. Refused while a check runs.
    Forget {
        schema_version: u32,
    },
}

/// The archive the player chose, by name only: its folder stays native.
#[derive(Debug, Serialize)]
pub struct ChosenSeedArchive {
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct SeedArchiveStatus {
    pub schema_version: u32,
    pub native: NativeSnapshot,
    pub chosen: Option<ChosenSeedArchive>,
    /// Present while a check runs and has reported progress.
    pub verifying: Option<super::install::contract::JobProgress>,
    pub last_error: Option<SeedArchiveError>,
}

/// Flat codes the webview maps to text. A foreign schema version is refused as `io`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedArchiveError {
    NotAFile,
    SizeMismatch,
    HashMismatch,
    Changed,
    Cancelled,
    Busy,
    Io,
    ReleaseUnavailable,
}

impl From<LocalSeedError> for SeedArchiveError {
    fn from(error: LocalSeedError) -> Self {
        match error {
            LocalSeedError::NotAFile => Self::NotAFile,
            LocalSeedError::SizeMismatch { .. } => Self::SizeMismatch,
            LocalSeedError::HashMismatch => Self::HashMismatch,
            LocalSeedError::Changed => Self::Changed,
            LocalSeedError::Cancelled => Self::Cancelled,
            LocalSeedError::Io => Self::Io,
        }
    }
}

impl From<StorageError> for SeedArchiveError {
    fn from(_: StorageError) -> Self {
        Self::Io
    }
}

impl From<&seed_archive::SeedArchiveRecord> for ChosenSeedArchive {
    fn from(record: &seed_archive::SeedArchiveRecord) -> Self {
        Self {
            file_name: record
                .seed
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string(),
            size: record.seed.size,
        }
    }
}

/// The running check, and the outcome of the last one until the webview sees it.
#[derive(Default)]
pub struct SeedArchive {
    worker: Option<VerifyWorker>,
    last_error: Option<SeedArchiveError>,
}

impl SeedArchive {
    /// Moves a finished check into `last_error` and reports whether one still runs.
    pub fn settle(&mut self) -> bool {
        let finished = match &self.worker {
            Some(worker) => *worker.result.borrow(),
            None => None,
        };
        if let Some(outcome) = finished {
            self.worker = None;
            self.last_error = outcome.err().map(SeedArchiveError::from);
        }
        self.worker.is_some()
    }
}

impl NativeHost {
    /// Inspect, cancel or forget the remembered archive.
    pub fn seed_archive_command(
        &self,
        command: SeedArchiveCommand,
    ) -> Result<SeedArchiveStatus, SeedArchiveError> {
        let version = match &command {
            SeedArchiveCommand::Inspect { schema_version }
            | SeedArchiveCommand::Cancel { schema_version }
            | SeedArchiveCommand::Forget { schema_version } => *schema_version,
        };
        if version != 1 {
            return Err(SeedArchiveError::Io);
        }
        let mut archive = self.seed_archive.lock().map_err(|_| SeedArchiveError::Io)?;
        match command {
            SeedArchiveCommand::Inspect { .. } => {}
            SeedArchiveCommand::Cancel { .. } => {
                archive.settle();
                if let Some(worker) = &archive.worker {
                    worker.cancel.cancel();
                }
            }
            SeedArchiveCommand::Forget { .. } => {
                if archive.settle() {
                    return Err(SeedArchiveError::Busy);
                }
                let store = self.store()?;
                let mut state = store.lock().map_err(|_| SeedArchiveError::Io)?;
                state.forget_seed_archive()?;
                archive.last_error = None;
            }
        }
        self.seed_archive_status(&mut archive)
    }

    /// Checks the native-picked `path` against `release` on a retained worker.
    /// Refused while another check runs.
    pub fn choose_seed_archive(
        &self,
        path: PathBuf,
        release: VerifiedRelease,
    ) -> Result<SeedArchiveStatus, SeedArchiveError> {
        let mut archive = self.seed_archive.lock().map_err(|_| SeedArchiveError::Io)?;
        if archive.settle() {
            return Err(SeedArchiveError::Busy);
        }
        let store = self.store()?;
        archive.worker = Some(seed_archive::start_verify(store, release, path));
        archive.last_error = None;
        self.seed_archive_status(&mut archive)
    }

    fn seed_archive_status(
        &self,
        archive: &mut SeedArchive,
    ) -> Result<SeedArchiveStatus, SeedArchiveError> {
        archive.settle();
        let verifying = match &archive.worker {
            Some(worker) => {
                let observed = worker.progress.borrow();
                observed.as_ref().map(super::install::progress)
            }
            None => None,
        };
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| SeedArchiveError::Io)?;
        let native = state.dispatch(NativeCommand::Inspect { schema_version: 1 })?;
        Ok(SeedArchiveStatus {
            schema_version: 1,
            native,
            chosen: state.seed_archive().map(ChosenSeedArchive::from),
            verifying,
            last_error: archive.last_error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_with_unknown_field_is_refused() {
        let json = r#"{"command":"inspect","schema_version":1,"extra":true}"#;
        assert!(serde_json::from_str::<SeedArchiveCommand>(json).is_err());
    }

    #[test]
    fn command_with_a_path_is_refused() {
        let json = r#"{"command":"forget","schema_version":1,"path":"C:\\archive.rar"}"#;
        assert!(serde_json::from_str::<SeedArchiveCommand>(json).is_err());
    }

    #[test]
    fn plain_command_is_accepted() {
        let json = r#"{"command":"cancel","schema_version":1}"#;
        assert!(matches!(
            serde_json::from_str::<SeedArchiveCommand>(json),
            Ok(SeedArchiveCommand::Cancel { schema_version: 1 })
        ));
    }

    #[test]
    fn error_codes_are_snake_case() {
        let code = |error| serde_json::to_string(&error).unwrap();
        assert_eq!(code(SeedArchiveError::NotAFile), "\"not_a_file\"");
        assert_eq!(code(SeedArchiveError::SizeMismatch), "\"size_mismatch\"");
        assert_eq!(code(SeedArchiveError::HashMismatch), "\"hash_mismatch\"");
        assert_eq!(
            code(SeedArchiveError::ReleaseUnavailable),
            "\"release_unavailable\""
        );
        assert_eq!(code(SeedArchiveError::Busy), "\"busy\"");
    }
}
