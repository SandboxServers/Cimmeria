//! How far into each launcher log file the last sent chunk reached
//! (launcher-logs LF-13). The file is `uploaded-log-offsets.json` in the launcher
//! state directory, so a repeat press sends only the lines written since.
//! A file that does not parse loads as empty: the next press sends from the
//! start of each file, which is a repeat of lines, never a loss.
use super::game_lines::{GameFile, KEY_PREFIX};
use super::lines::LogLine;
use super::LogUploadError;
use crate::storage::StorageError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::SystemTime;

/// The offsets file, in the launcher state directory.
const FILE: &str = "uploaded-log-offsets.json";

/// The format this build writes. Version 2 (LF-19) adds `heads` and `tails`;
/// a version-1 file has neither and loads with them empty.
const VERSION: u32 = 2;

/// The most bytes the offsets file may take. Launcher logs are a handful of
/// entries; each game file adds its key three times (offset, head, tail), about
/// 500 bytes at a 150-byte key, so `MAX_FILES` such entries fit (LF-19 A7).
pub(super) const MAX_BYTES: usize = 1024 * 1024;

/// The most files the offsets keep; the shell keeps 7 launcher logs, and the
/// game keeps one folder per session, so this leaves room for thousands. Past
/// this a live file is re-sent in full on each press (LF-19 A7).
pub(super) const MAX_FILES: usize = 2048;

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
    /// Game file key -> the fingerprint of the bytes that end at its saved
    /// offset (A6). A changed tail means the file was rewritten, so it restarts
    /// at 0. Missing (a version-1 file) means no tail check, as before.
    #[serde(default)]
    pub tails: BTreeMap<String, String>,
}

/// The file as this build writes it: the version is always `VERSION`.
#[derive(Serialize)]
struct Stored<'a> {
    version: u32,
    files: &'a BTreeMap<String, u64>,
    heads: &'a BTreeMap<String, String>,
    tails: &'a BTreeMap<String, String>,
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
            tails: &self.tails,
        };
        // Compact JSON: the file is bounded by MAX_BYTES, and pretty-printing
        // would spend that budget on indentation (A7).
        let bytes = serde_json::to_vec(&stored).map_err(|_| StorageError::Corrupt)?;
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

    /// `advance` for a game line. `read` is what the reader saw of the line's
    /// file: when it says the file restarted (its head or tail changed, or it got
    /// shorter than the saved offset), the old offset is dropped first, because
    /// `advance` never moves back and would otherwise skip the new file's lines.
    /// The head and the tail at the line's end are saved with the offset.
    pub(crate) fn advance_game(&mut self, line: &LogLine, read: &HashMap<String, GameFile>) {
        if let Some(file) = read.get(&line.file) {
            if file.restarted {
                self.files.insert(line.file.clone(), 0);
            }
            self.heads.insert(line.file.clone(), file.head.clone());
        }
        self.advance(line);
        // Only the furthest line sets the tail: `advance` kept the larger end.
        if self.files.get(&line.file) == Some(&line.end) {
            if let Some(tail) = read.get(&line.file).and_then(|f| f.tails.get(&line.end)) {
                self.tails.insert(line.file.clone(), tail.clone());
            }
        }
    }

    /// Forgets the offsets of game files that are gone, then, past `MAX_FILES`,
    /// the oldest game files by modification time (A7). `live` holds the game
    /// files read this press with their modification times; `busy` holds the
    /// files with lines in this press, which are never evicted. Launcher keys
    /// are not game files and are kept. Runs after the read, never on `advance`,
    /// so the file being advanced is not dropped by its own write.
    pub(crate) fn prune(&mut self, live: &HashMap<String, SystemTime>, busy: &HashSet<String>) {
        let gone: Vec<String> = self
            .files
            .keys()
            .filter(|key| key.starts_with(KEY_PREFIX) && !live.contains_key(*key))
            .cloned()
            .collect();
        for key in &gone {
            self.forget(key);
        }
        while self.files.len() > MAX_FILES {
            let oldest = self
                .files
                .keys()
                .filter(|key| !busy.contains(*key))
                .filter_map(|key| live.get(key).map(|modified| (*modified, key.clone())))
                .min();
            // Launcher keys have no game listing, so they go in name order, which
            // is date order for `launcher.<date>.log`.
            let key = match oldest {
                Some((_, key)) => key,
                None => match self.files.keys().find(|key| !busy.contains(*key)) {
                    Some(key) => key.clone(),
                    None => break,
                },
            };
            self.forget(&key);
        }
    }

    fn forget(&mut self, key: &str) {
        self.files.remove(key);
        self.heads.remove(key);
        self.tails.remove(key);
    }
}
