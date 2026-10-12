//! The recording state file, `<lab home>\record-<instance>.json`: it names the
//! recording `lab record start` began, so `stop` needs no id.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::checks::is_instance;

/// What the state file holds. A file without `instance` or `specs_dir` still reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordState {
    /// The recording the daemon is holding.
    pub recording_id: String,
    /// The instance it runs on.
    pub instance: Option<String>,
    /// The spec folder fixed at start, so the stop can print the draft's path.
    pub specs_dir: Option<String>,
}

/// The file as the CLI writes it: field order is the file's key order.
#[derive(Serialize)]
struct StateFile<'a> {
    recording_id: &'a str,
    instance: &'a str,
    #[serde(skip_serializing_if = "is_blank")]
    specs_dir: &'a str,
}

fn is_blank(text: &&str) -> bool {
    text.is_empty()
}

/// `<lab_home>\record-<instance>.json`. The name is checked first because it
/// becomes part of a path: `Err("bad instance name '<x>'")` when it is not an
/// instance name.
pub fn state_path(lab_home: &Path, instance: &str) -> Result<PathBuf, String> {
    if !is_instance(instance) {
        return Err(format!("bad instance name '{instance}'"));
    }
    Ok(lab_home.join(format!("record-{instance}.json")))
}

/// Writes the state file, creating the folder. `specs_dir` is empty when the
/// start sent none (then the key is left out).
pub fn save_state(
    path: &Path,
    recording_id: &str,
    instance: &str,
    specs_dir: &str,
) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = StateFile {
        recording_id,
        instance,
        specs_dir,
    };
    let text = serde_json::to_string(&file).map_err(io::Error::other)?;
    std::fs::write(path, format!("{text}\n"))
}

/// The state, or `None` when there is no file, it cannot be read, or it has
/// no non-empty `recording_id`.
pub fn read_state(path: &Path) -> Option<RecordState> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let object = value.as_object()?;
    let recording_id = object.get("recording_id")?.as_str()?.to_owned();
    if recording_id.is_empty() {
        return None;
    }
    let text_of = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    Some(RecordState {
        recording_id,
        instance: text_of("instance"),
        specs_dir: text_of("specs_dir"),
    })
}
