//! A player's own seed archive can replace the seed download.
//!
//! The player picks a file; [`verify_local_seed`] checks its size and SHA-256
//! against the signed seed entry and records its identity. Install then copies
//! it with [`VerifiedLocalSeed::copy_into`], which re-checks identity, size and
//! hash before the copy is renamed into place. Nothing here writes, moves or
//! deletes the player's file: it is only ever opened for reading.

use std::fs::{File, Metadata};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::install::Progress;
use crate::install_progress::{ProgressReporter, ProgressSink};
use crate::manifest::SeedEntry;

/// Read size for the streamed hash and the copy.
const READ_CHUNK: usize = 1024 * 1024;
/// Label of the progress events emitted while the archive is checked or copied.
const CHECK_LABEL: &str = "Checking your game archive";

/// What the seed step uses. Shared by Install, Repair, Update and adoption.
#[derive(Clone)]
pub struct SeedSourceHandle {
    /// The archive the seed step uses, when the player has chosen one.
    current: Arc<Mutex<Option<VerifiedLocalSeed>>>,
    /// Fired by [`SeedSourceHandle::use_local`] to stop a running download.
    switch: CancellationToken,
}

impl SeedSourceHandle {
    /// A handle that downloads the seed, until a local archive is chosen.
    pub fn download() -> Self {
        Self::with_current(None)
    }

    /// A handle that uses `seed` for every seed step.
    pub fn local(seed: VerifiedLocalSeed) -> Self {
        Self::with_current(Some(seed))
    }

    fn with_current(current: Option<VerifiedLocalSeed>) -> Self {
        Self {
            current: Arc::new(Mutex::new(current)),
            switch: CancellationToken::new(),
        }
    }

    /// Switch a running seed step to `seed`: stops its download at the next
    /// chunk and removes the partial file. Later seed steps use it too.
    pub fn use_local(&self, seed: VerifiedLocalSeed) {
        *self.lock() = Some(seed);
        self.switch.cancel();
    }

    /// The chosen local archive, if any.
    pub fn current(&self) -> Option<VerifiedLocalSeed> {
        self.lock().clone()
    }

    /// Fires when [`SeedSourceHandle::use_local`] is called.
    pub(crate) fn switched(&self) -> CancellationToken {
        self.switch.clone()
    }

    fn lock(&self) -> MutexGuard<'_, Option<VerifiedLocalSeed>> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A player's archive whose size and SHA-256 matched the signed seed when it
/// was chosen. Not proof at use: `copy_into` re-hashes what it copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedLocalSeed {
    /// Where the player keeps the archive. Only ever opened for reading.
    pub path: PathBuf,
    /// The size the signed seed entry requires.
    pub size: u64,
    /// The SHA-256 the signed seed entry requires.
    pub sha256: String,
    /// The file's identity when it was checked.
    pub identity: LocalFileIdentity,
}

/// Identifies one file on disk, so a replaced file is noticed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFileIdentity {
    /// Volume serial number (`dev` elsewhere).
    pub volume: u64,
    /// 128-bit file id (`ino` elsewhere).
    pub file_id: u128,
    /// Last write time in nanoseconds (`mtime` elsewhere).
    pub modified_ns: i128,
}

/// Why a local archive cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum LocalSeedError {
    /// The path is a link, a directory or another non-regular file.
    #[error("not a regular file")]
    NotAFile,
    /// The file's size is not the size the signed seed requires.
    #[error("archive size {actual} does not match the signed seed ({expected})")]
    SizeMismatch {
        /// The size the signed seed requires.
        expected: u64,
        /// The size on disk.
        actual: u64,
    },
    /// The file's SHA-256 is not the one the signed seed requires.
    #[error("archive does not match the signed seed")]
    HashMismatch,
    /// The file was replaced after it was checked.
    #[error("archive changed after it was checked")]
    Changed,
    /// The player cancelled, or the seed step was switched.
    #[error("cancelled")]
    Cancelled,
    /// Reading or writing failed.
    #[error("I/O error")]
    Io,
}

impl From<std::io::Error> for LocalSeedError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}

