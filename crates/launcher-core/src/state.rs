//! Persistent local state files.
//!
//! - [`InstalledState`] sits inside the game install directory and tracks
//!   which manifest patches have been applied (so reruns are no-ops).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::manifest::PatchEntry;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

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
    /// Patches that applied with some ops skipped by a tolerant apply. A
    /// patch listed here is also in `applied_patches`; see
    /// [`InstalledState::is_fully_applied`]. Left out of the file when empty,
    /// so a ledger without skips keeps its bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incomplete_patches: Vec<IncompletePatch>,
}

/// A target a tolerant apply skipped, as the ledger records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedTarget {
    /// Client-root relative path, `/` separated.
    pub path: String,
    /// The skip code, `SkipReason::code` (for example `source_missing`).
    pub reason: String,
}

/// A patch that applied with skipped ops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompletePatch {
    /// The patch's state key, as in `applied_patches`.
    pub state_key: String,
    /// The skipped targets, in the order the apply reported them.
    pub skipped: Vec<SkippedTarget>,
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

    /// Whether `patch` is recorded as applied. The record is the patch's
    /// [`PatchEntry::state_key`], not its bare id: a `sgw_game` patch is
    /// stored as `<id>@sgw_game`. Looking one up by id reported it
    /// missing forever and kept "Install / Update" lit with nothing to do,
    /// so every caller goes through this method.
    pub fn has_applied_patch(&self, patch: &PatchEntry) -> bool {
        self.has_applied_key(&patch.state_key())
    }

    /// The manifest patches not yet applied, in manifest order.
    pub fn missing_patches<'a>(&self, patches: &'a [PatchEntry]) -> Vec<&'a PatchEntry> {
        patches
            .iter()
            .filter(|p| !self.has_applied_patch(p))
            .collect()
    }

    fn has_applied_key(&self, key: &str) -> bool {
        self.applied_patches.iter().any(|p| p == key)
    }

    /// Whether `state_key` is recorded as applied with no skipped ops.
    pub fn is_fully_applied(&self, state_key: &str) -> bool {
        self.has_applied_key(state_key)
            && !self
                .incomplete_patches
                .iter()
                .any(|p| p.state_key == state_key)
    }

    /// The skipped ops across every incomplete patch.
    pub fn skipped_op_count(&self) -> usize {
        self.incomplete_patches
            .iter()
            .map(|p| p.skipped.len())
            .sum()
    }

    /// Record the skipped targets of `state_key`, replacing an earlier
    /// entry in place. An empty `skipped` removes the entry.
    pub fn record_incomplete(&mut self, state_key: &str, skipped: Vec<SkippedTarget>) {
        let at = self
            .incomplete_patches
            .iter()
            .position(|p| p.state_key == state_key);
        match (at, skipped.is_empty()) {
            (Some(i), true) => {
                self.incomplete_patches.remove(i);
            }
            (Some(i), false) => self.incomplete_patches[i].skipped = skipped,
            (None, false) => self.incomplete_patches.push(IncompletePatch {
                state_key: state_key.to_owned(),
                skipped,
            }),
            (None, true) => {}
        }
    }
}

/// Per-session runtime state for the telemetry pipeline. Separate
/// from `LauncherConfig` so frequent saves don't churn the user-
/// facing config file. All fields default so a corrupt file loads as
/// zero-state — telemetry state is never load-bearing.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetryState {
    #[serde(default)]
    pub last_upload_ms: Option<i64>,
    #[serde(default)]
    pub last_kill_switch_check_ms: Option<i64>,
    #[serde(default)]
    pub kill_switch_active: bool,
    /// Cached so a final bundle can flush even if the auth endpoint
    /// is unreachable at game-exit time.
    #[serde(default)]
    pub last_upload_endpoint: Option<String>,
}

