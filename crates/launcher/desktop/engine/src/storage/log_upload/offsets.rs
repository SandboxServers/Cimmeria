//! How far into each launcher log file the last sent chunk reached
//! (launcher-logs LF-13). The file is `uploaded-log-offsets.json` beside
//! `uploaded-logs.json`, so a repeat press sends only the lines written since.
//! A file that does not parse loads as empty: the next press sends from the
//! start of each file, which is a repeat of lines, never a loss.
use super::game_lines::GameFile;
use super::lines::LogLine;
use super::LogUploadError;
use crate::storage::StorageError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// The offsets file, in the launcher state directory.
const FILE: &str = "uploaded-log-offsets.json";

/// The format this build writes. Version 2 (LF-19) adds `heads`; a version-1
/// file has none and loads with it empty.
const VERSION: u32 = 2;

/// The most bytes the offsets file may take. Launcher logs are a handful of
/// entries; the game's session logs add one entry (path, offset, head) each.
const MAX_BYTES: usize = 256 * 1024;

/// The most files the offsets keep; the shell keeps 7 launcher logs, and the
/// game keeps one folder per session, so this leaves room for hundreds.
const MAX_FILES: usize = 512;

/// The byte offset already sent, per launcher log file name (`launcher.*.log`)
/// and per game log (`game/<path under the binaries folder>`).
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Offsets {
    pub version: u32,
    /// File name -> byte offset already sent.
    pub files: BTreeMap<String, u64>,
    /// Game file key -> the fingerprint of its first bytes when it was last
    /// sent (see `game_lines::head_of`). The client rewrites `sgwdebuglog` at
    /// its next launch; a changed head means a new file, so it restarts at 0.
    #[serde(default)]
    pub heads: BTreeMap<String, String>,
}

/// The file as this build writes it: the version is always `VERSION`.
#[derive(Serialize)]
struct Stored<'a> {
    version: u32,
    files: &'a BTreeMap<String, u64>,
    heads: &'a BTreeMap<String, String>,
}

impl Offsets {
    /// The offsets in `dir`, or none when the file is missing or does not parse.
    pub(crate) fn load(dir: &Path) -> Self {
        std::fs::read(dir.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_else(|| Self {
                version: VERSION,
                ..Self::default()
            })
    }

    /// Writes the offsets into `dir` atomically (temp file, then rename).
    pub(crate) fn save(&self, dir: &Path) -> Result<(), LogUploadError> {
        let stored = Stored {
            version: VERSION,
            files: &self.files,
            heads: &self.heads,
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
        self.trim();
    }

    /// `advance` for a game line. `read` is what the reader saw of the line's
    /// file: when it says the file restarted (its head changed, or it got
    /// shorter than the saved offset), the old offset is dropped first, because
    /// `advance` never moves back and would otherwise skip the new file's lines.
    pub(crate) fn advance_game(&mut self, line: &LogLine, read: &HashMap<String, GameFile>) {
        if let Some(file) = read.get(&line.file) {
            if file.restarted {
                self.files.insert(line.file.clone(), 0);
            }
            self.heads.insert(line.file.clone(), file.head.clone());
        }
        self.advance(line);
    }

    fn trim(&mut self) {
        while self.files.len() > MAX_FILES {
            if let Some((key, _)) = self.files.pop_first() {
                self.heads.remove(&key);
            }
        }
    }
}
