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
use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio_util::sync::CancellationToken;

/// The offsets key prefix of a game file: `game/<path under binaries, '/'>`.
pub(crate) const KEY_PREFIX: &str = "game/";

/// How many leading bytes the head fingerprint covers.
const HEAD_BYTES: usize = 256;

/// How many bytes before a saved offset the tail fingerprint covers (A6).
const TAIL_BYTES: usize = 64;

/// The longest message kept, in bytes; a longer one is cut on a char boundary.
const MAX_MESSAGE_BYTES: usize = 2_048;

/// The longest line the scrub sees, in bytes. Text past it is cut first, so a
/// huge line costs bounded CPU (A3). Never below `MAX_MESSAGE_BYTES`.
const MAX_RAW_BYTES: usize = 32 * 1024;

/// The bytes read per block while a file is streamed (A1). Memory per file is
/// this block plus the newest `MAX_LINES` raw lines, never the whole file.
const BLOCK_BYTES: usize = 1024 * 1024;

/// What the reader saw of one file, for `Offsets::advance_game`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameFile {
    /// The fingerprint to save once a line of this file is sent.
    pub head: String,
    /// True when the file is a new one (head changed) or shorter than its saved
    /// offset: the saved offset must be dropped, not kept by `advance`'s `max`.
    pub restarted: bool,
    /// The fingerprint of the bytes that end at each kept line's end, so the
    /// offset saved for that line can carry its tail (A6).
    pub tails: HashMap<u64, String>,
}