impl TelemetryState {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
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
            seed_adopted: false,
            ..Default::default()
        };
        s.save(dir.path()).unwrap();
        let loaded = InstalledState::load(dir.path());
        assert_eq!(loaded.applied_patches, vec!["a", "b"]);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("h"));
        assert!(!loaded.seed_adopted);
        assert!(loaded.has_applied_key("a"));
        assert!(!loaded.has_applied_key("c"));
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
            seed_adopted: true,
            ..Default::default()
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
            r#"{"applied_patches":["a"],"seed_sha256":"h","patched_host":"play.cimmeria.app"}"#,
        )
        .unwrap();
        let loaded = InstalledState::load(dir.path());
        assert!(!loaded.seed_adopted);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("h"));
    }

    // State files from launchers that still did the `.rdata` hostname
    // patch carry a `patched_host` field; it is ignored, and the rest loads.
    #[test]
    fn installed_state_ignores_the_retired_patched_host_field() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            InstalledState::path(dir.path()),
            r#"{"applied_patches":["a"],"seed_sha256":"h","patched_host":"x"}"#,
        )
        .unwrap();
        let loaded = InstalledState::load(dir.path());
        assert_eq!(loaded.applied_patches, vec!["a"]);
        assert_eq!(loaded.seed_sha256.as_deref(), Some("h"));
    }

    #[test]
    fn installed_state_missing_file_is_default() {
        let dir = tempfile::tempdir().unwrap();
        let s = InstalledState::load(dir.path());
        assert!(s.applied_patches.is_empty());
        assert!(s.seed_sha256.is_none());
    }

    // Default-state semantics: zero-value of every field, including
    // kill_switch_active = false. This is the in-memory shape used
    // before the first save and after a corrupt load.
    #[test]
    fn telemetry_state_default_is_zero_value() {
        let s = TelemetryState::default();
        assert_eq!(s.last_upload_ms, None);
        assert_eq!(s.last_kill_switch_check_ms, None);
        assert!(!s.kill_switch_active);
        assert_eq!(s.last_upload_endpoint, None);
    }

    // load() roundtrip via direct JSON write — save() will land
    // alongside the actual write callers in a later commit.
    #[test]
    fn telemetry_state_roundtrip_preserves_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry-state.json");
        let s = TelemetryState {
            last_upload_ms: Some(1_700_000_000_000),
            last_kill_switch_check_ms: Some(1_700_000_005_000),
            kill_switch_active: true,
            last_upload_endpoint: Some("https://func.example.net/api".into()),
        };
        std::fs::write(&path, serde_json::to_vec_pretty(&s).unwrap()).unwrap();
        let loaded = TelemetryState::load(&path);
        assert_eq!(loaded, s);
    }

    // Missing file is "no state yet" — the load path returns default
    // instead of an error. This is intentional: telemetry state is
    // never load-bearing (worst case is one extra auth handshake), so
    // a fresh install / first launch shouldn't have to special-case
    // "file doesn't exist."
    #[test]
    fn telemetry_state_load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("never-existed.json");
        let s = TelemetryState::load(&path);
        assert_eq!(s, TelemetryState::default());
    }

    // Corrupt file is also "no state yet" — same rationale. A
    // half-written file from a crash mid-save shouldn't block the
    // next launch.
    #[test]
    fn telemetry_state_load_corrupt_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry-state.json");
        std::fs::write(&path, b"{ not valid json").unwrap();
        let s = TelemetryState::load(&path);
        assert_eq!(s, TelemetryState::default());
    }

    // Forward-compat: a future launcher writing extra fields must not
    // break an older launcher loading the same file — serde ignores
    // unknown fields by default. Pin that behaviour so a stray
    // `deny_unknown_fields` attribute would surface as a test diff.
    #[test]
    fn telemetry_state_load_ignores_unknown_future_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry-state.json");
        std::fs::write(
            &path,
            r#"{"kill_switch_active":true,"future_field_v2":"some-value","another_future":42}"#,
        )
        .unwrap();
        let s = TelemetryState::load(&path);
        assert!(s.kill_switch_active);
    }

    // Partial JSON with only some fields set must load with the
    // unspecified fields defaulted. This is the cross-version
    // compatibility story: a future launcher adding a new field
    // shouldn't break old launchers that load the same file (they'll
    // just ignore the new field thanks to serde's permissive default).
    #[test]
    fn telemetry_state_load_partial_json_defaults_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telemetry-state.json");
        std::fs::write(&path, r#"{"kill_switch_active":true}"#).unwrap();
        let s = TelemetryState::load(&path);
        assert!(s.kill_switch_active);
        assert_eq!(s.last_upload_ms, None);
    }

    fn sgw_game_patch(id: &str) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: format!("p/{id}.zip"),
            size: 1,
            sha256: format!("h-{id}"),
            after: None,
            root: crate::manifest::PatchRoot::SgwGame,
            title: None,
            description: None,
        }
    }

    // Bug shape: the install panel looked patches up by bare id, but a
    // `sgw_game` patch is recorded as `<id>@sgw_game`, so the overlay
    // stayed "missing" after a good install and Install / Update stayed
    // lit with nothing to apply.
    #[test]
    fn an_applied_sgw_game_patch_is_not_missing() {
        let overlay = sgw_game_patch("bm-ui-overlay-11b3bae0a2b7");
        let state = InstalledState {
            applied_patches: vec!["base".into(), "bm-ui-overlay-11b3bae0a2b7@sgw_game".into()],
            seed_sha256: Some("h".into()),
            seed_adopted: false,
            ..Default::default()
        };
        assert!(state.has_applied_patch(&overlay));
        let mut base = sgw_game_patch("base");
        base.root = crate::manifest::PatchRoot::InstallDir;
        let patches = vec![base, overlay];
        assert!(state.missing_patches(&patches).is_empty());
    }

    /// The other direction: a bare-id record (written by a launcher that
    /// predates `root`, which extracted into the wrong directory) must
    /// still count as missing so the patch is applied again.
    #[test]
    fn a_bare_id_record_does_not_satisfy_an_sgw_game_patch() {
        let overlay = sgw_game_patch("overlay");
        let state = InstalledState {
            applied_patches: vec!["overlay".into()],
            seed_sha256: Some("h".into()),
            seed_adopted: false,
            ..Default::default()
        };
        assert!(!state.has_applied_patch(&overlay));
        let patches = vec![overlay];
        let missing = state.missing_patches(&patches);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id, "overlay");
    }

    // A ledger written before `incomplete_patches` existed loads, and saving
    // it again gives the same bytes: the empty list is not written. This is
    // the downloaded-install guard for the ledger.
    #[test]
    fn ledger_without_incomplete_patches_roundtrips_byte_identically() {
        let dir = tempfile::tempdir().unwrap();
        let text = "{\n  \"applied_patches\": [\n    \"a\"\n  ],\n  \"seed_sha256\": \"h\",\n  \"seed_adopted\": false\n}";
        std::fs::write(InstalledState::path(dir.path()), text).unwrap();
        let loaded = InstalledState::load(dir.path());
        assert!(loaded.incomplete_patches.is_empty());
        loaded.save(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(InstalledState::path(dir.path())).unwrap(),
            text
        );
    }

    #[test]
    fn incomplete_patches_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = InstalledState {
            applied_patches: vec!["a".into()],
            ..Default::default()
        };
        let skipped = vec![SkippedTarget {
            path: "Working/SGWGame/Maps/cell.umap".into(),
            reason: "source_mismatch".into(),
        }];
        s.record_incomplete("a", skipped.clone());
        s.save(dir.path()).unwrap();
        let loaded = InstalledState::load(dir.path());
        assert_eq!(
            loaded.incomplete_patches,
            vec![IncompletePatch {
                state_key: "a".into(),
                skipped,
            }]
        );
    }

    #[test]
    fn is_fully_applied_is_false_for_an_incomplete_key_and_true_for_a_clean_one() {
        let mut s = InstalledState {
            applied_patches: vec!["a".into(), "b".into()],
            ..Default::default()
        };
        s.record_incomplete(
            "a",
            vec![SkippedTarget {
                path: "x.upk".into(),
                reason: "source_missing".into(),
            }],
        );
        assert!(!s.is_fully_applied("a"));
        assert!(s.is_fully_applied("b"));
        assert!(!s.is_fully_applied("c"));
        assert_eq!(s.skipped_op_count(), 1);
        // An empty record removes the entry again.
        s.record_incomplete("a", Vec::new());
        assert!(s.is_fully_applied("a"));
        assert!(s.incomplete_patches.is_empty());
    }
}
