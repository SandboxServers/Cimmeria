//! The golden file on disk: pretty JSON under a size cap, so a golden stays
//! reviewable in a diff (README D-GD10).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::event::{RunFingerprint, FINGERPRINT_SCHEMA};
use crate::uat::evidence::path_safe;

/// Bumped when the golden's own shape changes, so an old golden is refused.
pub const GOLDEN_SCHEMA: u32 = 1;
/// The written text, not a compact form, is measured against this cap.
pub const GOLDEN_MAX_BYTES: usize = 128 * 1024;
/// Ordered events in one row. A noisier row needs an `ignore` family.
pub const GOLDEN_MAX_ROW_EVENTS: usize = 1500;

/// One section's agreed fingerprint, as committed under `golden/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenFile {
    pub schema: u32,
    pub section: String,
    pub rows: Vec<String>,
    pub recorded_utc: String,
    pub spec_sha256: String,
    pub build: GoldenBuild,
    /// The agreeing runs (local evidence folders, not committed).
    pub evidence: Vec<GoldenEvidence>,
    pub fingerprint: RunFingerprint,
}

/// The server build the golden was recorded against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenBuild {
    pub server: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sgw_sha256: Option<String>,
}

/// One agreeing run, by the id and folder it had when it was recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenEvidence {
    pub run_id: String,
    pub run_dir: String,
}

/// `<specs_dir>/golden/<path_safe(section)>.json`.
pub fn golden_path(specs_dir: &Path, section: &str) -> PathBuf {
    specs_dir
        .join("golden")
        .join(format!("{}.json", path_safe(section)))
}

/// Pretty JSON plus a trailing newline. Err when a row has more than
/// GOLDEN_MAX_ROW_EVENTS events (naming the row and suggesting an `ignore`
/// family) or the text is over GOLDEN_MAX_BYTES (naming the size).
pub fn to_text(g: &GoldenFile) -> Result<String, String> {
    for row in &g.fingerprint.rows {
        if row.events.len() > GOLDEN_MAX_ROW_EVENTS {
            return Err(format!(
                "row {} has {} ordered events, over the {GOLDEN_MAX_ROW_EVENTS} cap; \
                 add an `ignore` glob for its noisy messages in the spec's [row.golden]",
                row.row,
                row.events.len()
            ));
        }
    }
    let mut text = serde_json::to_string_pretty(g).map_err(|e| format!("encode golden: {e}"))?;
    text.push('\n');
    if text.len() > GOLDEN_MAX_BYTES {
        return Err(format!(
            "golden text is {} bytes, over the {GOLDEN_MAX_BYTES} byte (128 KiB) cap",
            text.len()
        ));
    }
    Ok(text)
}

/// Write the golden, creating its folder. Returns the bytes written.
pub fn save(path: &Path, g: &GoldenFile) -> Result<usize, String> {
    let text = to_text(g)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    std::fs::write(path, &text).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(text.len())
}

