//! The launcher's log lines for the debug-log upload (launcher-logs LF-13).
//! `collect` reads the `launcher.<yyyy-mm-dd>.log` files the shell writes,
//! oldest file first, from each file's saved offset to its last complete
//! line. A line that starts with the appender's timestamp begins a record;
//! any other line is folded into the record before it. Nothing is sent here.
use super::offsets::Offsets;
use super::LogUploadError;
use crate::storage::game_telemetry::ip_scrub::scrub_ips;
use crate::storage::game_telemetry::launcher_event::Redaction;
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// The most lines `collect` keeps: the newest, across all files.
pub(crate) const MAX_LINES: usize = 20_000;

/// The longest message kept, in bytes; a longer one is cut on a char boundary.
const MAX_MESSAGE_BYTES: usize = 2_048;

/// The levels the appender writes, as `tracing` spells them.
const LEVELS: [&str; 5] = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

/// One record of a launcher log file, ready to become a telemetry row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogLine {
    /// The log file's name, never its path.
    pub file: String,
    /// The 1-based number of the record's first physical line in the file.
    pub line_no: u64,
    /// The byte offset just past the record's last `\n`.
    pub end: u64,
    /// The appender's timestamp in milliseconds, when the line has one.
    pub ts_ms: Option<i64>,
    /// The appender's level (`INFO`, ...), when the line has one.
    pub level: Option<String>,
    /// The appender's target (the module path), when the line has one.
    pub target: Option<String>,
    /// The scrubbed text, cut to 2,048 bytes.
    pub message: String,
    /// True when the message was cut.
    pub truncated: bool,
}

/// The lines `collect` read: oldest first, at most the newest `MAX_LINES`,
/// and how many complete new lines there were before that cut.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Collected {
    pub lines: Vec<LogLine>,
    pub lines_total: u64,
}

/// The new lines in every launcher log file under `log_dir`, from the offsets
/// in `offsets`. A missing folder is no lines. A file that cannot be read is
/// skipped, and its offset stays where it was. A cancel, checked before each
/// file, returns `CollectFailed`: the caller has no other error for it.
pub(crate) fn collect(
    log_dir: &Path,
    offsets: &Offsets,
    cancel: &CancellationToken,
) -> Result<Collected, LogUploadError> {
    let cancelled = || {
        if cancel.is_cancelled() {
            Err(LogUploadError::CollectFailed)
        } else {
            Ok(())
        }
    };
    cancelled()?;
    let redaction = Redaction::from_env();
    let mut collected = Collected::default();
    for name in launcher_files(log_dir) {
        cancelled()?;
        let saved = offsets.files.get(&name).copied().unwrap_or(0);
        let Ok(bytes) = std::fs::read(log_dir.join(&name)) else {
            continue;
        };
        let lines = read_file(&name, &bytes, saved, &redaction);
        collected.lines_total += lines.len() as u64;
        collected.lines.extend(lines);
        // Keep the memory bound at one file's lines plus the newest MAX_LINES.
        let excess = collected.lines.len().saturating_sub(MAX_LINES);
        collected.lines.drain(..excess);
    }
    Ok(collected)
}

/// The launcher log file names in `dir`, oldest first (the names sort by date).
fn launcher_files(dir: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("launcher.") && name.ends_with(".log"))
        .collect();
    names.sort();
    names
}

/// The complete records of one file's `bytes` from `saved`. A file now shorter
/// than `saved` was rotated or rewritten, so it is read from 0.
fn read_file(name: &str, bytes: &[u8], saved: u64, redaction: &Redaction) -> Vec<LogLine> {
    let len = bytes.len() as u64;
    let start = if saved > len { 0 } else { saved as usize };
    // Only up to the last `\n`: a half-written line waits for its newline.
    let end = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    if end <= start {
        return Vec::new();
    }
    let lines_before = bytes[..start].iter().filter(|&&b| b == b'\n').count() as u64;
    let records = fold_records(&bytes[start..end], start as u64, lines_before);
    records
        .into_iter()
        .map(|record| finish(name, record, redaction))
        .collect()
}

