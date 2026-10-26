//! Plans, stages and commits the composed cooked-data PAKs in the client's
//! cache (D-LP22). Stage writes each PAK beside its destination under a temp
//! name; commit renames the temps over the PAKs. So no PAK is ever partly
//! written, and no failure before commit changes the cache.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, ErrorKind};
use std::path::{Path, PathBuf};

use cimmeria_cooked_pak::catalog::{CategoryBody, IndexEntry};
use cimmeria_cooked_pak::compose::{compose, ComposeError};
use cimmeria_cooked_pak::decide::{decide, Action};
use cimmeria_cooked_pak::pak::{read_pak, read_version, write_pak, PakError};

use super::io_detail::{io_detail_of, IoDetail};

/// The suffix of a staged PAK's temp file, beside its destination.
pub const TEMP_SUFFIX: &str = ".cimmeria-presync.tmp";

/// The client's cooked-data cache folder: `firesky/SGWGame/Cache.en-US`.
pub fn cache_dir(firesky: &Path) -> PathBuf {
    firesky.join("SGWGame").join("Cache.en-US")
}

/// Refuses a `firesky` whose `firesky`, `SGWGame` or `Cache.en-US` folder is a
/// symlink or junction, so a write can't leave the game folder. A missing
/// folder is fine; one that can't be inspected is refused.
pub fn ensure_not_linked(firesky: &Path) -> Result<(), ApplyError> {
    let sgw = firesky.join("SGWGame");
    let linked = [firesky.to_path_buf(), sgw.clone(), cache_dir(firesky)]
        .iter()
        .any(|path| is_link(path));
    if linked {
        return Err(ApplyError::UnsafePath);
    }
    Ok(())
}

/// Whether a path is a symlink or a Windows junction (a reparse point). A
/// missing path is not a link; any other stat failure counts as one.
fn is_link(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(meta) => meta.file_type().is_symlink() || is_reparse_point(&meta),
        Err(error) => error.kind() != ErrorKind::NotFound,
    }
}

#[cfg(windows)]
fn is_reparse_point(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    // FILE_ATTRIBUTE_REPARSE_POINT: a junction, or another reparse point.
    meta.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_meta: &fs::Metadata) -> bool {
    false
}

/// What the presync does with one category, with the versions it decided on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// Write the category, leave it, or refuse because the source is newer.
    pub action: Action,
    /// The cache PAK's version; None when it is missing or unreadable.
    pub cache_version: Option<u32>,
    /// The source PAK's version; None when it is missing or unreadable.
    pub source_version: Option<u32>,
}

/// Reads both versions of one category's PAK and decides what to do. An
/// unreadable cache PAK counts as None, so it is rewritten.
pub fn plan(entry: &IndexEntry, cache_dir: &Path, source_dir: &Path) -> Plan {
    let cache_version = read_version(&cache_dir.join(&entry.pak)).ok().flatten();
    let source_version = read_version(&source_dir.join(&entry.pak)).ok().flatten();
    Plan {
        action: decide(cache_version, source_version, entry.version),
        cache_version,
        source_version,
    }
}

/// A PAK composed and written to its temp file, not yet renamed over its
/// destination. Dropping it deletes the temp, unless [`commit`] renamed it.
#[derive(Debug)]
pub struct Staged {
    /// The category the PAK belongs to.
    pub category_id: u32,
    /// The PAK's file name in the cache.
    pub pak: String,
    /// The size of the written temp, in bytes.
    pub bytes: u64,
    temp: PathBuf,
    committed: bool,
}

impl Drop for Staged {
    fn drop(&mut self) {
        if !self.committed {
            // Best effort: a temp that is already gone is the goal anyway.
            let _ = fs::remove_file(&self.temp);
        }
    }
}

/// Composes one category from the source PAK and the served body, checks the
/// digest, and writes the result to its temp file, synced to disk. On any
/// failure nothing is left behind and the cache is unchanged.
pub fn stage(
    category_id: u32,
    body: &CategoryBody,
    pak: &str,
    cache_dir: &Path,
    source_dir: &Path,
) -> Result<Staged, ApplyError> {
    if !is_plain_name(pak) {
        return Err(ApplyError::UnsafePath);
    }
    let source = read_pak(&source_dir.join(pak)).map_err(source_error)?;
    let entries = compose(body, &source.entries).map_err(|error| match error {
        ComposeError::BaseMissing { missing, first } => ApplyError::BaseMissing { missing, first },
        ComposeError::DigestMismatch => ApplyError::DigestMismatch,
    })?;
    let temp = cache_dir.join(format!("{pak}{TEMP_SUFFIX}"));
    match write_temp(cache_dir, &temp, body.version, &entries) {
        Ok(bytes) => Ok(Staged {
            category_id,
            pak: pak.to_owned(),
            bytes,
            temp,
            committed: false,
        }),
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(ApplyError::WriteFailed(io_detail_of(&error)))
        }
    }
}

/// Renames every staged temp over its destination, in ascending category
/// order, and returns how many were renamed. First every existing destination
/// is opened exclusively; if any is in use, nothing is renamed. A rename that
/// fails after that returns the count renamed so far, and the rest of the
/// `Staged` values delete their temps.
pub fn commit(mut staged: Vec<Staged>, cache_dir: &Path) -> Result<u32, CommitError> {
    staged.sort_by_key(|item| item.category_id);
    for item in &staged {
        if let Err(detail) = check_not_in_use(&cache_dir.join(&item.pak)) {
            return Err(CommitError {
                committed: 0,
                category_id: item.category_id,
                pak: item.pak.clone(),
                error: in_use_or_write(detail),
            });
        }
    }
    let mut committed = 0;
    for item in staged.iter_mut() {
        let dest = cache_dir.join(&item.pak);
        match fs::rename(&item.temp, &dest) {
            Ok(()) => {
                item.committed = true;
                committed += 1;
            }
            Err(error) => {
                return Err(CommitError {
                    committed,
                    category_id: item.category_id,
                    pak: item.pak.clone(),
                    error: in_use_or_write(io_detail_of(&error)),
                });
            }
        }
    }
    Ok(committed)
}