/// Read a golden. Both its own schema and the fingerprint's must be the
/// ones this build writes, or the rows would be compared unlike for unlike.
pub fn load(path: &Path) -> Result<GoldenFile, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let g: GoldenFile =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if g.schema != GOLDEN_SCHEMA {
        return Err(format!(
            "{}: golden schema {} is not {GOLDEN_SCHEMA}",
            path.display(),
            g.schema
        ));
    }
    if g.fingerprint.schema != FINGERPRINT_SCHEMA {
        return Err(format!(
            "{}: fingerprint schema {} is not {FINGERPRINT_SCHEMA}",
            path.display(),
            g.fingerprint.schema
        ));
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat::golden::event::{RowFingerprint, Variance};
    use crate::uat::load_sections;

    /// A golden holding `rows`, with the section and build fields filled in.
    fn golden_with(rows: Vec<RowFingerprint>) -> GoldenFile {
        GoldenFile {
            schema: GOLDEN_SCHEMA,
            section: "gm-parity".to_string(),
            rows: rows.iter().map(|r| r.row.clone()).collect(),
            recorded_utc: "2026-10-10T00:00:00Z".to_string(),
            spec_sha256: "00".repeat(32),
            build: GoldenBuild {
                server: "abc1234".to_string(),
                sgw_sha256: None,
            },
            evidence: vec![GoldenEvidence {
                run_id: "run-1".to_string(),
                run_dir: "runs/run-1".to_string(),
            }],
            fingerprint: RunFingerprint {
                schema: FINGERPRINT_SCHEMA,
                section: "gm-parity".to_string(),
                rows,
            },
        }
    }

    /// A row whose ordered stream is `events` copies of `event`.
    fn row_with(id: &str, events: Vec<String>) -> RowFingerprint {
        RowFingerprint {
            row: id.to_string(),
            result: "PASS".to_string(),
            tap: "full".to_string(),
            tap_faults: Vec::new(),
            clauses: Vec::new(),
            transitions: Vec::new(),
            windows_opened: Vec::new(),
            windows_closed: Vec::new(),
            events,
            unordered: Vec::new(),
            variance: Variance::default(),
        }
    }

    /// A golden saved and loaded again is the same value, and the bytes
    /// `save` reports are the bytes on disk.
    #[test]
    fn a_golden_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = golden_path(dir.path(), "gm-parity");
        assert_eq!(path, dir.path().join("golden").join("gm-parity.json"));

        let g = golden_with(vec![row_with(
            "FS-01",
            vec!["c2s interact target=tag:ArmYourself_FrostBody".to_string()],
        )]);
        let bytes = save(&path, &g).unwrap();
        assert_eq!(bytes, std::fs::read(&path).unwrap().len());
        assert_eq!(load(&path).unwrap(), g);
    }

    /// A row of 1 501 events is refused and named, and a file over the
    /// byte cap is refused and sized. A row of exactly 1 500 short events
    /// is within both caps.
    #[test]
    fn an_oversized_golden_is_refused() {
        let too_many = golden_with(vec![row_with(
            "FS-02",
            vec!["s2c onDialogDisplay".to_string(); GOLDEN_MAX_ROW_EVENTS + 1],
        )]);
        let err = to_text(&too_many).unwrap_err();
        assert!(err.contains("FS-02"), "{err}");
        assert!(err.contains("ignore"), "{err}");

        let at_limit = golden_with(vec![row_with(
            "FS-02",
            vec!["s2c onDialogDisplay".to_string(); GOLDEN_MAX_ROW_EVENTS],
        )]);
        assert!(to_text(&at_limit).is_ok());

        let too_big = golden_with(vec![row_with(
            "FS-03",
            vec!["x".repeat(100); GOLDEN_MAX_ROW_EVENTS],
        )]);
        let err = to_text(&too_big).unwrap_err();
        assert!(err.contains("128 KiB"), "{err}");

        let dir = tempfile::tempdir().unwrap();
        let path = golden_path(dir.path(), "gm-parity");
        assert!(save(&path, &too_big).is_err());
        assert!(!path.exists());
    }

    /// A golden whose schema or fingerprint schema is not this build's is
    /// refused by `load`, naming the schema.
    #[test]
    fn a_wrong_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("golden.json");

        let mut newer = golden_with(Vec::new());
        newer.schema = GOLDEN_SCHEMA + 1;
        save(&path, &newer).unwrap();
        let err = load(&path).unwrap_err();
        assert!(err.contains("golden schema"), "{err}");

        let mut stale = golden_with(Vec::new());
        stale.fingerprint.schema = FINGERPRINT_SCHEMA + 1;
        save(&path, &stale).unwrap();
        let err = load(&path).unwrap_err();
        assert!(err.contains("fingerprint schema"), "{err}");
    }

    /// Every committed golden parses, fits the cap, names a section in the
    /// committed specs, and lists only rows that section has. A missing
    /// folder passes: no golden is committed until GD-10.
    #[test]
    fn committed_goldens_parse_and_fit() {
        let specs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
        let dir = specs.join("golden");
        if !dir.is_dir() {
            return;
        }
        let sections = load_sections(&specs, None).unwrap();
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if !path.extension().is_some_and(|x| x == "json") {
                continue;
            }
            let g = load(&path).unwrap_or_else(|e| panic!("{e}"));
            to_text(&g).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let section = sections
                .iter()
                .find(|s| s.spec.section.id == g.section)
                .unwrap_or_else(|| panic!("{}: no spec section {}", path.display(), g.section));
            for row in &g.rows {
                assert!(
                    section.spec.rows.iter().any(|r| &r.id == row),
                    "{}: row {row} is not in section {}",
                    path.display(),
                    g.section
                );
            }
        }
    }
}
