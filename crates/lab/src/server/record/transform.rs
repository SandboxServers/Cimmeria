//! `lab_record_transform` (lab-record LR-08): turn a recording folder that
//! is already on disk into a draft again, with other options, without a
//! client or a lease. It shares the stop's log read and draft writer
//! ([`super::stop::write_draft`]), so the draft lands in the same place, is
//! never written over an existing file (a re-transform takes `<name>-2.toml`
//! and on), and the answer has the stop's shape. A log without its stop line
//! is refused: stop the recording first.
//!
//! The folder must be a recording folder: a direct child of
//! `<uat root>/records/`. A bare recording id names one; a path is
//! canonicalized and must land there, so `..`, another absolute path, a
//! UNC share or a `\\?\` path is refused before anything is read.

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use super::checks::{check_character_expr, check_section, check_specs_dir};
use super::run::{refused, Failure};
use super::stop::{failed_at, read_raw, write_draft, DraftOpts, TransformFn};

/// What `lab_record_transform` asks for.
pub(super) struct TransformReq {
    /// A recording id, or a path to its folder.
    pub dir: String,
    /// Replaces the recorded section (the draft's name and its base).
    pub section: Option<String>,
    pub character_expr: Option<String>,
    pub profile: Option<String>,
    /// Where `drafts/` goes, as the caller gave it (checked like start's).
    pub specs_dir: Option<PathBuf>,
    /// Used when the caller gave none: the daemon's spec directory, which an
    /// installed daemon (its working directory is the lab home) may lack.
    pub default_specs: Option<PathBuf>,
    pub transform: TransformFn,
}

/// Check the arguments, find the folder under `records`, and write its
/// draft.
pub(super) fn transform_dir(records: &Path, req: TransformReq) -> Result<Value, Failure> {
    if let Some(s) = req.section.as_deref() {
        check_section(s)?;
    }
    check_character_expr(req.character_expr.as_deref())?;
    let specs = check_specs_dir(req.specs_dir)?.or(req.default_specs);
    let raw = recording_folder(records, &req.dir)?;
    let mut log = read_raw(&raw)?;
    // No stop line: still recording, or a recorder that died mid-write. A
    // draft now would be partial, and its name would block that
    // recording's own stop (`create_new`).
    if log.stop.is_none() {
        return Err(failed_at(
            &raw,
            "recording has no stop line; stop it first".into(),
        ));
    }
    if let Some(s) = req.section {
        log.start.section = s;
    }
    let out = write_draft(
        &raw,
        &log,
        DraftOpts {
            specs: specs.as_deref(),
            character_expr: req.character_expr,
            profile: req.profile,
            transform: req.transform,
            // A re-transform never refuses for an existing draft: it gets
            // `<name>-2.toml`, `-3`, ...
            suffix_on_exists: true,
        },
    );
    match &out {
        Ok(v) => tracing::info!(target: "lab.record", raw = %raw.display(),
            draft = %v["draft"], "recording transformed again"),
        Err(f) => tracing::warn!(target: "lab.record", raw = %raw.display(),
            error = %f.error, "recording transform failed"),
    }
    out
}

/// `dir` as a canonical recording folder: a direct child of `records`.
fn recording_folder(records: &Path, dir: &str) -> Result<PathBuf, Failure> {
    let refuse = |why: &str| {
        refused(format!(
            "dir: {why}; give a recording id or its folder under <uat root>/records/"
        ))
    };
    if dir.is_empty() {
        return Err(refuse("empty"));
    }
    if dir.starts_with(r"\\?\") || dir.starts_with("//?/") {
        return Err(refuse("a verbatim path (\\\\?\\) is refused"));
    }
    if dir.starts_with(r"\\") || dir.starts_with("//") {
        return Err(refuse("a UNC path is refused"));
    }
    let given = Path::new(dir);
    if given.components().any(|c| c == Component::ParentDir) {
        return Err(refuse("`..` is refused"));
    }
    let root = records
        .canonicalize()
        .map_err(|e| refused(format!("no recordings yet ({}: {e})", records.display())))?;
    // A bare id or relative path is read under the records root.
    let candidate = if given.is_absolute() || given.has_root() {
        given.to_path_buf()
    } else {
        records.join(given)
    };
    let canon = candidate
        .canonicalize()
        .map_err(|_| refuse("no such recording folder"))?;
    if canon.parent() != Some(root.as_path()) || !canon.is_dir() {
        return Err(refuse("not a recording folder"));
    }
    Ok(canon)
}