/// The game lines `collect` read, plus what it saw of each file they came from.
/// Its `collected` lines hold raw chat text: never format this with `{:?}` in
/// an event, a log line or a panic (A10, see `LogLine`).
#[derive(Debug, Default)]
pub(crate) struct GameCollected {
    pub collected: Collected,
    pub files: HashMap<String, GameFile>,
    /// Whether the game has any debug or session log file at all.
    pub has_files: bool,
    /// Every game file listed this press, with its modification time (A7).
    pub live: HashMap<String, SystemTime>,
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
    for (key, path, modified) in inputs {
        cancelled()?;
        out.live.insert(key.clone(), modified);
        let saved = offsets.files.get(&key).copied().unwrap_or(0);
        let saved_head = offsets.heads.get(&key).map(String::as_str);
        let saved_tail = offsets.tails.get(&key).map(String::as_str);
        let read = read_file(&key, &path, saved, (saved_head, saved_tail), &redaction);
        let (lines, file, total) = match read {
            Ok(read) => read,
            Err(reason) => {
                // The file is skipped with its offset untouched: its lines wait
                // for the next press. Telemetry carries no path (R5).
                events::game_file_skipped(reason);
                events::collect_failed("game_file_unreadable");
                continue;
            }
        };
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

/// `(offsets key, path, modified)` per log file, oldest modification first so the
/// "newest MAX_LINES" cut keeps the right ones. Symlinks are skipped: a link
/// named `sgwdebuglog` must not make the launcher read some other file.
fn inputs(game: &Path) -> Result<Vec<(String, PathBuf, SystemTime)>, LogUploadError> {
    let Ok(files) = collect_log_inputs(game) else {
        events::collect_failed("game_files");
        return Ok(Vec::new());
    };
    let binaries = binaries_dir(game);
    // The binaries folder resolved once. A directory on the way in (a junction
    // named `sessions`, say) must not lead out of it, so each candidate is
    // resolved and must still sit under this folder (A8).
    let Ok(root) = binaries.canonicalize() else {
        return Ok(Vec::new());
    };
    let mut found: Vec<(SystemTime, String, PathBuf)> = files
        .into_iter()
        .filter_map(|path| {
            let meta = std::fs::symlink_metadata(&path).ok()?;
            if !meta.file_type().is_file() {
                return None;
            }
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let key = key_of(&binaries, &path)?;
            let resolved = path.canonicalize().ok()?;
            if !resolved.starts_with(&root) {
                return None;
            }
            Some((modified, key, resolved))
        })
        .collect();
    found.sort();
    Ok(found
        .into_iter()
        .map(|(modified, key, path)| (key, path, modified))
        .collect())
}

/// `game/<path under binaries with '/'>`, or none for a path outside it or a
/// name that is not valid Unicode.
fn key_of(binaries: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(binaries).ok()?;
    let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
    Some(format!("{KEY_PREFIX}{}", parts?.join("/")))
}

/// The fingerprint of a file's first bytes: `<n>:<h12>`, `n` the number of
/// bytes hashed (at most 256) and `h12` the first 12 hex digits of their
/// SHA-256. The length is part of it so a short file that is still being
/// written does not look like a rewrite when it grows.
pub(crate) fn head_of(bytes: &[u8]) -> String {
    fingerprint(&bytes[..bytes.len().min(HEAD_BYTES)])
}

/// The fingerprint of the bytes that end at a saved offset (`bytes` is the file
/// up to that offset): the last 64 of them, in the same `<n>:<h12>` form (A6).
fn tail_of(bytes: &[u8]) -> String {
    fingerprint(&bytes[bytes.len().saturating_sub(TAIL_BYTES)..])
}

/// `<n>:<h12>` for `bytes`: `n` the number of bytes hashed and `h12` the first
/// 12 hex digits of their SHA-256.
fn fingerprint(bytes: &[u8]) -> String {
    format!("{}:{}", bytes.len(), h12(bytes))
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

/// True when the file still holds the bytes `saved` fingerprinted just before
/// the offset `at`. The reader seeks, so the caller seeks back to where it was.
/// A fingerprint longer than `TAIL_BYTES`, or one that reaches before the file,
/// does not match.
fn tail_matches(reader: &mut std::fs::File, at: u64, saved: &str) -> std::io::Result<bool> {
    let n = saved
        .split_once(':')
        .and_then(|(n, _)| n.parse::<usize>().ok())
        .filter(|&n| n <= TAIL_BYTES && n as u64 <= at);
    let Some(n) = n else {
        return Ok(false);
    };
    let mut bytes = vec![0u8; n];
    reader.seek(SeekFrom::Start(at - n as u64))?;
    reader.read_exact(&mut bytes)?;
    Ok(fingerprint(&bytes) == saved)
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

/// True when `seg` holds a character that is not whitespace. Bytes (UTF-8) or
/// units (UTF-16LE) from 0x80 up count as text, so a line of only exotic Unicode
/// spaces is sent rather than skipped: the safe direction.
fn has_text(seg: &[u8], encoding: Encoding) -> bool {
    let blank = |v: u16| matches!(v, 0x09..=0x0D | 0x20);
    match encoding {
        Encoding::Utf8 => seg.iter().any(|&b| !blank(u16::from(b))),
        Encoding::Utf16Le => seg
            .as_chunks::<2>()
            .0
            .iter()
            .any(|unit| !blank(u16::from_le_bytes(*unit))),
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
/// `MAX_LINES` only, and what the reader saw of the file. The file is streamed
/// in `BLOCK_BYTES` blocks (A1): every newline is counted from the first line so
/// `line_no` stays right, and only the newest raw lines are held. A failed open,
/// metadata call or read is the closed reason `read_failed`.
fn read_file(
    key: &str,
    path: &Path,
    saved: u64,
    (saved_head, saved_tail): (Option<&str>, Option<&str>),
    redaction: &Redaction,
) -> Result<(Vec<LogLine>, GameFile, u64), &'static str> {
    let mut reader = std::fs::File::open(path).map_err(|_| "read_failed")?;
    // The open handle is what gets read, and a folder or other non-file is not
    // read at all (A8).
    let meta = reader.metadata().map_err(|_| "read_failed")?;
    if !meta.is_file() {
        return Err("read_failed");
    }
    let len = meta.len();
    let mut block = vec![0u8; BLOCK_BYTES];
    let first_len = fill(&mut reader, &mut block).map_err(|_| "read_failed")?;
    let first = &block[..first_len];
    let (encoding, bom) = detect(first);
    // A tail check needs a seek, so it runs before the stream is read on: the
    // reader is put back at the end of the first block afterwards.
    let tail_changed = match saved_tail {
        Some(tail) if saved > 0 && saved <= len => {
            let same = tail_matches(&mut reader, saved, tail).map_err(|_| "read_failed")?;
            reader
                .seek(SeekFrom::Start(first_len as u64))
                .map_err(|_| "read_failed")?;
            !same
        }
        _ => false,
    };
    let restarted =
        saved > len || tail_changed || saved_head.is_some_and(|h| !head_matches(h, first));
    let mut start = if restarted { 0 } else { saved as usize };
    if encoding == Encoding::Utf16Le {
        start &= !1;
    }
    let mut scan = Scan::new(encoding, bom, start.max(bom) as u64);
    scan.push_recent(&first[..bom]);
    scan.feed(&first[bom..]);
    let head = head_of(first);
    loop {
        let n = fill(&mut reader, &mut block).map_err(|_| "read_failed")?;
        if n == 0 {
            break;
        }
        scan.feed(&block[..n]);
    }
    let total = scan.total;
    let file = GameFile {
        head,
        restarted,
        tails: scan
            .ring
            .iter()
            .map(|line| (line.end, line.tail.clone()))
            .collect(),
    };
    Ok((scan.lines(key, redaction), file, total))
}

/// Reads into `buf` until it is full or the file ends; the bytes read.
fn fill(reader: &mut std::fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// A complete non-blank line, raw: its bytes are cut to `MAX_RAW_BYTES` (A3).
struct RawLine {
    line_no: u64,
    end: u64,
    bytes: Vec<u8>,
    over: bool,
    /// The tail fingerprint at `end` (A6).
    tail: String,
}

/// The streaming line reader's state for one file: the newlines seen so far,
/// the line being read, and the newest `MAX_LINES` complete non-blank lines.
struct Scan {
    encoding: Encoding,
    /// Bytes per newline unit: 1 for UTF-8, 2 for UTF-16LE.
    unit: usize,
    /// Byte offset of the next unit to process, the first line's start at first.
    pos: u64,
    /// Bytes of a unit cut by the end of the last block.
    carry: Vec<u8>,
    /// Newlines seen so far: the `line_no` of the line that just ended.
    line_no: u64,
    /// A line ending at or before this offset is counted, not kept.
    start: u64,
    /// The line being read, up to `MAX_RAW_BYTES`.
    cur: Vec<u8>,
    /// True when the line is longer than `cur`.
    over: bool,
    /// True when the line has a character that is not whitespace.
    has_text: bool,
    /// The newest `MAX_LINES` kept lines, oldest first.
    ring: VecDeque<RawLine>,
    /// Every kept line of the file, not only the newest.
    total: u64,
    /// The last `TAIL_BYTES` bytes consumed, up to the line being ended (A6).
    recent: Vec<u8>,
}

impl Scan {
    fn new(encoding: Encoding, bom: usize, start: u64) -> Self {
        Self {
            encoding,
            unit: if encoding == Encoding::Utf16Le { 2 } else { 1 },
            pos: bom as u64,
            carry: Vec::new(),
            line_no: 0,
            start,
            cur: Vec::new(),
            over: false,
            has_text: false,
            ring: VecDeque::new(),
            total: 0,
            recent: Vec::new(),
        }
    }

    /// Adds consumed bytes to `recent`, keeping only the last `TAIL_BYTES`.
    fn push_recent(&mut self, seg: &[u8]) {
        self.recent
            .extend_from_slice(&seg[seg.len().saturating_sub(TAIL_BYTES)..]);
        let extra = self.recent.len().saturating_sub(TAIL_BYTES);
        self.recent.drain(..extra);
    }

    /// Takes the next block of the file. A unit cut by the block's end waits
    /// for the next block.
    fn feed(&mut self, data: &[u8]) {
        let mut work = std::mem::take(&mut self.carry);
        work.extend_from_slice(data);
        let usable = work.len() - work.len() % self.unit;
        self.consume(&work[..usable]);
        self.carry = work[usable..].to_vec();
    }

    /// Every complete line in `data`, which starts at `self.pos`.
    fn consume(&mut self, data: &[u8]) {
        let mut from = 0;
        while let Some(at) = self.next_newline(&data[from..]) {
            let at = from + at;
            self.append(&data[from..at]);
            self.push_recent(&data[from..at + self.unit]);
            self.line_no += 1;
            let end = self.pos + (at + self.unit) as u64;
            if end > self.start {
                self.finish(end);
            }
            self.cur.clear();
            self.over = false;
            self.has_text = false;
            from = at + self.unit;
        }
        self.append(&data[from..]);
        self.push_recent(&data[from..]);
        self.pos += data.len() as u64;
    }

    /// The index of the first newline unit in `data`, unit-aligned.
    fn next_newline(&self, data: &[u8]) -> Option<usize> {
        match self.encoding {
            Encoding::Utf8 => data.iter().position(|&b| b == b'\n'),
            Encoding::Utf16Le => data
                .as_chunks::<2>()
                .0
                .iter()
                .position(|unit| *unit == [b'\n', 0])
                .map(|i| 2 * i),
        }
    }

    /// Adds a part of the current line; the whole part decides `has_text`.
    fn append(&mut self, seg: &[u8]) {
        if !self.has_text {
            self.has_text = has_text(seg, self.encoding);
        }
        let room = MAX_RAW_BYTES - self.cur.len();
        if seg.len() > room {
            self.over = true;
        }
        self.cur.extend_from_slice(&seg[..seg.len().min(room)]);
    }

    /// Keeps the line that just ended, if it has text.
    fn finish(&mut self, end: u64) {
        if !self.has_text {
            return;
        }
        self.total += 1;
        self.ring.push_back(RawLine {
            line_no: self.line_no,
            end,
            bytes: std::mem::take(&mut self.cur),
            over: self.over,
            tail: tail_of(&self.recent),
        });
        if self.ring.len() > MAX_LINES {
            self.ring.pop_front();
        }
    }

    /// The kept lines as messages: control characters become U+FFFD (A4), the
    /// line is cut before the scrub (A3), then scrubbed and cut to the message
    /// limit.
    fn lines(&self, key: &str, redaction: &Redaction) -> Vec<LogLine> {
        self.ring
            .iter()
            .map(|raw| {
                let decoded = decode(&raw.bytes, self.encoding);
                let text: String = decoded
                    .trim_end_matches('\r')
                    .chars()
                    .map(|c| {
                        if c.is_control() && c != '\t' {
                            '\u{FFFD}'
                        } else {
                            c
                        }
                    })
                    .collect();
                let (text, raw_cut) = cut(text, MAX_RAW_BYTES);
                let scrubbed = scrub_ips(&redaction.scrub_words(&text)).into_owned();
                let (message, message_cut) = cut(scrubbed, MAX_MESSAGE_BYTES);
                LogLine {
                    file: key.to_string(),
                    line_no: raw.line_no,
                    end: raw.end,
                    // The sample proves no timestamp format worth trusting across
                    // time zones, so the row takes the read time (C-LF5).
                    ts_ms: None,
                    level: None,
                    target: None,
                    message,
                    truncated: raw.over || raw_cut || message_cut,
                }
            })
            .collect()
    }
}

/// `message`, cut to `limit` bytes on a char boundary, and whether it was.
fn cut(message: String, limit: usize) -> (String, bool) {
    if message.len() <= limit {
        return (message, false);
    }
    let mut end = limit;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    (message[..end].to_string(), true)
}

#[cfg(test)]
#[path = "game_lines_tests.rs"]
mod tests;
