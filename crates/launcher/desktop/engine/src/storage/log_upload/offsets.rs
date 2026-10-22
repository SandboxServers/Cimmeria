//! How far into each launcher log file the last sent chunk reached
//! (launcher-logs LF-13). The file is `uploaded-log-offsets.json` beside
//! `uploaded-logs.json`, so a repeat press sends only the lines written since.
//! A file that does not parse loads as empty: the next press sends from the
//! start of each file, which is a repeat of lines, never a loss.
use super::lines::LogLine;
use super::LogUploadError;
use crate::storage::StorageError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The offsets file, in the launcher state directory.
const FILE: &str = "uploaded-log-offsets.json";

/// The format this build writes.
const VERSION: u32 = 1;

/// The most bytes the offsets file may take. One entry per launcher log file
/// is a few dozen bytes, so this is far above what a normal install writes.
const MAX_BYTES: usize = 64 * 1024;

/// The byte offset already sent, per launcher log file name.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Offsets {
    pub version: u32,
    /// File name -> byte offset already sent.
    pub files: BTreeMap<String, u64>,
}

/// The file as this build writes it: the version is always `VERSION`.
#[derive(Serialize)]
struct Stored<'a> {
    version: u32,
    files: &'a BTreeMap<String, u64>,
}

impl Offsets {
    /// The offsets in `dir`, or none when the file is missing or does not parse.
    pub(crate) fn load(dir: &Path) -> Self {
        std::fs::read(dir.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_else(|| Self {
                version: VERSION,
                files: BTreeMap::new(),
            })
    }

    /// Writes the offsets into `dir` atomically (temp file, then rename).
    pub(crate) fn save(&self, dir: &Path) -> Result<(), LogUploadError> {
        let stored = Stored {
            version: VERSION,
            files: &self.files,
        };
        let bytes = serde_json::to_vec_pretty(&stored).map_err(|_| StorageError::Corrupt)?;
        std::fs::create_dir_all(dir).map_err(|_| StorageError::Io)?;
        crate::storage::atomic::write_bytes(dir, FILE, &bytes, MAX_BYTES)?;
        Ok(())
    }

    /// Moves `line`'s file forward to the line's end. Never moves it back, so
    /// a late or repeated `advance` cannot resend lines.
    pub(crate) fn advance(&mut self, line: &LogLine) {
        let sent = self.files.entry(line.file.clone()).or_default();
        *sent = (*sent).max(line.end);
    }
}
