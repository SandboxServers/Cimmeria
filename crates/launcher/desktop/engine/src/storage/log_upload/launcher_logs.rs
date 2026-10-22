//! The launcher's own logs in the player log upload (launcher-telemetry LT-21b;
//! D-LT11, D-LT18). The shell writes `launcher.<yyyy-mm-dd>.log` under
//! `<app_data>/logs`. The newest three are scrubbed line by line (home folder,
//! absolute paths, IP literals) and added to the game's zip under `launcher/`.
//! Only the uploaded copy is scrubbed: the local files are never changed.
//!
//! Known limits, accepted: the digest and the "nothing new" check cover only
//! the game logs, and with no game logs nothing is uploaded, launcher logs
//! included.
use super::{events, LogUploadError};
use crate::storage::game_telemetry::ip_scrub::scrub_ips;
use crate::storage::game_telemetry::launcher_event::Redaction;
use cimmeria_launcher_core::logs;
use std::io::Write;
use std::path::Path;

/// The most launcher log bytes one upload adds, counted after scrubbing.
pub(super) const LAUNCHER_LOG_CAP: usize = 4 * 1024 * 1024;

/// How many of the newest launcher log files one upload includes.
const KEEP_FILES: usize = 3;

/// One scrubbed launcher log for the zip: its entry name, its bytes and its
/// modification time as a zip timestamp (`None` if it has none).
pub(super) type LauncherEntry = (String, Vec<u8>, Option<zip::DateTime>);

/// The scrubbed launcher log entries for the newest launcher logs in `dir`,
/// oldest first, within `cap` bytes. The oldest lines go first. A missing
/// folder or an unreadable file gives no entry.
pub(super) fn entries(dir: &Path, redaction: &Redaction, cap: usize) -> Vec<LauncherEntry> {
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
    let newest = names.split_off(names.len().saturating_sub(KEEP_FILES));

    let mut files: Vec<(String, Vec<String>, Option<zip::DateTime>)> = newest
        .into_iter()
        .filter_map(|name| {
            let bytes = std::fs::read(dir.join(&name)).ok()?;
            // The file's own time, so the server's newest-first replay sorts it.
            let modified = std::fs::metadata(dir.join(&name))
                .and_then(|m| m.modified())
                .ok()
                .and_then(logs::zip_time);
            let lines = String::from_utf8_lossy(&bytes)
                .lines()
                .map(|line| scrub_ips(&redaction.scrub(line)).into_owned())
                .collect();
            Some((name, lines, modified))
        })
        .collect();

    // Each line is counted with its newline, as it is written below.
    let mut total: usize = files
        .iter()
        .flat_map(|(_, lines, _)| lines)
        .map(|line| line.len() + 1)
        .sum();
    for (_, lines, _) in &mut files {
        let mut drop = 0;
        while total > cap && drop < lines.len() {
            total -= lines[drop].len() + 1;
            drop += 1;
        }
        lines.drain(..drop);
    }

    files
        .into_iter()
        .filter(|(_, lines, _)| !lines.is_empty())
        .map(|(name, lines, modified)| {
            let mut bytes = Vec::new();
            for line in lines {
                bytes.extend_from_slice(line.as_bytes());
                bytes.push(b'\n');
            }
            (format!("launcher/{name}"), bytes, modified)
        })
        .collect()
}

/// A zip error on the way to the player's zip, reported as `CollectFailed`.
/// Fails the upload, so the event is what tells a reader why nothing was sent.
fn failed<E>(_: E) -> LogUploadError {
    events::collect_failed("launcher_logs_zip");
    LogUploadError::CollectFailed
}

/// `body` (a game zip) with each entry added, deflated, each with its own
/// modification time. A zip error maps to `CollectFailed`.
pub(super) fn append(body: Vec<u8>, entries: &[LauncherEntry]) -> Result<Vec<u8>, LogUploadError> {
    let mut zip = zip::ZipWriter::new_append(std::io::Cursor::new(body)).map_err(failed)?;
    let base = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes, modified) in entries {
        let options = match modified {
            Some(t) => base.last_modified_time(*t),
            None => base,
        };
        zip.start_file(name.as_str(), options).map_err(failed)?;
        zip.write_all(bytes).map_err(failed)?;
    }
    let cursor = zip.finish().map_err(failed)?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
#[path = "launcher_logs_tests.rs"]
mod tests;
