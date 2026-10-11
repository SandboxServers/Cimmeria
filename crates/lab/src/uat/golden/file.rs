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

/// Write the golden, creating its folder. Returns the bytes written. The
/// text goes to a temporary file first and is renamed over the target, so a
/// failed write never destroys the previous golden.
pub fn save(path: &Path, g: &GoldenFile) -> Result<usize, String> {
    check_consistent(g)?;
    let text = to_text(g)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("replace {}: {e}", path.display())
    })?;
    Ok(text.len())
}

/// Read a golden. Both its own schema and the fingerprint's must be the
/// ones this build writes, or the rows would be compared unlike for unlike.
/// The schemas are checked before the shape, so a golden from another
/// schema says so instead of failing on a missing field.
pub fn load(path: &Path) -> Result<GoldenFile, String> {
    let at = path.display();
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {at}: {e}"))?;
    let raw: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{at}: {e}"))?;
    let schema = raw.get("schema").and_then(|v| v.as_u64());
    if schema != Some(u64::from(GOLDEN_SCHEMA)) {
        return Err(format!(
            "{at}: golden schema {} is not {GOLDEN_SCHEMA}",
            schema.map_or("missing".to_string(), |s| s.to_string())
        ));
    }
    let fp_schema = raw.pointer("/fingerprint/schema").and_then(|v| v.as_u64());
    if fp_schema != Some(u64::from(FINGERPRINT_SCHEMA)) {
        return Err(format!(
            "{at}: fingerprint schema {} is not {FINGERPRINT_SCHEMA}",
            fp_schema.map_or("missing".to_string(), |s| s.to_string())
        ));
    }
    let g: GoldenFile = serde_json::from_value(raw).map_err(|e| format!("{at}: {e}"))?;
    check_consistent(&g).map_err(|e| format!("{at}: {e}"))?;
    Ok(g)
}

/// The header must describe the fingerprint it carries: same section, and
/// `rows` exactly the fingerprint's row ids in order. A hand-edited golden
/// that disagrees would be diffed against rows nobody recorded.
fn check_consistent(g: &GoldenFile) -> Result<(), String> {
    if g.fingerprint.section != g.section {
        return Err(format!(
            "section {} but the fingerprint is for {}",
            g.section, g.fingerprint.section
        ));
    }
    let fp_rows: Vec<&str> = g.fingerprint.rows.iter().map(|r| r.row.as_str()).collect();
    if fp_rows != g.rows.iter().map(String::as_str).collect::<Vec<_>>() {
        return Err(format!(
            "rows {:?} but the fingerprint holds {fp_rows:?}",
            g.rows
        ));
    }
    Ok(())
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

        let mut row = row_with(
            "FS-01",
            vec!["c2s interact target=tag:ArmYourself_FrostBody".to_string()],
        );
        // The resolved variance is stored per row (D-GD5), so it must
        // survive the trip too.
        row.variance.ignore = vec!["avatarUpdate*".to_string()];
        row.variance.allow_unordered = vec!["onEffectResults".to_string()];
        row.variance
            .key_args
            .insert("onSequence".to_string(), vec!["target_id".to_string()]);
        row.variance.late_bind_settle_ms = 750;
        let g = golden_with(vec![row]);
        let bytes = save(&path, &g).unwrap();
        assert!(!path.with_extension("json.tmp").exists());
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

    /// Every problem with the goldens in `specs_dir/golden/`, given each
    /// committed section's id and row ids. Empty when all are good.
    fn golden_problems(specs_dir: &Path, sections: &[(String, Vec<String>)]) -> Vec<String> {
        let dir = specs_dir.join("golden");
        let mut problems = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let is_json = path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("json"));
            if !is_json {
                continue;
            }
            let at = path.display();
            // The committed bytes, which autocrlf or hand edits can grow.
            let on_disk = std::fs::metadata(&path).unwrap().len();
            if on_disk > GOLDEN_MAX_BYTES as u64 {
                problems.push(format!("{at}: {on_disk} bytes on disk, over the cap"));
            }
            let g = match load(&path) {
                Ok(g) => g,
                Err(e) => {
                    problems.push(e);
                    continue;
                }
            };
            if let Err(e) = to_text(&g) {
                problems.push(format!("{at}: {e}"));
            }
            if path != golden_path(specs_dir, &g.section) {
                problems.push(format!(
                    "{at}: section {} belongs in another file",
                    g.section
                ));
            }
            match sections.iter().find(|(id, _)| *id == g.section) {
                None => problems.push(format!("{at}: no spec section {}", g.section)),
                Some((_, rows)) => {
                    for row in g.rows.iter().filter(|r| !rows.contains(r)) {
                        problems.push(format!("{at}: row {row} is not in section {}", g.section));
                    }
                }
            }
        }
        problems
    }

    /// Every committed golden parses, fits the cap, sits at its section's
    /// path, names a section in the committed specs, and lists only rows
    /// that section has. A missing folder passes: no golden is committed
    /// until GD-10.
    #[test]
    fn committed_goldens_parse_and_fit() {
        let specs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
        if !specs.join("golden").is_dir() {
            return;
        }
        let sections: Vec<(String, Vec<String>)> = load_sections(&specs, None)
            .unwrap()
            .iter()
            .map(|s| {
                let rows = s.spec.rows.iter().map(|r| r.id.clone()).collect();
                (s.spec.section.id.clone(), rows)
            })
            .collect();
        let problems = golden_problems(&specs, &sections);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// The guard catches each way a committed golden can be wrong, so the
    /// committed-golden test is not vacuous while the folder is empty.
    #[test]
    fn the_golden_guard_reports_bad_files() {
        let dir = tempfile::tempdir().unwrap();
        let sections = vec![("gm-parity".to_string(), vec!["FS-01".to_string()])];
        let good = golden_with(vec![row_with("FS-01", Vec::new())]);
        save(&golden_path(dir.path(), "gm-parity"), &good).unwrap();
        assert!(golden_problems(dir.path(), &sections).is_empty());

        // A misnamed file, a row the spec lacks, and a tampered header.
        save(&dir.path().join("golden").join("other.json"), &good).unwrap();
        let mut ghost_row = golden_with(vec![row_with("FS-99", Vec::new())]);
        ghost_row.section = "gm-parity".to_string();
        std::fs::write(
            golden_path(dir.path(), "gm-parity"),
            serde_json::to_string(&ghost_row).unwrap(),
        )
        .unwrap();
        let mut tampered = good.clone();
        tampered.rows = vec!["FS-02".to_string()];
        std::fs::write(
            dir.path().join("golden").join("tampered.JSON"),
            serde_json::to_string(&tampered).unwrap(),
        )
        .unwrap();

        let problems = golden_problems(dir.path(), &sections).join("\n");
        assert!(problems.contains("belongs in another file"), "{problems}");
        assert!(
            problems.contains("row FS-99 is not in section"),
            "{problems}"
        );
        assert!(problems.contains("but the fingerprint holds"), "{problems}");
    }
}