#[cfg(test)]
mod tests {
    use super::super::run::RECORD_FILE;
    use super::*;
    use crate::uat::record::emit::transform;

    const FIXTURE: &str = include_str!("../../uat/record/fixtures/fs_p3_record.jsonl");

    struct Roots {
        _tmp: tempfile::TempDir,
        records: PathBuf,
        specs: PathBuf,
        outside: PathBuf,
    }

    /// `records/<id>/record.jsonl`, an empty spec directory, and a copy of
    /// the same log outside the records root.
    fn roots(id: &str) -> Roots {
        let tmp = tempfile::tempdir().unwrap();
        let records = tmp.path().join("records");
        let specs = tmp.path().join("specs");
        let outside = tmp.path().join("elsewhere").join(id);
        for d in [records.join(id), specs.clone(), outside.clone()] {
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(RECORD_FILE), FIXTURE).unwrap();
        }
        Roots {
            _tmp: tmp,
            records,
            specs,
            outside,
        }
    }

    fn req(r: &Roots, dir: &str) -> TransformReq {
        TransformReq {
            dir: dir.into(),
            section: None,
            character_expr: None,
            profile: None,
            specs_dir: Some(r.specs.clone()),
            default_specs: None,
            transform,
        }
    }

    fn drafts(r: &Roots) -> usize {
        std::fs::read_dir(r.specs.join("drafts"))
            .map(|d| d.count())
            .unwrap_or(0)
    }

    /// Regression guard: nothing outside `<uat root>/records/` is read, by
    /// an absolute path, `..`, the root itself, UNC or verbatim.
    #[test]
    fn a_dir_outside_the_records_root_is_refused() {
        let r = roots("20261012-143000-p2");
        let outside = r.outside.display().to_string();
        let up = r
            .records
            .join("..")
            .join("elsewhere")
            .join("20261012-143000-p2");
        for dir in [
            outside.as_str(),
            &up.display().to_string(),
            "../elsewhere/20261012-143000-p2",
            &r.records.display().to_string(),
            r"\\server\share\records\x",
            r"\\?\C:\records\x",
            "",
            "no-such-recording",
        ] {
            let f = transform_dir(&r.records, req(&r, dir)).unwrap_err();
            assert_eq!(f.exit, 2, "{dir}: {}", f.error);
            assert!(f.error.starts_with("dir: "), "{dir}: {}", f.error);
        }
        assert_eq!(drafts(&r), 0, "a refused call writes nothing");
    }

    /// A recording id, or its full folder path, writes a draft under the
    /// spec directory's `drafts/` and answers like the stop; a second and
    /// third call take `-2` and `-3`, never overwriting the first.
    #[test]
    fn a_good_dir_writes_a_draft() {
        let id = "20261012-143000-p2";
        let r = roots(id);
        let v = transform_dir(&r.records, req(&r, id)).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["raw"], id);
        let name = v["draft"].as_str().unwrap().to_string();
        let first = r.specs.join("drafts").join(&name);
        assert!(first.is_file(), "{v}");
        assert!(v["rows"].as_u64().is_some() && v["todo"].as_u64().is_some());
        assert!(v.to_string().len() < 400);
        // Marked, so an overwrite would show.
        std::fs::write(&first, "# first").unwrap();

        // The same folder by full path: the draft exists, so it gets `-2`.
        let stem = name.strip_suffix(".toml").unwrap();
        let full = r.records.join(id).display().to_string();
        let v = transform_dir(&r.records, req(&r, &full)).unwrap();
        assert_eq!(v["draft"], format!("{stem}-2.toml"), "{v}");
        let v = transform_dir(&r.records, req(&r, id)).unwrap();
        assert_eq!(v["draft"], format!("{stem}-3.toml"), "{v}");
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "# first");
        assert_eq!(drafts(&r), 3);

        // Another section names another draft.
        let v = transform_dir(
            &r.records,
            TransformReq {
                section: Some("other-section".into()),
                ..req(&r, &full)
            },
        )
        .unwrap();
        assert!(
            v["draft"].as_str().unwrap().starts_with("other-section-"),
            "{v}"
        );
        assert_eq!(drafts(&r), 4);
    }

    /// Regression guard: a log without its stop line (a recording still
    /// running, or a dead recorder's) is refused and no draft is written,
    /// so it cannot take the name the recording's own stop will need.
    #[test]
    fn a_recording_without_a_stop_line_is_refused() {
        let id = "20261012-143000-p2";
        let r = roots(id);
        let unstopped: String = FIXTURE
            .lines()
            .filter(|l| !l.starts_with(r#"{"kind":"stop""#))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_ne!(
            unstopped.len(),
            FIXTURE.len(),
            "the fixture has a stop line"
        );
        std::fs::write(r.records.join(id).join(RECORD_FILE), unstopped).unwrap();
        let f = transform_dir(&r.records, req(&r, id)).unwrap_err();
        assert_eq!(f.exit, 1, "{}", f.error);
        assert!(f.error.contains("no stop line"), "{}", f.error);
        assert_eq!(drafts(&r), 0);
    }

    /// A link inside `records/` that leads outside it is refused: the
    /// folder is checked after canonicalizing, so the link is followed.
    #[test]
    fn a_link_inside_records_to_outside_is_refused() {
        let r = roots("20261012-143000-p2");
        let link = r.records.join("20261012-150000-p2");
        #[cfg(windows)]
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&r.outside)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&r.outside, &link).is_ok();
        assert!(made, "could not make the link");
        let f = transform_dir(&r.records, req(&r, "20261012-150000-p2")).unwrap_err();
        assert_eq!(f.exit, 2, "{}", f.error);
        assert!(f.error.contains("not a recording folder"), "{}", f.error);
        assert_eq!(drafts(&r), 0);
    }

    #[test]
    fn bad_section_or_character_expr_is_refused_first() {
        let r = roots("20261012-143000-p2");
        let f = transform_dir(
            &r.records,
            TransformReq {
                section: Some("../x".into()),
                ..req(&r, "20261012-143000-p2")
            },
        )
        .unwrap_err();
        assert!(f.error.starts_with("section:"), "{}", f.error);
        let f = transform_dir(
            &r.records,
            TransformReq {
                character_expr: Some("x'; --".into()),
                ..req(&r, "20261012-143000-p2")
            },
        )
        .unwrap_err();
        assert!(f.error.starts_with("character_expr:"), "{}", f.error);
        let f = transform_dir(
            &r.records,
            TransformReq {
                specs_dir: Some(PathBuf::from(r"\\server\share\specs")),
                ..req(&r, "20261012-143000-p2")
            },
        )
        .unwrap_err();
        assert!(f.error.starts_with("specs_dir:"), "{}", f.error);
        assert_eq!(drafts(&r), 0);
    }

    /// Without a `specs_dir` the daemon's own spec directory is used.
    #[test]
    fn no_specs_dir_falls_back_to_the_daemons() {
        let r = roots("20261012-143000-p2");
        let v = transform_dir(
            &r.records,
            TransformReq {
                specs_dir: None,
                default_specs: Some(r.specs.clone()),
                ..req(&r, "20261012-143000-p2")
            },
        )
        .unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(drafts(&r), 1);
    }
}
