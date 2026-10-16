//! Local, world-scoped spatial observations saved under the lab data directory.

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::probe_catalog::replace_file;

const MAX_BOOKMARKS: usize = 256;
const MAX_FILE_BYTES: usize = 256 * 1024;
static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// One observation of a BigWorld metre position, not a persistent spawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bookmark {
    pub id: String,
    pub instance: String,
    pub world_id: i64,
    pub space_id: Option<u64>,
    pub position_m: [f64; 3],
    pub facing_deg: Option<f64>,
    pub label: String,
    pub source: String,
    pub screenshot_path: Option<String>,
    pub observed_at_ms: i64,
}

/// User supplied fields for a new or updated bookmark.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookmarkDraft {
    pub world_id: i64,
    pub space_id: Option<u64>,
    pub position_m: [f64; 3],
    pub facing_deg: Option<f64>,
    pub label: String,
    pub source: String,
    pub screenshot_path: Option<String>,
    pub observed_at_ms: i64,
}

fn valid_instance(instance: &str) -> bool {
    !instance.is_empty()
        && instance.len() <= 64
        && instance
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn validate(bookmark: &Bookmark) -> Result<(), String> {
    if Uuid::parse_str(&bookmark.id).is_err() {
        return Err("bookmark id is not a UUID".into());
    }
    if !valid_instance(&bookmark.instance) || bookmark.world_id < 0 || bookmark.observed_at_ms <= 0
    {
        return Err("bookmark identity, world or observation time invalid".into());
    }
    if !bookmark
        .position_m
        .iter()
        .all(|n| n.is_finite() && n.abs() <= 1_000_000.0)
        || !bookmark
            .facing_deg
            .is_none_or(|n| n.is_finite() && (-360.0..=360.0).contains(&n))
    {
        return Err("bookmark coordinates or facing invalid".into());
    }
    if !valid_text(&bookmark.label, 128)
        || !valid_text(&bookmark.source, 128)
        || bookmark
            .screenshot_path
            .as_deref()
            .is_some_and(|s| !valid_text(s, 1024))
    {
        return Err("bookmark label, source or screenshot path invalid".into());
    }
    Ok(())
}

/// Refuse an action if the live instance, world or known loaded space differs.
pub fn action_allowed(
    bookmark: &Bookmark,
    instance: &str,
    world_id: Option<i64>,
    space_id: Option<u64>,
) -> Result<(), &'static str> {
    if bookmark.instance != instance {
        return Err("instance_mismatch");
    }
    if world_id != Some(bookmark.world_id) {
        return Err("cross_world_or_unknown");
    }
    if bookmark.space_id.is_some() && bookmark.space_id != space_id {
        return Err("cross_space_or_unknown");
    }
    Ok(())
}

/// Atomic, per-instance JSON store under the daemon's per-user data directory.
#[derive(Clone)]
pub struct BookmarkStore {
    root: PathBuf,
}

impl BookmarkStore {
    /// Use a caller-selected per-user lab state directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, instance: &str) -> Result<PathBuf, String> {
        if !valid_instance(instance) {
            return Err("invalid instance label".into());
        }
        Ok(self
            .root
            .join("workbench")
            .join("bookmarks")
            .join(format!("{instance}.json")))
    }

    /// List saved observations, rejecting corrupted or oversized storage.
    pub fn list(&self, instance: &str) -> Result<Vec<Bookmark>, String> {
        let path = self.path(instance)?;
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.to_string()),
        };
        if bytes.len() > MAX_FILE_BYTES {
            return Err("bookmark file too large".into());
        }
        let rows: Vec<Bookmark> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if rows.len() > MAX_BOOKMARKS {
            return Err("bookmark count exceeds 256".into());
        }
        let mut ids = std::collections::HashSet::new();
        for row in &rows {
            validate(row)?;
            if row.instance != instance || !ids.insert(row.id.as_str()) {
                return Err("bookmark instance or id mismatch".into());
            }
        }
        Ok(rows)
    }

    fn write(&self, instance: &str, rows: &[Bookmark]) -> Result<(), String> {
        let path = self.path(instance)?;
        let parent = path.parent().ok_or("bookmark directory missing")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(rows).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err("bookmark file too large".into());
        }
        let temp = parent.join(format!(".{instance}.{}.tmp", Uuid::new_v4()));
        fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        if let Err(e) = replace_file(&temp, &path) {
            let _ = fs::remove_file(&temp);
            return Err(e.to_string());
        }
        Ok(())
    }

    /// Create a stable UUID, or update the named observation in place.
    pub fn save(
        &self,
        instance: &str,
        id: Option<String>,
        draft: BookmarkDraft,
    ) -> Result<Bookmark, String> {
        let _guard = WRITE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut rows = self.list(instance)?;
        let row = Bookmark {
            id: id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
            instance: instance.into(),
            world_id: draft.world_id,
            space_id: draft.space_id,
            position_m: draft.position_m,
            facing_deg: draft.facing_deg,
            label: draft.label,
            source: draft.source,
            screenshot_path: draft.screenshot_path,
            observed_at_ms: draft.observed_at_ms,
        };
        validate(&row)?;
        if let Some(index) = rows.iter().position(|old| old.id == row.id) {
            rows[index] = row.clone();
        } else if id.is_some() {
            return Err("bookmark_missing".into());
        } else if rows.len() >= MAX_BOOKMARKS {
            return Err("bookmark count exceeds 256".into());
        } else {
            rows.push(row.clone());
        }
        self.write(instance, &rows)?;
        Ok(row)
    }

    /// Delete only an observation belonging to this instance.
    pub fn delete(&self, instance: &str, id: &str) -> Result<(), String> {
        let _guard = WRITE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut rows = self.list(instance)?;
        let before = rows.len();
        rows.retain(|row| row.id != id);
        if rows.len() == before {
            return Err("bookmark_missing".into());
        }
        self.write(instance, &rows)
    }
}