/// A record while its continuation lines are still being folded in.
struct Record {
    line_no: u64,
    end: u64,
    /// True when the first line starts with the appender's timestamp.
    starts: bool,
    text: String,
}

/// Splits `window` (file bytes from `start`, all ending in `\n`) into records.
/// `lines_before` is how many physical lines the file has before `start`.
/// Empty lines are dropped, but still move the offset past them.
fn fold_records(window: &[u8], start: u64, lines_before: u64) -> Vec<Record> {
    let mut records: Vec<Record> = Vec::new();
    let mut pos = 0u64;
    let numbers = (lines_before + 1)..;
    for (index, raw) in numbers.zip(window.split_inclusive(|&b| b == b'\n')) {
        pos += raw.len() as u64;
        let line_end = start + pos;
        let body = raw.strip_suffix(b"\n").unwrap_or(raw);
        let body = body.strip_suffix(b"\r").unwrap_or(body);
        let text = String::from_utf8_lossy(body);
        let text = text.trim();
        if text.is_empty() {
            if let Some(last) = records.last_mut() {
                last.end = line_end;
            }
            continue;
        }
        let starts = starts_record(text);
        if !starts {
            if let Some(last) = records.last_mut() {
                last.text.push_str(" | ");
                last.text.push_str(text);
                last.end = line_end;
                continue;
            }
        }
        records.push(Record {
            line_no: index,
            end: line_end,
            starts,
            text: text.to_string(),
        });
    }
    records
}

/// Turns a finished record into a line: the appender's fields when it parses,
/// the whole text otherwise. The message is scrubbed, then cut.
fn finish(file: &str, record: Record, redaction: &Redaction) -> LogLine {
    let header = if record.starts {
        parse_header(&record.text)
    } else {
        None
    };
    let (ts_ms, level, target, message) = match header {
        Some(h) => (Some(h.ts_ms), Some(h.level), h.target, h.message),
        None => (None, None, None, record.text.as_str()),
    };
    let scrubbed = scrub_ips(&redaction.scrub(message)).into_owned();
    let (message, truncated) = cut(scrubbed);
    LogLine {
        file: file.to_string(),
        line_no: record.line_no,
        end: record.end,
        ts_ms,
        level: level.map(str::to_string),
        target: target.map(str::to_string),
        message,
        truncated,
    }
}

/// The parts of a line in the appender's format.
struct Header<'a> {
    ts_ms: i64,
    level: &'a str,
    target: Option<&'a str>,
    message: &'a str,
}

/// Parses `2026-10-22T12:34:56.123456Z  INFO target: message`. None when the
/// line does not have that shape.
fn parse_header(text: &str) -> Option<Header<'_>> {
    let (ts, rest) = text.split_once(' ')?;
    let ts_ms = chrono::DateTime::parse_from_rfc3339(ts)
        .ok()?
        .timestamp_millis();
    let rest = rest.trim_start();
    let (level, rest) = rest.split_once(' ').unwrap_or((rest, ""));
    if !LEVELS.contains(&level) {
        return None;
    }
    let rest = rest.trim_start();
    let (target, message) = match rest.split_once(": ") {
        Some((target, message)) if !target.contains(' ') => (Some(target), message),
        _ => match rest.strip_suffix(':') {
            Some(target) if !target.contains(' ') => (Some(target), ""),
            _ => (None, rest),
        },
    };
    Some(Header {
        ts_ms,
        level,
        target,
        message,
    })
}

/// True when `text` begins with an RFC 3339 timestamp and a space, which is
/// how the appender starts every record.
fn starts_record(text: &str) -> bool {
    text.split_once(' ')
        .is_some_and(|(ts, _)| chrono::DateTime::parse_from_rfc3339(ts).is_ok())
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
#[path = "lines_tests.rs"]
mod tests;
