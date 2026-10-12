//! The player's chosen seed archive, remembered across launches.
//!
//! The record is only a choice. It is checked against the release at the point
//! of use: a record for another seed hash is ignored and the shared seed source
//! falls back to download. Verification runs in [`start_verify`], never on the
//! UI thread.

use super::{atomic, ensure_regular_or_absent, read, DesktopState, StorageError};
use crate::catalog::VerifiedRelease;
use crate::install_progress::ProgressSink;
use cimmeria_launcher_core::seed_source::{
    verify_local_seed, LocalSeedError, SeedSourceHandle, VerifiedLocalSeed,
};
use serde::{Deserialize, Serialize};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

const SEED_ARCHIVE_FILE: &str = "seed-archive.json";

/// A verified local seed and the seed hash of the release it was checked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedArchiveRecord {
    /// The archive as it was verified, including its file identity.
    pub seed: VerifiedLocalSeed,
    /// The signed seed SHA-256 the archive was verified against.
    pub seed_sha256: String,
}

/// Missing, unreadable or corrupt records are ignored: the player simply
/// chooses the archive again.
pub(super) fn load(root: &Path) -> Option<SeedArchiveRecord> {
    read::<SeedArchiveRecord>(&root.join(SEED_ARCHIVE_FILE))
        .ok()
        .flatten()
}

/// Observes one background verification of a chosen archive.
// Until the install, repair and update steps call it.
#[allow(dead_code)]
pub struct VerifyWorker {
    /// Bytes checked so far, as the shared progress sink reports them.
    pub progress: watch::Receiver<Option<crate::install::Progress>>,
    /// `None` while the check runs; then the outcome, once.
    pub result: watch::Receiver<Option<Result<(), LocalSeedError>>>,
}

impl DesktopState {
    /// The seed source for `release`. A remembered archive is applied only when
    /// its hash matches this release; otherwise the handle is in download mode.
    pub fn seed_source_for(&self, release: &VerifiedRelease) -> SeedSourceHandle {
        let expected = &release.manifest().seed.sha256;
        match &self.seed_archive {
            Some(record) if record.seed_sha256.eq_ignore_ascii_case(expected) => {
                if self.seed_source.current().as_ref() != Some(&record.seed) {
                    self.seed_source.use_local(record.seed.clone());
                }
            }
            _ => {
                if self.seed_source.current().is_some() {
                    self.seed_source.clear();
                }
            }
        }
        self.seed_source.clone()
    }

    /// Persists the choice before it is remembered in memory.
    pub fn record_seed_archive(&mut self, record: SeedArchiveRecord) -> Result<(), StorageError> {
        atomic::write(&self.directory.root, SEED_ARCHIVE_FILE, &record)?;
        self.seed_archive = Some(record);
        Ok(())
    }

    /// Back to download mode, and no archive remembered after the next launch.
    pub fn forget_seed_archive(&mut self) -> Result<(), StorageError> {
        self.seed_source.clear();
        self.seed_archive = None;
        let path = self.directory.root.join(SEED_ARCHIVE_FILE);
        ensure_regular_or_absent(&path)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(_) => Err(StorageError::Io),
        }
    }
}

/// Checks `path` against the release's seed on a blocking thread, then makes it
/// the shared seed source and remembers it. Must run inside a Tokio runtime.
#[allow(dead_code)] // Until the install, repair and update steps call it.
pub fn start_verify(
    state: Arc<Mutex<DesktopState>>,
    release: VerifiedRelease,
    path: PathBuf,
) -> VerifyWorker {
    let (progress, observed) = ProgressSink::latest();
    let (send, result) = watch::channel(None);
    tokio::task::spawn_blocking(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            verify_and_remember(&state, &release, &path, &progress)
        }))
        .unwrap_or(Err(LocalSeedError::Io));
        send.send_replace(Some(outcome));
    });
    VerifyWorker {
        progress: observed,
        result,
    }
}

#[allow(dead_code)] // Reached through `start_verify`.
fn verify_and_remember(
    state: &Mutex<DesktopState>,
    release: &VerifiedRelease,
    path: &Path,
    progress: &ProgressSink,
) -> Result<(), LocalSeedError> {
    let seed = &release.manifest().seed;
    let verified = verify_local_seed(path, seed, &CancellationToken::new(), progress)?;
    let mut state = state.lock().map_err(|_| LocalSeedError::Io)?;
    state.seed_source.use_local(verified.clone());
    state
        .record_seed_archive(SeedArchiveRecord {
            seed: verified,
            seed_sha256: seed.sha256.clone(),
        })
        .map_err(|_| LocalSeedError::Io)
}

#[cfg(test)]
mod tests;