/// Size first (instant refusal), then a streamed SHA-256 with progress and cancel.
/// Opens without following links; reads only.
pub fn verify_local_seed(
    path: &Path,
    seed: &SeedEntry,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<VerifiedLocalSeed, LocalSeedError> {
    let mut file = open_regular(path)?;
    let size = file.metadata()?.len();
    if size != seed.size {
        return Err(LocalSeedError::SizeMismatch {
            expected: seed.size,
            actual: size,
        });
    }
    let identity = identity_of(&file)?;
    let mut hasher = Sha256::new();
    stream_file(&mut file, seed.size, cancel, progress, |chunk| {
        hasher.update(chunk);
        Ok(())
    })?;
    if !hex_digest(hasher).eq_ignore_ascii_case(&seed.sha256) {
        return Err(LocalSeedError::HashMismatch);
    }
    Ok(VerifiedLocalSeed {
        path: path.to_path_buf(),
        size: seed.size,
        sha256: seed.sha256.clone(),
        identity,
    })
}

impl VerifiedLocalSeed {
    /// Opens `path` without following links, checks identity and size are the
    /// verified ones, copies to `<dest>.local-copy` while hashing, fsyncs, and
    /// renames onto `dest` only if the hash matches. Never writes, moves or
    /// deletes `path`.
    pub fn copy_into(
        &self,
        dest: &Path,
        cancel: &CancellationToken,
        progress: &ProgressSink,
    ) -> Result<(), LocalSeedError> {
        let copy = local_copy_path(dest);
        let result = self
            .copy_to(&copy, cancel, progress)
            .and_then(|()| std::fs::rename(&copy, dest).map_err(LocalSeedError::from));
        if result.is_err() {
            let _ = std::fs::remove_file(&copy);
        }
        result
    }

    /// Copies the archive to `copy` and checks the copy's hash. Only `copy`
    /// is ever created or removed here.
    fn copy_to(
        &self,
        copy: &Path,
        cancel: &CancellationToken,
        progress: &ProgressSink,
    ) -> Result<(), LocalSeedError> {
        let mut source = open_regular(&self.path)?;
        if source.metadata()?.len() != self.size || identity_of(&source)? != self.identity {
            return Err(LocalSeedError::Changed);
        }
        let _ = std::fs::remove_file(copy);
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(copy)?;
        let mut hasher = Sha256::new();
        let copied = stream_file(&mut source, self.size, cancel, progress, |chunk| {
            hasher.update(chunk);
            out.write_all(chunk)?;
            Ok(())
        })?;
        if copied != self.size {
            return Err(LocalSeedError::Changed);
        }
        out.sync_all()?;
        if !hex_digest(hasher).eq_ignore_ascii_case(&self.sha256) {
            return Err(LocalSeedError::HashMismatch);
        }
        Ok(())
    }
}

/// `<dest>.local-copy`, the scratch file a local copy is written to first.
fn local_copy_path(dest: &Path) -> PathBuf {
    let mut name = dest.as_os_str().to_owned();
    name.push(".local-copy");
    PathBuf::from(name)
}

/// Opens `path` for reading, refusing links and other non-regular files.
fn open_regular(path: &Path) -> Result<File, LocalSeedError> {
    let link = std::fs::symlink_metadata(path)?;
    if !link.is_file() {
        return Err(LocalSeedError::NotAFile);
    }
    let file = open_no_follow(path)?;
    if opened_is_link(&link, &file.metadata()?) {
        return Err(LocalSeedError::NotAFile);
    }
    Ok(file)
}

/// Opens the file itself: on Windows the open does not follow a reparse point.
#[cfg(windows)]
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

/// Elsewhere the link check in [`open_regular`] compares the opened file
/// with the link's own metadata. No `O_NOFOLLOW`: the crate has no libc.
#[cfg(not(windows))]
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

/// Whether the opened handle is a reparse point (Windows).
#[cfg(windows)]
fn opened_is_link(_link: &Metadata, opened: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    opened.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

/// Whether the opened file is a different object from the link check (Unix).
#[cfg(not(windows))]
fn opened_is_link(link: &Metadata, opened: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    link.dev() != opened.dev() || link.ino() != opened.ino()
}

/// The identity of the open file, from the handle.
#[cfg(windows)]
fn identity_of(file: &File) -> Result<LocalFileIdentity, LocalSeedError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, FileIdInfo, GetFileInformationByHandleEx, FILE_BASIC_INFO, FILE_ID_INFO,
    };
    let handle = file.as_raw_handle();
    let mut id: FILE_ID_INFO = unsafe { std::mem::zeroed() };
    let mut basic: FILE_BASIC_INFO = unsafe { std::mem::zeroed() };
    // SAFETY: `handle` is open for as long as `file` is. Each call writes
    // no more than the size of the struct it is given.
    unsafe {
        let id_ok = GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        );
        if id_ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let basic_ok = GetFileInformationByHandleEx(
            handle,
            FileBasicInfo,
            (&mut basic as *mut FILE_BASIC_INFO).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        );
        if basic_ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(LocalFileIdentity {
        volume: id.VolumeSerialNumber,
        file_id: u128::from_le_bytes(id.FileId.Identifier),
        // FILETIME ticks are 100 ns.
        modified_ns: i128::from(basic.LastWriteTime) * 100,
    })
}

/// The identity of the open file, from the handle (Unix).
#[cfg(not(windows))]
fn identity_of(file: &File) -> Result<LocalFileIdentity, LocalSeedError> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    Ok(LocalFileIdentity {
        volume: meta.dev(),
        file_id: u128::from(meta.ino()),
        modified_ns: i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec()),
    })
}

/// Reads `source` to the end in [`READ_CHUNK`] pieces, passing each piece to
/// `sink`, reporting progress and honouring `cancel` between reads. Returns
/// the number of bytes read.
fn stream_file(
    source: &mut File,
    size: u64,
    cancel: &CancellationToken,
    progress: &ProgressSink,
    mut sink: impl FnMut(&[u8]) -> Result<(), LocalSeedError>,
) -> Result<u64, LocalSeedError> {
    let mut buf = vec![0u8; READ_CHUNK];
    let mut done = 0u64;
    loop {
        if cancel.is_cancelled() {
            return Err(LocalSeedError::Cancelled);
        }
        let n = match source.read(&mut buf) {
            Ok(0) => return Ok(done),
            Ok(n) => n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        sink(&buf[..n])?;
        done += n as u64;
        progress.report(Progress::Downloading {
            label: CHECK_LABEL.to_string(),
            downloaded: done,
            total: size,
        });
    }
}

/// Lower-case hex of a finished SHA-256.
fn hex_digest(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "seed_source_tests.rs"]
mod tests;
