//! `--apply`: `git apply` the proposed diff in this checkout, unless its spec
//! has local edits or is not tracked. Never stages, never commits. Every
//! error starts with the diff's path, so the proposal is never lost with the
//! failure.

use std::path::Path;
use std::process::{Command, Stdio};

/// The spec folder, from the repo root: a diff for any other path is not this
/// checkout's to apply.
pub const SPEC_PREFIX: &str = "docs/guides/uat-specs/";

/// What [`apply_diff`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub ok: bool,
    /// Set when `!ok`; starts with the diff path (`<diff>: <why>`).
    pub error: Option<String>,
    /// The repo-relative file the diff names, when it was read far enough.
    pub target: Option<String>,
}

impl ApplyResult {
    /// A refusal: `<diff_path>: <message>` (just the message with no path).
    pub fn refused(diff_path: &str, message: &str, target: Option<String>) -> ApplyResult {
        let error = if diff_path.is_empty() {
            message.to_string()
        } else {
            format!("{diff_path}: {message}")
        };
        ApplyResult {
            ok: false,
            error: Some(error),
            target,
        }
    }
}

/// The number of `+++ ` file headers in a diff.
pub fn diff_file_count(text: &str) -> usize {
    text.split('\n')
        .filter(|l| l.trim_end_matches('\r').starts_with("+++ "))
        .count()
}

/// The repo-relative file a diff applies to (its `+++ b/<path>` line), or
/// `None` when it names none or more than one: `lab_uat_calibrate` proposes
/// for one spec, so a second file is not its diff.
pub fn diff_target(text: &str) -> Option<String> {
    if diff_file_count(text) != 1 {
        return None;
    }
    text.split('\n').find_map(|l| {
        l.trim_end_matches('\r')
            .strip_prefix("+++ b/")
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
    })
}

/// Runs git in `repo`; `Some(true)` when it exits 0, `None` when it cannot run.
fn git_ok(repo: &Path, args: &[&str]) -> Option<bool> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map(|s| s.success())
}

/// Whether a repo-relative file has local edits, staged or not (`git diff
/// HEAD`), or is not tracked at all (`git diff` cannot see an untracked file,
/// and `git apply` over it would leave nothing to revert to): `true` then,
/// and when git cannot tell.
pub fn spec_dirty(repo: &Path, rel: &str) -> bool {
    if git_ok(repo, &["ls-files", "--error-unmatch", "--", rel]) != Some(true) {
        return true;
    }
    git_ok(repo, &["diff", "--quiet", "HEAD", "--", rel]) != Some(true)
}

/// `git apply` the diff file in `repo`. The refusals, in order (each is
/// `<diff>: <message>`):
///
/// 1. `no diff was proposed; nothing to apply` (empty path)
/// 2. `the diff file is missing; nothing to apply`
/// 3. `the diff touches <n> files, not one spec; apply it by hand` (n > 1)
/// 4. `the diff names no file`
/// 5. `the diff is for <target>, not this checkout's docs/guides/uat-specs/; apply it by hand`
/// 6. `<target> is not in <repo>`
/// 7. `<target> has uncommitted changes or is not tracked; commit or revert them, then: git -C "<repo>" apply "<diff>"`
/// 8. `git apply failed: <first line of git's error>`
pub fn apply_diff(repo: &Path, diff_path: &str) -> ApplyResult {
    if diff_path.is_empty() {
        return ApplyResult::refused(diff_path, "no diff was proposed; nothing to apply", None);
    }
    if !Path::new(diff_path).exists() {
        return ApplyResult::refused(
            diff_path,
            "the diff file is missing; nothing to apply",
            None,
        );
    }
    let text = match std::fs::read_to_string(diff_path) {
        Ok(t) => t,
        Err(e) => {
            return ApplyResult::refused(diff_path, &format!("cannot read the diff: {e}"), None)
        }
    };
    let files = diff_file_count(&text);
    if files > 1 {
        let m = format!("the diff touches {files} files, not one spec; apply it by hand");
        return ApplyResult::refused(diff_path, &m, None);
    }
    let Some(target) = diff_target(&text) else {
        return ApplyResult::refused(diff_path, "the diff names no file", None);
    };
    if !target.starts_with(SPEC_PREFIX) {
        let m = format!(
            "the diff is for {target}, not this checkout's {SPEC_PREFIX}; apply it by hand"
        );
        return ApplyResult::refused(diff_path, &m, Some(target));
    }
    if !repo.join(&target).exists() {
        let m = format!("{target} is not in {}", repo.display());
        return ApplyResult::refused(diff_path, &m, Some(target));
    }
    if spec_dirty(repo, &target) {
        let m = format!(
            "{target} has uncommitted changes or is not tracked; commit or revert them, then: git -C \"{}\" apply \"{diff_path}\"",
            repo.display()
        );
        return ApplyResult::refused(diff_path, &m, Some(target));
    }
    let output = match Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("apply")
        .arg(diff_path)
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let m = format!("git apply failed: {e}");
            return ApplyResult::refused(diff_path, &m, Some(target));
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first = stderr
            .lines()
            .chain(stdout.lines())
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("no message");
        let m = format!("git apply failed: {first}");
        return ApplyResult::refused(diff_path, &m, Some(target));
    }
    ApplyResult {
        ok: true,
        error: None,
        target: Some(target),
    }
}
