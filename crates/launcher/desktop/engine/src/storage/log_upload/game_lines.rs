//! The game's own log lines for the debug-log upload (launcher-logs LF-19).
//! `collect` reads `sgwdebuglog*` in the binaries folder and every
//! `sessions/**/*.log` below it (the file discovery launcher-core already uses
//! for the zip, so key dumps and `current-session.json` stay out), from each
//! file's saved byte offset to its last complete line.
//!
//! D-LU3: no content filter. Chat and character names go through as written;
//! only the per-word scrub and the IP scrub every message gets apply. Nothing
//! is sent here.
//!
//! Wire quirks this file owns:
//! - The QA client writes `SGWDebugLog.log` as UTF-8 (checked on a real file,
//!   see the LF-19 report), but BigWorld's unicode logs have been UTF-16LE, so
//!   both are read. Offsets are always bytes of the file, never characters.
//! - The client rewrites `SGWDebugLog.log` at its next launch. A new file can
//!   be longer than the saved offset, so length alone does not catch it: the
//!   file's first bytes are fingerprinted (`head_of`) and a change restarts it.
use super::lines::{Collected, LogLine, MAX_LINES};
use super::offsets::Offsets;
use super::{events, LogUploadError};
use crate::storage::game_telemetry::ip_scrub::scrub_ips;
use crate::storage::game_telemetry::launcher_event::Redaction;
use cimmeria_launcher_core::install_layout::binaries_dir;
use cimmeria_launcher_core::logs::collect_log_inputs;
use cimmeria_launcher_core::telemetry::events::TelemetryEvent;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// The offsets key prefix of a game file: `game/<path under binaries, '/'>`.
const KEY_PREFIX: &str = "game/";

/// How many leading bytes the head fingerprint covers.
const HEAD_BYTES: usize = 256;

/// The longest message kept, in bytes; a longer one is cut on a char boundary.
const MAX_MESSAGE_BYTES: usize = 2_048;

/// The largest file read whole. A bigger one is left alone rather than loaded
/// into memory; no real client log comes near this.
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

/// What the reader saw of one file, for `Offsets::advance_game`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameFile {
    /// The fingerprint to save once a line of this file is sent.
    pub head: String,
    /// True when the file is a new one (head changed) or shorter than its saved
    /// offset: the saved offset must be dropped, not kept by `advance`'s `max`.
    pub restarted: bool,
}

/// The game lines `collect` read, plus what it saw of each file they came from.
#[derive(Debug, Default)]
pub(crate) struct GameCollected {
    pub collected: Collected,
    pub files: HashMap<String, GameFile>,
    /// Whether the game has any debug or session log file at all.
    pub has_files: bool,
}

/// The new complete lines in every game log under `game` (the game folder, the
/// one that holds `Working/Binaries`). A missing folder is no lines. A file
/// that cannot be read is skipped with its offset untouched. A cancel, checked
/// before each file, returns `CollectFailed`, as `lines::collect` does.
pub(crate) fn collect(
    game: &Path,
    offsets: &Offsets,
    cancel: &CancellationToken,
) -> Result<GameCollected, LogUploadError> {
    let cancelled = || {
        if cancel.is_cancelled() {
            Err(LogUploadError::CollectFailed)
        } else {
            Ok(())
        }
    };
    cancelled()?;
    let inputs = inputs(game)?;
    let redaction = Redaction::from_env();
    let mut out = GameCollected {
        has_files: !inputs.is_empty(),
        ..GameCollected::default()
    };
    for (key, path) in inputs {
        cancelled()?;
        let Ok(bytes) = read_whole(&path) else {
            continue;
        };
        let saved = offsets.files.get(&key).copied().unwrap_or(0);
        let saved_head = offsets.heads.get(&key).map(String::as_str);
        let (lines, file, total) = read_file(&key, &bytes, saved, saved_head, &redaction);
        out.collected.lines_total += total;
        out.collected.lines.extend(lines);
        out.files.insert(key, file);
        // Keep the memory bound at one file's lines plus the newest MAX_LINES.
        let excess = out.collected.lines.len().saturating_sub(MAX_LINES);
        out.collected.lines.drain(..excess);
    }
    Ok(out)
}

/// The same lines with `file` replaced by the bare file name, which is what the
/// row's `file` field wants (C-LF5: a name, never a path). The offsets key stays
/// on the originals.
pub(crate) fn row_lines(lines: &[LogLine]) -> Vec<LogLine> {
    lines
        .iter()
        .map(|line| LogLine {
            file: line.file.rsplit('/').next().unwrap_or("").to_string(),
            ..line.clone()
        })
        .collect()
}

/// Shifts the rows' `seq` up by `by`, so game rows continue the launcher rows'
/// numbering instead of repeating it inside one upload.
pub(crate) fn continue_seq(rows: &mut [TelemetryEvent], by: u64) {
    for row in rows {
        if let TelemetryEvent::ClientNative(event) = row {
            event.seq += by;
        }
    }
}

/// `(offsets key, path)` per log file, oldest modification first so the
/// "newest MAX_LINES" cut keeps the right ones. Symlinks are skipped: a link
/// named `sgwdebuglog` must not make the launcher read some other file.
fn inputs(game: &Path) -> Result<Vec<(String, PathBuf)>, LogUploadError> {
    let Ok(files) = collect_log_inputs(game) else {
        events::collect_failed("game_files");
        return Err(LogUploadError::CollectFailed);
    };
    let binaries = binaries_dir(game);
    let mut found: Vec<(std::time::SystemTime, String, PathBuf)> = files
        .into_iter()
        .filter_map(|path| {
            let meta = std::fs::symlink_metadata(&path).ok()?;
            if !meta.file_type().is_file() {
                return None;
            }
            let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            let key = key_of(&binaries, &path)?;
            Some((modified, key, path))
        })
        .collect();
    found.sort();
    Ok(found
        .into_iter()
        .map(|(_, key, path)| (key, path))
        .collect())
}

