//! `lab_record_transform` (lab-record LR-08): turn a recording folder that
//! is already on disk into a draft again, with other options, without a
//! client or a lease. It shares the stop's log read and draft writer
//! ([`super::stop::write_draft`]), so the draft lands in the same place, is
//! never written over an existing file, and the answer has the stop's shape.
//!
//! The folder must be a recording folder: a direct child of
//! `<uat root>/records/`. A bare recording id names one; a path is
//! canonicalized and must land there, so `..`, another absolute path, a
//! UNC share or a `\\?\` path is refused before anything is read.

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use super::checks::{check_character_expr, check_section, check_specs_dir};
use super::run::{refused, Failure};
use super::stop::{read_raw, write_draft, DraftOpts, TransformFn};

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
    /// spec directory's `drafts/` and answers like the stop; a second call
    /// never overwrites it.
    #[test]
    fn a_good_dir_writes_a_draft() {
        let id = "20261012-143000-p2";
        let r = roots(id);
        let v = transform_dir(&r.records, req(&r, id)).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["raw"], id);
        let name = v["draft"].as_str().unwrap();
        assert!(r.specs.join("drafts").join(name).is_file(), "{v}");
        assert!(v["rows"].as_u64().is_some() && v["todo"].as_u64().is_some());
        assert!(v.to_string().len() < 400);

        // The same folder by full path: the draft exists, so it is refused.
        let full = r.records.join(id).display().to_string();
        let f = transform_dir(&r.records, req(&r, &full)).unwrap_err();
        assert_eq!(f.exit, 1);
        assert!(f.error.contains("exists, not overwritten"), "{}", f.error);

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
        assert_eq!(drafts(&r), 2);
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
