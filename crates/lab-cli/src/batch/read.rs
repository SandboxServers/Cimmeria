//! Reading a batch folder. A lane may be appending while a watcher reads, so a
//! line that does not parse (a half-written last line) is skipped, never an
//! error.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{Plan, RunRecord};

fn read_text(path: &Path) -> io::Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// A batch's `plan.json`, or `None` when it is missing, unreadable or not a
/// plan (a batch from before plan.json existed has none).
pub fn read_plan(path: &Path) -> Option<Plan> {
    let text = read_text(path).ok()??;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

/// The records of `batch.jsonl`. A missing file is no records; a line that is
/// blank or does not parse as a record is skipped; any other I/O error (a
/// locked or denied file) is returned.
pub fn read_records(path: &Path) -> io::Result<Vec<RunRecord>> {
    let Some(text) = read_text(path)? else {
        return Ok(Vec::new());
    };
    Ok(text
        .lines()
        .map(|line| line.trim().trim_start_matches('\u{feff}'))
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<RunRecord>(line).ok())
        .collect())
}

/// The `batch-*` folder under `root` with the greatest name (names are
/// `batch-yyyyMMdd-HHmmss`, so name order is start order; file times are not),
/// or `None` when `root` is missing or holds none. Files named `batch-*` are
/// ignored. Any other I/O error is returned.
pub fn find_latest_batch(root: &Path) -> io::Result<Option<PathBuf>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(None)
        }
        Err(e) => return Err(e),
    };
    let mut best: Option<(String, PathBuf)> = None;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_ascii_lowercase().starts_with("batch-") || !entry.path().is_dir() {
            continue;
        }
        if best.as_ref().is_none_or(|(top, _)| name > *top) {
            best = Some((name, entry.path()));
        }
    }
    Ok(best.map(|(_, path)| path))
}
