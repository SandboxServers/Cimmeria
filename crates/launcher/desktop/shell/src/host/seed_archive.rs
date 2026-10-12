//! Native-chosen seed archive: the webview names commands only, never a path.
use super::install::contract::{progress, JobProgress};
use super::NativeHost;
use cimmeria_launcher_engine::{
    catalog::VerifiedRelease,
    manifest::SeedEntry,
    seed_archive::{self, LocalSeedError, VerifyWorker},
    NativeCommand, NativeSnapshot, StorageError,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A failed release fetch holds off the next one for this long.
const RELEASE_RETRY: Duration = Duration::from_secs(60);

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
    /// Verify the archive found in Downloads. The path stays native: none is sent.
    AcceptOffer {
        schema_version: u32,
    },
}

/// The archive the player chose, by name only: its folder stays native.
#[derive(Debug, Serialize)]
pub struct ChosenSeedArchive {
    pub file_name: String,
    pub size: u64,
}

/// An archive found in Downloads that the player has not accepted yet. Its
/// size is the seed's exact size; its folder stays native.
#[derive(Debug, Serialize)]
pub struct OfferedSeedArchive {
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct SeedArchiveStatus {
    pub schema_version: u32,
    pub native: NativeSnapshot,
    pub chosen: Option<ChosenSeedArchive>,
    /// Present while no archive is chosen and a matching file is in Downloads.
    pub offer: Option<OfferedSeedArchive>,
    /// Present while a check runs: a zero placeholder until its first report.
    pub verifying: Option<JobProgress>,
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

/// The webview's view of an offer: its name and size, never its folder.
fn offered_archive(path: &Path) -> Option<OfferedSeedArchive> {
    let file_name = path.file_name()?.to_str()?.to_string();
    let size = std::fs::metadata(path).ok()?.len();
    Some(OfferedSeedArchive { file_name, size })
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
/// Also caches what the Downloads offer needs, for the session.
#[derive(Default)]
pub struct SeedArchive {
    worker: Option<VerifyWorker>,
    last_error: Option<SeedArchiveError>,
    /// The Downloads file found for the seed. Kept here, never sent to the webview.
    offer: Option<PathBuf>,
    /// The seed of the signed release, once a fetch has succeeded.
    seed: Option<SeedEntry>,
    /// When the last release fetch failed; the next one waits `RELEASE_RETRY`.
    release_failed_at: Option<Instant>,
    /// True while one release fetch runs; a poll never starts a second one.
    release_in_flight: bool,
    /// True once Downloads was scanned since mount or the last Forget.
    scanned: bool,
}

impl SeedArchive {
    /// No seed is cached, no fetch is running, and any failed fetch is old enough to retry.
    fn release_wanted(&self) -> bool {
        self.seed.is_none()
            && !self.release_in_flight
            && match self.release_failed_at {
                None => true,
                Some(failed) => failed.elapsed() >= RELEASE_RETRY,
            }
    }

    /// Takes the right to fetch the release when it is wanted. A `true` answer must
    /// end in a command carrying the release, or in a recorded failure.
    fn claim_release(&mut self) -> bool {
        let wanted = self.release_wanted();
        if wanted {
            self.release_in_flight = true;
        }
        wanted
    }

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
    /// Inspect, cancel, forget or accept the offer for the remembered archive.
    /// `release` is the signed release when the shell could fetch it; `Inspect`
    /// looks for a Downloads offer only when it is present. `AcceptOffer` needs it.
    pub fn seed_archive_command(
        &self,
        command: SeedArchiveCommand,
        release: Option<VerifiedRelease>,
    ) -> Result<SeedArchiveStatus, SeedArchiveError> {
        let version = match &command {
            SeedArchiveCommand::Inspect { schema_version }
            | SeedArchiveCommand::Cancel { schema_version }
            | SeedArchiveCommand::Forget { schema_version }
            | SeedArchiveCommand::AcceptOffer { schema_version } => *schema_version,
        };
        if version != 1 {
            return Err(SeedArchiveError::Io);
        }
        let mut archive = self.seed_archive.lock().map_err(|_| SeedArchiveError::Io)?;
        if let Some(fetched) = &release {
            archive.seed = Some(fetched.manifest().seed.clone());
            archive.release_failed_at = None;
            archive.release_in_flight = false;
        }
        match command {
            SeedArchiveCommand::Inspect { .. } => {
                if archive.settle() {
                    archive.offer = None;
                } else if !archive.scanned {
                    self.scan_downloads(&mut archive)?;
                }
            }
            SeedArchiveCommand::AcceptOffer { .. } => {
                let release = release.ok_or(SeedArchiveError::ReleaseUnavailable)?;
                if archive.settle() {
                    return Err(SeedArchiveError::Busy);
                }
                // Nothing offered (a stale click): report the current status unchanged.
                if let Some(path) = archive.offer.take() {
                    self.start_check(&mut archive, release, path)?;
                }
            }
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
                archive.offer = None;
                archive.scanned = false;
            }
        }
        self.seed_archive_status(&mut archive)
    }

    /// Whether Inspect should fetch the signed release now: no seed is cached, no
    /// fetch runs, and a failed fetch, if any, was at least a minute ago. A `true`
    /// answer claims the fetch; end it with `seed_archive_release_failed` or with a
    /// command carrying the release.
    pub fn seed_archive_release_wanted(&self) -> bool {
        self.seed_archive
            .lock()
            .is_ok_and(|mut archive| archive.claim_release())
    }

    /// Records a failed release fetch, so the next one waits `RELEASE_RETRY`.
    pub fn seed_archive_release_failed(&self) {
        if let Ok(mut archive) = self.seed_archive.lock() {
            archive.release_failed_at = Some(Instant::now());
            archive.release_in_flight = false;
        }
    }

    /// Whether a check runs now. The file dialog is refused while one does.
    pub fn seed_archive_busy(&self) -> bool {
        self.seed_archive
            .lock()
            .is_ok_and(|mut archive| archive.settle())
    }

    /// Starts the check of `path` against `release`, and remembers that release's
    /// seed so the status can name the check's size before its first report.
    fn start_check(
        &self,
        archive: &mut SeedArchive,
        release: VerifiedRelease,
        path: PathBuf,
    ) -> Result<(), SeedArchiveError> {
        let store = self.store()?;
        archive.seed = Some(release.manifest().seed.clone());
        archive.worker = Some(seed_archive::start_verify(store, release, path));
        archive.last_error = None;
        Ok(())
    }

    /// Looks in Downloads for the cached seed and caches the result. Runs once per
    /// mount or Forget; nothing is scanned before a seed is known.
    fn scan_downloads(&self, archive: &mut SeedArchive) -> Result<(), SeedArchiveError> {
        let Some(seed) = &archive.seed else {
            return Ok(());
        };
        let store = self.store()?;
        let chosen = store
            .lock()
            .map_err(|_| SeedArchiveError::Io)?
            .seed_archive()
            .is_some_and(|record| matches_release(&record.seed_sha256, Some(seed)));
        archive.offer = if chosen {
            None
        } else {
            seed_archive::downloads_folder()
                .and_then(|dir| seed_archive::find_downloads_offer(&dir, seed))
        };
        archive.scanned = true;
        Ok(())
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
        self.start_check(&mut archive, release, path)?;
        archive.offer = None;
        self.seed_archive_status(&mut archive)
    }

    fn seed_archive_status(
        &self,
        archive: &mut SeedArchive,
    ) -> Result<SeedArchiveStatus, SeedArchiveError> {
        archive.settle();
        let verifying = verifying_progress(archive);
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| SeedArchiveError::Io)?;
        let native = state.dispatch(NativeCommand::Inspect { schema_version: 1 })?;
        let seed = archive.seed.as_ref();
        Ok(SeedArchiveStatus {
            schema_version: 1,
            native,
            chosen: state
                .seed_archive()
                .filter(|record| matches_release(&record.seed_sha256, seed))
                .map(ChosenSeedArchive::from),
            offer: archive.offer.as_deref().and_then(offered_archive),
            verifying,
            last_error: archive.last_error,
        })
    }
}

/// Whether a remembered archive with `record_sha` is the cached release's seed.
/// Before any release is cached, every remembered archive counts.
fn matches_release(record_sha: &str, seed: Option<&SeedEntry>) -> bool {
    seed.is_none_or(|seed| record_sha.eq_ignore_ascii_case(&seed.sha256))
}

/// The running check's latest progress. Until the first report arrives it is a
/// zero download of the seed's size, so the panel shows the check from the start.
fn verifying_progress(archive: &SeedArchive) -> Option<JobProgress> {
    let worker = archive.worker.as_ref()?;
    let observed = worker.progress.borrow();
    let report = match observed.as_ref() {
        Some(value) => progress(value),
        None => JobProgress::Download {
            current: 0,
            total: archive.seed.as_ref().map_or(0, |seed| seed.size),
        },
    };
    Some(report)
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
    fn accept_offer_with_a_path_is_refused() {
        let json = r#"{"command":"accept_offer","schema_version":1,"path":"C:\\archive.rar"}"#;
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

    #[test]
    fn failed_release_fetch_waits_before_retry() {
        let mut archive = SeedArchive::default();
        assert!(archive.release_wanted());
        archive.release_failed_at = Some(Instant::now());
        assert!(!archive.release_wanted());
        archive.release_failed_at = Instant::now().checked_sub(RELEASE_RETRY);
        assert!(archive.release_wanted());
    }

    #[test]
    fn one_release_fetch_runs_at_a_time() {
        let mut archive = SeedArchive::default();
        assert!(archive.claim_release());
        // A poll while the fetch runs must not start a second one.
        assert!(!archive.claim_release());
        assert!(!archive.release_wanted());
    }

    fn seed_entry(sha256: &str, size: u64) -> SeedEntry {
        SeedEntry {
            blob: String::new(),
            size,
            sha256: sha256.to_string(),
        }
    }

    #[test]
    fn stale_archive_is_not_the_chosen_one() {
        let seed = seed_entry(&"AB".repeat(32), 1);
        assert!(matches_release(&"ab".repeat(32), Some(&seed)));
        assert!(!matches_release(&"cd".repeat(32), Some(&seed)));
        // Before a release is cached, a remembered archive still counts as chosen.
        assert!(matches_release(&"cd".repeat(32), None));
    }

    #[test]
    fn check_that_just_started_reports_a_placeholder() {
        let (_progress_sender, progress) = tokio::sync::watch::channel(None);
        let (_result_sender, result) = tokio::sync::watch::channel(None);
        let archive = SeedArchive {
            worker: Some(VerifyWorker {
                progress,
                result,
                cancel: tokio_util::sync::CancellationToken::new(),
            }),
            seed: Some(seed_entry(&"ab".repeat(32), 4096)),
            ..SeedArchive::default()
        };
        assert!(matches!(
            verifying_progress(&archive),
            Some(JobProgress::Download {
                current: 0,
                total: 4096
            })
        ));
        assert!(verifying_progress(&SeedArchive::default()).is_none());
    }
}