/// `game/<path under binaries with '/'>`, or none for a path outside it or a
/// name that is not valid Unicode.
fn key_of(binaries: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(binaries).ok()?;
    let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
    Some(format!("{KEY_PREFIX}{}", parts?.join("/")))
}

fn read_whole(path: &Path) -> std::io::Result<Vec<u8>> {
    if std::fs::metadata(path)?.len() > MAX_FILE_BYTES {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    std::fs::read(path)
}

/// The fingerprint of a file's first bytes: `<n>:<h12>`, `n` the number of
/// bytes hashed (at most 256) and `h12` the first 12 hex digits of their
/// SHA-256. The length is part of it so a short file that is still being
/// written does not look like a rewrite when it grows.
pub(crate) fn head_of(bytes: &[u8]) -> String {
    let n = bytes.len().min(HEAD_BYTES);
    format!("{n}:{}", h12(&bytes[..n]))
}

fn h12(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .take(6)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// True when `bytes` still starts with the bytes `saved` fingerprinted.
fn head_matches(saved: &str, bytes: &[u8]) -> bool {
    let Some((n, hash)) = saved.split_once(':') else {
        return false;
    };
    let Ok(n) = n.parse::<usize>() else {
        return false;
    };
    n <= bytes.len() && h12(&bytes[..n]) == hash
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Utf16Le,
}

/// The file's encoding and how many bytes of byte-order mark to skip. UTF-16LE
/// when it starts `FF FE`, or when more than 30% of the odd-offset bytes in the
/// first 512 are NUL (ASCII text in UTF-16LE has a NUL high byte). Otherwise
/// UTF-8, whose BOM is skipped.
fn detect(bytes: &[u8]) -> (Encoding, usize) {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return (Encoding::Utf16Le, 2);
    }
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (Encoding::Utf8, 3);
    }
    let window = &bytes[..bytes.len().min(512)];
    let odd = window.len() / 2;
    let nul = window
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|&&b| b == 0)
        .count();
    if odd > 0 && nul * 100 > odd * 30 {
        (Encoding::Utf16Le, 0)
    } else {
        (Encoding::Utf8, 0)
    }
}

/// The byte offset just past each `\n` at or after `from`. A half-written last
/// line has no entry, so it waits for its newline.
fn line_ends(bytes: &[u8], from: usize, encoding: Encoding) -> Vec<usize> {
    match encoding {
        Encoding::Utf8 => bytes[from..]
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b == b'\n')
            .map(|(i, _)| from + i + 1)
            .collect(),
        Encoding::Utf16Le => bytes[from..]
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, unit)| **unit == [b'\n', 0])
            .map(|(i, _)| from + 2 * i + 2)
            .collect(),
    }
}

fn decode(bytes: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        Encoding::Utf16Le => char::decode_utf16(
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|unit| u16::from_le_bytes(*unit)),
        )
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect(),
    }
}

/// The complete non-empty lines of one file from the saved offset, newest
/// `MAX_LINES` only, and what the reader saw of the file.
fn read_file(
    key: &str,
    bytes: &[u8],
    saved: u64,
    saved_head: Option<&str>,
    redaction: &Redaction,
) -> (Vec<LogLine>, GameFile, u64) {
    let (encoding, bom) = detect(bytes);
    let restarted =
        saved > bytes.len() as u64 || saved_head.is_some_and(|h| !head_matches(h, bytes));
    let file = GameFile {
        head: head_of(bytes),
        restarted,
    };
    let mut start = if restarted { 0 } else { saved as usize };
    if encoding == Encoding::Utf16Le {
        start &= !1;
    }
    let start = start.max(bom);
    let ends = line_ends(bytes, bom, encoding);
    let before = ends.partition_point(|&end| end <= start);
    let newline = if encoding == Encoding::Utf16Le { 2 } else { 1 };
    let mut kept: Vec<(u64, usize, String)> = Vec::new();
    for index in before..ends.len() {
        let begin = if index == 0 { bom } else { ends[index - 1] };
        let end = ends[index];
        let text = decode(&bytes[begin..end - newline], encoding);
        let text = text.trim_end_matches('\r');
        if text.trim().is_empty() {
            continue;
        }
        kept.push((index as u64 + 1, end, text.to_string()));
    }
    let total = kept.len() as u64;
    let skip = kept.len().saturating_sub(MAX_LINES);
    let lines: Vec<LogLine> = kept
        .into_iter()
        .skip(skip)
        .map(|(line_no, end, text)| {
            let scrubbed = scrub_ips(&redaction.scrub(&text)).into_owned();
            let (message, truncated) = cut(scrubbed);
            LogLine {
                file: key.to_string(),
                line_no,
                end: end as u64,
                // The sample proves no timestamp format worth trusting across
                // time zones, so the row takes the read time (C-LF5).
                ts_ms: None,
                level: None,
                target: None,
                message,
                truncated,
            }
        })
        .collect();
    (lines, file, total)
}

/// `message`, cut to `MAX_MESSAGE_BYTES` on a char boundary, and whether it was.
fn cut(message: String) -> (String, bool) {
    if message.len() <= MAX_MESSAGE_BYTES {
        return (message, false);
    }
    let mut end = MAX_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    (message[..end].to_string(), true)
}

#[cfg(test)]
#[path = "game_lines_tests.rs"]
mod tests;
