//! Writing a batch folder (used by `lab uat`, LX-15 to LX-17).

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;

use super::{Plan, RunRecord};

static APPEND: Mutex<()> = Mutex::new(());

/// Writes `plan.json`.
pub fn write_plan(path: &Path, plan: &Plan) -> io::Result<()> {
    let text = serde_json::to_string_pretty(plan).map_err(io::Error::other)?;
    fs::write(path, text)
}

/// Appends one record to `batch.jsonl` as one line.
pub fn append_record(path: &Path, record: &RunRecord) -> io::Result<()> {
    let mut line = serde_json::to_string(record).map_err(io::Error::other)?;
    line.push('\n');
    let _guard = APPEND.lock().unwrap_or_else(|e| e.into_inner());
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())
}
