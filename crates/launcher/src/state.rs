//! Persistent local state files.
//!
//! - [`InstalledState`] sits inside the game install directory and tracks
//!   which manifest patches have been applied (so reruns are no-ops).
//! - [`UploadedLedger`] sits next to the launcher .exe and tracks log-zip
//!   content digests already uploaded, so we don't double-pay storage on
//!   repeat "Upload Logs" clicks.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct InstalledState {
    #[serde(default)]
    pub applied_patches: Vec<String>,
    #[serde(default)]
    pub seed_sha256: Option<String>,
}

impl InstalledState {
    pub fn path(install_dir: &Path) -> PathBuf {
        install_dir.join("launcher-installed.json")
    }

    pub fn load(install_dir: &Path) -> Self {
        let path = Self::path(install_dir);
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, install_dir: &Path) -> Result<(), StateError> {
        let path = Self::path(install_dir);
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn has_applied(&self, patch_id: &str) -> bool {
        self.applied_patches.iter().any(|p| p == patch_id)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UploadedLedger {
    #[serde(default)]
    pub entries: Vec<UploadedEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedEntry {
    pub sha256: String,
    pub blob_name: String,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

impl UploadedLedger {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), StateError> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn contains(&self, sha256: &str) -> bool {
        self.entries.iter().any(|e| e.sha256 == sha256)
    }

    pub fn record(&mut self, sha256: String, blob_name: String) {
        self.entries.push(UploadedEntry {
            sha256,
            blob_name,
            uploaded_at: chrono::Utc::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_state_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let s = InstalledState {
            applied_patches: vec!["a".into(), "b".into()],
            seed_sha256: Some("h".into()),
        };
        s.save(dir.path()).unwrap();
        let loaded = InstalledState::load(dir.path());
        assert_eq!(loaded.applied_patches, vec!["a", "b"]);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("h"));
        assert!(loaded.has_applied("a"));
        assert!(!loaded.has_applied("c"));
    }

    #[test]
    fn installed_state_missing_file_is_default() {
        let dir = tempfile::tempdir().unwrap();
        let s = InstalledState::load(dir.path());
        assert!(s.applied_patches.is_empty());
        assert!(s.seed_sha256.is_none());
    }

    #[test]
    fn uploaded_ledger_dedupe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("u.json");
        let mut led = UploadedLedger::default();
        assert!(!led.contains("h1"));
        led.record("h1".into(), "logs/x.zip".into());
        led.save(&path).unwrap();
        let loaded = UploadedLedger::load(&path);
        assert!(loaded.contains("h1"));
        assert!(!loaded.contains("h2"));
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].blob_name, "logs/x.zip");
    }
}
