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

/// Maximum log-upload ledger entries kept in memory + on disk. Each entry
/// is ~100 bytes; capping at 100 keeps the file under 10 KB even after a
/// thousand upload clicks. Display use is "have we seen this digest?" so
/// keeping a sliding window of the most-recent N is sufficient.
pub const LEDGER_KEEP: usize = 100;

/// Atomic write: stage to `<path>.tmp` then rename onto `path`. NTFS
/// rename is atomic on the same volume, so a power loss between the
/// truncate and the finish either leaves the old content or the new
/// content — never a half-written file that the load path would treat
/// as "corrupted, fall back to default" (which would silently mean
/// "nothing installed → re-download the seed").
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct InstalledState {
    #[serde(default)]
    pub applied_patches: Vec<String>,
    #[serde(default)]
    pub seed_sha256: Option<String>,
    /// Host that was last written into SGW.exe's `.rdata`. Lets the
    /// installer detect a `server_host` config change and re-patch even
    /// after the original CME literal has already been replaced. `None`
    /// for installs predating this field (treated as "patched host unknown").
    #[serde(default)]
    pub patched_host: Option<String>,
    /// True when the seed entry was recorded by an "Adopt existing
    /// install" flow rather than a real seed download + extract. The
    /// hash on `seed_sha256` was copied from the manifest, NOT computed
    /// over the on-disk bytes — so for adopted installs we trust the
    /// user's assertion that the files match what the manifest's seed
    /// would have produced. Patches still apply on top normally; the
    /// flag exists so the UI can surface "(adopted — unverified)" next
    /// to the install state and so a future hash audit can find these
    /// rows.
    #[serde(default)]
    pub seed_adopted: bool,
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
        let bytes = serde_json::to_vec_pretty(self)?;
        atomic_write(&path, &bytes)?;
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
        let bytes = serde_json::to_vec_pretty(self)?;
        atomic_write(path, &bytes)?;
        Ok(())
    }

    pub fn contains(&self, sha256: &str) -> bool {
        self.entries.iter().any(|e| e.sha256 == sha256)
    }

    /// Records a new upload and prunes oldest entries beyond
    /// [`LEDGER_KEEP`]. Callers persist with `save()`.
    pub fn record(&mut self, sha256: String, blob_name: String) {
        self.entries.push(UploadedEntry {
            sha256,
            blob_name,
            uploaded_at: chrono::Utc::now(),
        });
        if self.entries.len() > LEDGER_KEEP {
            let overflow = self.entries.len() - LEDGER_KEEP;
            self.entries.drain(0..overflow);
        }
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
            patched_host: Some("play.cimmeria.gg".into()),
            seed_adopted: false,
        };
        s.save(dir.path()).unwrap();
        let loaded = InstalledState::load(dir.path());
        assert_eq!(loaded.applied_patches, vec!["a", "b"]);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("h"));
        assert_eq!(loaded.patched_host.as_deref(), Some("play.cimmeria.gg"));
        assert!(!loaded.seed_adopted);
        assert!(loaded.has_applied("a"));
        assert!(!loaded.has_applied("c"));
    }

    // Adopt-existing-install path writes `seed_adopted: true` alongside
    // the manifest-copied seed hash. The flag must roundtrip so the UI
    // can surface "(adopted)" on subsequent launches.
    #[test]
    fn installed_state_roundtrip_adopted() {
        let dir = tempfile::tempdir().unwrap();
        let s = InstalledState {
            applied_patches: vec![],
            seed_sha256: Some("manifest-seed-hash".into()),
            patched_host: None,
            seed_adopted: true,
        };
        s.save(dir.path()).unwrap();
        let loaded = InstalledState::load(dir.path());
        assert!(loaded.seed_adopted);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("manifest-seed-hash"));
    }

    // Legacy state files written before `seed_adopted` existed must
    // default the field to false (real install, not adopted) so the
    // UI doesn't suddenly start flagging healthy installs.
    #[test]
    fn installed_state_legacy_without_seed_adopted_defaults_false() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            InstalledState::path(dir.path()),
            r#"{"applied_patches":["a"],"seed_sha256":"h","patched_host":"play.cimmeria.gg"}"#,
        )
        .unwrap();
        let loaded = InstalledState::load(dir.path());
        assert!(!loaded.seed_adopted);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("h"));
    }

    // Forwards-compat: state files written before the patched_host field
    // existed should deserialize with patched_host = None and otherwise
    // load normally.
    #[test]
    fn installed_state_loads_legacy_without_patched_host() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            InstalledState::path(dir.path()),
            r#"{"applied_patches":["a"],"seed_sha256":"h"}"#,
        )
        .unwrap();
        let loaded = InstalledState::load(dir.path());
        assert_eq!(loaded.applied_patches, vec!["a"]);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("h"));
        assert!(loaded.patched_host.is_none());
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
