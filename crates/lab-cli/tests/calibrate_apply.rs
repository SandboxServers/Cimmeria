//! LX-12a: the diff target and --apply in a scratch repository created with
//! `git init`. Ports checks [25]-[27] and [33]-[44] of test-calibrate.ps1.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cimmeria_lab_cli::commands::calibrate::apply::{
    apply_diff, diff_file_count, diff_target, spec_dirty,
};
use cimmeria_lab_cli::commands::calibrate::outcome::{apply_error_json, Outcome, APPLY_ERROR_MAX};
use serde_json::{json, Value};
use tempfile::TempDir;

const X_DIFF: &str = "--- a/docs/guides/uat-specs/x.toml\n+++ b/docs/guides/uat-specs/x.toml\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n";

fn git(repo: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

struct Scratch {
    _tmp: TempDir,
    root: PathBuf,
    repo: PathBuf,
    spec: PathBuf,
}

/// A repository with `docs/guides/uat-specs/x.toml` = "a\nb\nc\n", committed.
fn scratch() -> Scratch {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("docs/guides/uat-specs")).unwrap();
    git(&repo, &["init", "-q", "."]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["config", "core.autocrlf", "false"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    let spec = repo.join("docs/guides/uat-specs/x.toml");
    fs::write(&spec, "a\nb\nc\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    Scratch {
        _tmp: tmp,
        root,
        repo,
        spec,
    }
}

/// A diff file outside the checkout (git add -A and reset would take it).
fn diff_file(s: &Scratch, name: &str, text: &str) -> String {
    let p = s.root.join(name);
    fs::write(&p, text).unwrap();
    p.to_string_lossy().into_owned()
}

// [25] 'the diff names its spec file'
#[test]
fn the_diff_names_its_spec_file() {
    assert_eq!(
        diff_target(X_DIFF).as_deref(),
        Some("docs/guides/uat-specs/x.toml")
    );
}
// [26] 'a diff with no +++ line names nothing'
#[test]
fn a_diff_with_no_plus_line_names_nothing() {
    assert_eq!(diff_target("no header"), None);
}
// [27] 'a diff with two +++ lines names nothing'
#[test]
fn a_diff_with_two_plus_lines_names_nothing() {
    let two = format!("{X_DIFF}--- a/docs/guides/uat-specs/other.toml\n+++ b/docs/guides/uat-specs/other.toml\n@@ -1 +1 @@\n-a\n+b\n");
    assert_eq!(diff_target(&two), None);
    assert_eq!(diff_file_count(&two), 2);
}
// [33] 'a spec with local edits is refused and left alone'
#[test]
fn a_spec_with_local_edits_is_refused_and_left_alone() {
    let s = scratch();
    let diff = diff_file(&s, "x.diff", X_DIFF);
    fs::write(&s.spec, "a\nb\nc\nlocal\n").unwrap();
    let r = apply_diff(&s.repo, &diff);
    assert!(!r.ok && r.error.unwrap().contains("uncommitted changes"));
    assert_eq!(fs::read_to_string(&s.spec).unwrap(), "a\nb\nc\nlocal\n");
}
// [34] 'a staged edit is refused too'
#[test]
fn a_staged_edit_is_refused_too() {
    let s = scratch();
    let diff = diff_file(&s, "x.diff", X_DIFF);
    fs::write(&s.spec, "a\nb\nc\nlocal\n").unwrap();
    git(&s.repo, &["add", "-A"]);
    assert!(!apply_diff(&s.repo, &diff).ok);
}
// [35] 'a clean spec gets the diff'
#[test]
fn a_clean_spec_gets_the_diff() {
    let s = scratch();
    let diff = diff_file(&s, "x.diff", X_DIFF);
    let r = apply_diff(&s.repo, &diff);
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(r.target.as_deref(), Some("docs/guides/uat-specs/x.toml"));
    assert_eq!(fs::read_to_string(&s.spec).unwrap(), "a\nB\nc\n");
}
// [36] 'nothing is staged or committed'
#[test]
fn nothing_is_staged_or_committed() {
    let s = scratch();
    let diff = diff_file(&s, "x.diff", X_DIFF);
    let head = git(&s.repo, &["rev-parse", "HEAD"]);
    assert!(apply_diff(&s.repo, &diff).ok);
    assert_eq!(git(&s.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&s.repo, &["diff", "--cached", "--name-only"]), "");
}
// [37] 'a diff outside the spec folder is not applied'
#[test]
fn a_diff_outside_the_spec_folder_is_not_applied() {
    let s = scratch();
    let other = diff_file(
        &s,
        "other.diff",
        "--- a/elsewhere/x.toml\n+++ b/elsewhere/x.toml\n@@ -1,1 +1,1 @@\n-a\n+b\n",
    );
    assert!(apply_diff(&s.repo, &other)
        .error
        .unwrap()
        .contains("not this checkout"));
}
// [38] 'no diff, nothing applied'
#[test]
fn no_diff_nothing_applied() {
    let s = scratch();
    let missing = s.repo.join("missing.diff").to_string_lossy().into_owned();
    let r = apply_diff(&s.repo, &missing);
    assert!(!r.ok && r.error.unwrap().contains("the diff file is missing"));
    assert!(!apply_diff(&s.repo, "").ok);
}
// [39] 'an apply error starts with the diff path'
#[test]
fn an_apply_error_starts_with_the_diff_path() {
    let s = scratch();
    let diff = diff_file(&s, "x.diff", X_DIFF);
    fs::write(&s.spec, "a\nB\nc\nlocal\n").unwrap();
    let r = apply_diff(&s.repo, &diff);
    assert!(r.error.unwrap().starts_with(&format!("{diff}: ")));
}
// [40] 'an untracked spec is dirty, a clean tracked one is not'
#[test]
fn an_untracked_spec_is_dirty_a_clean_tracked_one_is_not() {
    let s = scratch();
    fs::write(s.repo.join("docs/guides/uat-specs/y.toml"), "a\nb\nc\n").unwrap();
    assert!(spec_dirty(&s.repo, "docs/guides/uat-specs/y.toml"));
    assert!(!spec_dirty(&s.repo, "docs/guides/uat-specs/x.toml"));
}
// [41] 'an untracked spec is refused and left alone'
#[test]
fn an_untracked_spec_is_refused_and_left_alone() {
    let s = scratch();
    let y = s.repo.join("docs/guides/uat-specs/y.toml");
    fs::write(&y, "a\nb\nc\n").unwrap();
    let ydiff = diff_file(&s, "y.diff", &X_DIFF.replace("x.toml", "y.toml"));
    let r = apply_diff(&s.repo, &ydiff);
    assert!(!r.ok && r.error.unwrap().contains("not tracked"));
    assert_eq!(fs::read_to_string(&y).unwrap(), "a\nb\nc\n");
}
// [42] 'a two-file diff is refused'
#[test]
fn a_two_file_diff_is_refused() {
    let s = scratch();
    let two = diff_file(&s, "two.diff", "--- a/docs/guides/uat-specs/x.toml\n+++ b/docs/guides/uat-specs/x.toml\n@@ -1,3 +1,3 @@\n-a\n+A\n b\n c\n--- a/docs/guides/uat-specs/z.toml\n+++ b/docs/guides/uat-specs/z.toml\n@@ -1 +1 @@\n-q\n+r\n");
    let r = apply_diff(&s.repo, &two);
    assert!(!r.ok && r.error.unwrap().contains("touches 2 files"));
    assert_eq!(fs::read_to_string(&s.spec).unwrap(), "a\nb\nc\n");
}
// [43] 'a failing git apply is reported and changes nothing'
#[test]
fn a_failing_git_apply_is_reported_and_changes_nothing() {
    let s = scratch();
    let stale = diff_file(&s, "stale.diff", "--- a/docs/guides/uat-specs/x.toml\n+++ b/docs/guides/uat-specs/x.toml\n@@ -1,3 +1,3 @@\n a\n-zzz\n+B\n c\n");
    let r = apply_diff(&s.repo, &stale);
    assert!(
        !r.ok
            && r.error
                .unwrap()
                .starts_with(&format!("{stale}: git apply failed"))
    );
    assert_eq!(fs::read_to_string(&s.spec).unwrap(), "a\nb\nc\n");
}
// [44] 'a failing git apply under -Json keeps run_dir, diff and outcomes'
#[test]
fn a_failing_git_apply_under_json_keeps_run_dir_diff_and_outcomes() {
    let s = scratch();
    let stale = diff_file(&s, "stale.diff", "--- a/docs/guides/uat-specs/x.toml\n+++ b/docs/guides/uat-specs/x.toml\n@@ -1,3 +1,3 @@\n a\n-zzz\n+B\n c\n");
    let r = apply_diff(&s.repo, &stale);
    let outcomes = vec![
        Outcome::from_value(
            &json!({"ok": true, "pose": "frost", "score": 8312, "confirmed": 3, "changed": {"bearing_deg": [322.5, 330.0]}}),
        ),
        Outcome::from_value(&json!({"ok": true, "pose": "supply", "score": 7400, "confirmed": 3})),
    ];
    let root = s.root.to_string_lossy().into_owned();
    let j = apply_error_json(
        &format!("{} (--apply)", r.error.unwrap()),
        &root,
        &stale,
        &outcomes,
        APPLY_ERROR_MAX,
    );
    let v: Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["exit"], 2);
    assert!(v["error"].as_str().unwrap().starts_with(&stale));
    assert_eq!(
        (v["diff"].as_str(), v["run_dir"].as_str()),
        (Some(stale.as_str()), Some(root.as_str()))
    );
    assert_eq!(v["outcomes"].as_array().unwrap().len(), 2);
}

// New: a target that walks out of the spec folder is not applied, though it
// starts with the folder's name.
#[test]
fn a_diff_that_walks_out_of_the_spec_folder_is_not_applied() {
    let s = scratch();
    fs::write(s.repo.join("keep.txt"), "a\nb\nc\n").unwrap();
    git(&s.repo, &["add", "keep.txt"]);
    git(&s.repo, &["commit", "-q", "-m", "keep"]);
    let up = diff_file(
        &s,
        "up.diff",
        "--- a/docs/guides/uat-specs/../../../keep.txt\n+++ b/docs/guides/uat-specs/../../../keep.txt\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n",
    );
    let r = apply_diff(&s.repo, &up);
    assert!(
        !r.ok && r.error.unwrap().contains("not this checkout"),
        "refused as outside the spec folder"
    );
    assert_eq!(
        fs::read_to_string(s.repo.join("keep.txt")).unwrap(),
        "a\nb\nc\n"
    );
}

// New: a second file with no `+++` line (a mode change) still counts as a file.
#[test]
fn a_second_file_without_a_plus_line_is_counted() {
    let two = format!(
        "diff --git a/docs/guides/uat-specs/x.toml b/docs/guides/uat-specs/x.toml\n{X_DIFF}diff --git a/docs/guides/uat-specs/z.toml b/docs/guides/uat-specs/z.toml\nold mode 100644\nnew mode 100755\n"
    );
    assert_eq!(diff_file_count(&two), 2);
    assert_eq!(diff_target(&two), None);
    let s = scratch();
    let diff = diff_file(&s, "mode.diff", &two);
    let r = apply_diff(&s.repo, &diff);
    assert!(!r.ok && r.error.unwrap().contains("touches 2 files"));
    assert_eq!(fs::read_to_string(&s.spec).unwrap(), "a\nb\nc\n");
}