/// Deletes the stray `*.cimmeria-presync.tmp` files directly in `cache_dir`.
/// Nothing else is touched: no other name, no subfolder. Best effort; returns
/// how many files it removed.
pub fn sweep_temps(cache_dir: &Path) -> u32 {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let is_temp = entry.file_name().to_string_lossy().ends_with(TEMP_SUFFIX);
        let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
        if is_temp && is_file && fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// A commit that failed. `committed` counts the renames made before it, so
/// those PAKs are whole and new; the others are whole and still old.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitError {
    /// How many PAKs were renamed before the failure.
    pub committed: u32,
    /// The category whose rename or pre-check failed.
    pub category_id: u32,
    /// That category's PAK file name.
    pub pak: String,
    /// Why it failed.
    pub error: ApplyError,
}

/// Why a presync step failed. Each variant has a stable `reason` code.
#[derive(Debug, Clone, PartialEq)]
pub enum ApplyError {
    /// The source folder has no such PAK.
    SourceMissing,
    /// The source PAK exists but could not be read.
    SourceUnreadable(IoDetail),
    /// The served body needs base records the source PAK lacks.
    BaseMissing {
        /// How many keys are missing.
        missing: u32,
        /// The first missing key.
        first: u32,
    },
    /// The composed records do not match the served digest.
    DigestMismatch,
    /// The cache PAK is open in another process (a sharing or lock violation).
    PakInUse(IoDetail),
    /// Writing or renaming a PAK failed for another reason.
    WriteFailed(IoDetail),
    /// The PAK name has a path separator, a colon or `..`.
    UnsafePath,
}

impl ApplyError {
    /// The snake_case reason the report carries for this error.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::SourceMissing => "source_missing",
            Self::SourceUnreadable(_) => "source_unreadable",
            Self::BaseMissing { .. } => "base_missing",
            Self::DigestMismatch => "digest_mismatch",
            Self::PakInUse(_) => "pak_in_use",
            Self::WriteFailed(_) => "write_failed",
            Self::UnsafePath => "unsafe_path",
        }
    }

    /// The I/O detail of the error, when it has one.
    pub fn io(&self) -> Option<IoDetail> {
        match self {
            Self::SourceUnreadable(detail) | Self::PakInUse(detail) | Self::WriteFailed(detail) => {
                Some(*detail)
            }
            _ => None,
        }
    }
}

/// A PAK file name that stays inside its folder.
fn is_plain_name(pak: &str) -> bool {
    !pak.is_empty() && !pak.contains(['/', '\\', ':']) && !pak.contains("..")
}

/// Maps a source read failure: a missing file is `SourceMissing`, anything
/// else is `SourceUnreadable` with its I/O code.
fn source_error(error: PakError) -> ApplyError {
    match error {
        PakError::Io(io) if io.kind() == ErrorKind::NotFound => ApplyError::SourceMissing,
        other => ApplyError::SourceUnreadable(pak_detail(&other)),
    }
}

/// Names a PAK read failure: an I/O cause by its code, a malformed ZIP as
/// `other`.
fn pak_detail(error: &PakError) -> IoDetail {
    match error {
        PakError::Io(io) => io_detail_of(io),
        PakError::Zip(_) => IoDetail {
            kind: "other",
            os_code: None,
        },
    }
}

/// Writes the composed entries to `temp` at `version`, then syncs the file.
/// Returns the file's size.
fn write_temp(
    cache_dir: &Path,
    temp: &Path,
    version: u32,
    entries: &BTreeMap<u32, Vec<u8>>,
) -> std::io::Result<u64> {
    fs::create_dir_all(cache_dir)?;
    let mut out = BufWriter::new(File::create(temp)?);
    write_pak(&mut out, version, entries).map_err(|error| match error {
        PakError::Io(io) => io,
        PakError::Zip(zip) => std::io::Error::other(zip),
    })?;
    let file = out.into_inner().map_err(|error| error.into_error())?;
    file.sync_all()?;
    Ok(file.metadata()?.len())
}

/// Opens an existing destination with no sharing, so it fails while another
/// process has it open. A destination that does not exist is fine.
fn check_not_in_use(dest: &Path) -> Result<(), IoDetail> {
    #[cfg(windows)]
    let opened = {
        use std::os::windows::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(dest)
    };
    #[cfg(not(windows))]
    let opened = OpenOptions::new().read(true).write(true).open(dest);
    match opened {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_detail_of(&error)),
    }
}

/// Classes a commit failure: a file held by another process or refused by
/// the OS's permissions is `PakInUse`, anything else `WriteFailed`.
fn in_use_or_write(detail: IoDetail) -> ApplyError {
    match detail.kind {
        "sharing_violation" | "lock_violation" | "permission_denied" => {
            ApplyError::PakInUse(detail)
        }
        _ => ApplyError::WriteFailed(detail),
    }
}

#[cfg(test)]
#[path = "tests/apply.rs"]
mod tests;
