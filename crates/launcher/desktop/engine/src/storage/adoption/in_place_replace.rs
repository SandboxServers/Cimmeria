//! The in-place replace and restore steps: a release file that needs replacing has
//! its original set aside first, and an abandoned adoption puts each one back. Split
//! out of `in_place.rs`; the moves that call these stay there.
use super::in_place::{holds, present, Hashed, Way};
use super::model::FileCheck;
use super::publication::Point;
use super::*;
use cimmeria_launcher_core::install_layout::bundled_cooked_alias;
use std::collections::BTreeSet;

/// The stock cooked-data tier a light adoption places (LA-11b), as folder-relative
/// `(stock, placed)` directories in the folder's spelling. `None` unless the plan is
/// light and one of its rows renames a stock-spelled file to its placed path.
pub(super) fn cooked_tier(plan: &Plan) -> Option<(String, String)> {
    if !matches!(plan.provenance.check, FileCheck::Light) {
        return None;
    }
    plan.report.files.iter().find_map(|d| {
        let source = d.source_path.as_deref()?;
        (bundled_cooked_alias(source).is_some() && !d.path.eq_ignore_ascii_case(source)).then(
            || {
                (
                    source[..27].to_string(),
                    format!("{}SourceCache.en-us", &source[..16]),
                )
            },
        )
    })
}

/// Whether `path` is a plain directory. Missing is `false`; a link, junction or file
/// is refused.
fn plain_dir(path: &Path) -> Result<bool, Error> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if super::super::failed_cleanup::plain(&meta) && meta.is_dir() => Ok(true),
        Ok(_) => Err(StorageError::UnsafeFile.into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(StorageError::Io.into()),
    }
}

/// Before any file moves (adopt, and abandon too, so that a run that stopped before
/// the rename can move files back): rename the stock tier to the placed one.
/// Idempotent.
pub(super) fn place_cooked_tier(way: Way<'_>, from: &Path, to: &Path) -> Result<(), Error> {
    let (plan, root) = match way {
        Way::Adopt { plan, .. } => (plan, from),
        Way::Abandon { plan } => (plan, to),
    };
    let Some((stock, placed)) = cooked_tier(plan) else {
        return Ok(());
    };
    let (stock, placed) = (
        inventory::native(root, &stock),
        inventory::native(root, &placed),
    );
    // Already renamed: a re-run, a recovery, or the second call.
    if !plain_dir(&stock)? {
        return Ok(());
    }
    if plain_dir(&placed)? && std::fs::remove_dir(&placed).is_err() {
        // Both tiers hold files (a stored-move row looks like a rename row): nothing to place.
        tracing::debug!(target: "adoption", "light adoption: both cooked-data tiers hold files; the directory stays as it is");
        return Ok(());
    }
    std::fs::rename(&stock, &placed)?;
    tracing::debug!(target: "adoption", "light adoption: placed the bundled cooked-data tier");
    Ok(())
}

/// After abandon moved the files back: rename the placed tier back to the stock
/// spelling.
pub(super) fn restore_cooked_tier(way: Way<'_>, to: &Path) -> Result<(), Error> {
    let Way::Abandon { plan } = way else {
        return Ok(());
    };
    let Some((stock, placed)) = cooked_tier(plan) else {
        return Ok(());
    };
    let (stock, placed) = (
        inventory::native(to, &stock),
        inventory::native(to, &placed),
    );
    match (plain_dir(&placed)?, plain_dir(&stock)?) {
        (true, false) => std::fs::rename(&placed, &stock)?,
        (true, true) => {
            tracing::warn!(target: "adoption", "abandoning light adoption: both cooked-data tiers are in the folder; the placed one stays")
        }
        _ => (),
    }
    Ok(())
}

/// The login file the launcher writes its server list into (D-AI13).
const LOGIN_INTERNAL: &str = "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua";

/// The kind of a replaced release file, which says where its original goes. A
/// file that verifies needs no replacement, except a `LoginInternal.lua` the
/// launcher rewrote, whose original is backed up (D-AI13). `None` renames as usual.
pub(super) fn replaced_kind(path: &str, reason: Classification) -> Option<Classification> {
    match reason {
        Classification::Matched | Classification::KnownTransform | Classification::Extra => {
            (path == LOGIN_INTERNAL).then_some(Classification::Modified)
        }
        kind => Some(kind),
    }
}

/// Where a replaced file's original is kept: a modified file's original goes to
/// the backup (D-AI1), one that needs patching to the work folder's `originals`.
/// A missing file has no original.
fn original_home(plan: &Plan, path: &str, kind: Classification) -> Result<Option<PathBuf>, Error> {
    Ok(match kind {
        Classification::Modified => {
            let backup = plan.report.backup.as_ref().ok_or(StorageError::Corrupt)?;
            Some(inventory::native(backup, path))
        }
        Classification::NeedsPatching => {
            Some(inventory::native(&plan.stage().join("originals"), path))
        }
        _ => None,
    })
}

/// Installs the replacement for one release file: the original is set aside
/// first, then the staged replacement is copied into `game`. Idempotent: an
/// original already set aside is not moved again, and a `game` file already
/// holding the replacement is done.
pub(super) fn replace_file(
    path: &str,
    source: &Path,
    target: &Path,
    plan: &Plan,
    replacement: &Replacement,
    kind: Classification,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    if let Some(home) = original_home(plan, path, kind)? {
        match (present(source)?, present(&home)?) {
            (true, false) => {
                if let Some(parent) = home.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::rename(source, &home)?;
                tracing::debug!(target: "adoption", path, original = %home.display(), "in-place adoption: the original was set aside");
                hook(Point::Replaced)?;
            }
            (false, false) if kind == Classification::Modified => {
                tracing::error!(target: "adoption", path, "in-place adoption: a modified release file is in neither place");
                return Err(StorageError::Corrupt.into());
            }
            // Already set aside; a file at the old path now is one the release does not list.
            _ => (),
        }
    }
    let expected = Hashed {
        sha256: replacement.sha256,
        size: replacement.size,
    };
    if present(target)? {
        if holds(target, &expected)? {
            return Ok(());
        }
        // A partial copy from an interrupted run. `game` is this launcher's own
        // tree, so the partial file is replaced rather than kept.
        std::fs::remove_file(target)?;
    }
    let staged = inventory::native(&plan.stage().join("replacements"), path);
    let entry = inventory::hash(&staged, &CancellationToken::new())?;
    if entry.sha256 != replacement.sha256 || entry.size != replacement.size {
        tracing::error!(target: "adoption", path, "in-place adoption: a staged replacement no longer matches the review");
        return Err(StorageError::Corrupt.into());
    }
    // The copy lands beside the target and is renamed over it only once it verifies,
    // so an interrupted copy never leaves a partial file at the release path.
    let partial = partial_path(target);
    if present(&partial)? {
        std::fs::remove_file(&partial)?;
    }
    inventory::copy(&staged, &partial, &entry, &CancellationToken::new())?;
    if !holds(&partial, &expected)? {
        std::fs::remove_file(&partial)?;
        return Err(StorageError::Corrupt.into());
    }
    std::fs::rename(&partial, target)?;
    Ok(())
}

/// The name a replacement is copied to before it is renamed over `target`.
fn partial_path(target: &Path) -> PathBuf {
    let mut partial = target.as_os_str().to_owned();
    partial.push(".adopt-partial");
    PathBuf::from(partial)
}

/// Undoes [`replace_file`] while the adoption is staged: the original goes back
/// from where it was set aside, and the replacement copy leaves `game` only if it
/// still holds the reviewed bytes. A file in both places keeps the folder's copy
/// and the set-aside original stays where it is.
pub(super) fn restore_replaced(
    path: &str,
    game_copy: &Path,
    root_copy: &Path,
    plan: &Plan,
    replacement: &Replacement,
    kind: Classification,
) -> Result<(), Error> {
    if let Some(home) = original_home(plan, path, kind)? {
        match (present(&home)?, present(root_copy)?) {
            (true, false) => {
                if let Some(parent) = root_copy.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::rename(&home, root_copy)?;
            }
            (true, true) => {
                tracing::warn!(target: "adoption", path, "abandoning in-place adoption: the original is in both the backup and its old path; the folder's copy stays")
            }
            (false, false) if kind == Classification::Modified => {
                tracing::error!(target: "adoption", path, "abandoning in-place adoption: a modified release file is in neither place");
                return Err(StorageError::Corrupt.into());
            }
            _ => (),
        }
    }
    let expected = Hashed {
        sha256: replacement.sha256,
        size: replacement.size,
    };
    if present(game_copy)? && holds(game_copy, &expected)? {
        std::fs::remove_file(game_copy)?;
    }
    // A copy an interrupted install left beside the release path is never the reviewed file.
    let partial = partial_path(game_copy);
    if present(&partial)? {
        std::fs::remove_file(&partial)?;
    }
    Ok(())
}

/// Abandon's last folder steps, once the originals are back: the directories the
/// restored originals emptied go, then the backup folder, and so does the work
/// folder with its replacement copies. A folder still holding a kept original
/// stays, and is logged.
pub(super) fn clear_abandoned(plan: &Plan, files: &[&str]) -> Result<(), Error> {
    let originals = plan.stage().join("originals");
    if let Some(backup) = &plan.report.backup {
        remove_emptied(backup, files)?;
        remove_empty_dir(backup);
    }
    remove_emptied(&originals, files)?;
    let stage = plan.stage();
    match std::fs::remove_dir_all(stage.join("replacements")) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => {
            tracing::warn!(target: "adoption", ?error, "abandoned in-place adoption: could not remove the staged replacements")
        }
    }
    remove_empty_dir(&originals);
    remove_empty_dir(&stage);
    Ok(())
}

/// Removes `dir` if it is empty. A folder that is missing or still holds a file
/// is left alone; a kept file is the reason it stays.
fn remove_empty_dir(dir: &Path) {
    match std::fs::remove_dir(dir) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => {
            tracing::warn!(target: "adoption", ?error, directory = %dir.display(), "abandoned in-place adoption: a folder is left in place")
        }
    }
}

/// Whether `path` is a custom `LoginInternal.lua` that setup rewrites. Its original
/// is kept in the backup at the same relative path (D-AI13).
pub(super) fn keeps_login(plan: &Plan, path: &str) -> bool {
    path.eq_ignore_ascii_case(LOGIN_INTERNAL)
        && plan.report.login_replaced
        && plan.report.backup.is_some()
}

/// Copies the original `LoginInternal.lua` into the backup before it moves into
/// `game`. Idempotent: a copy already holding the same bytes is kept, and a
/// partial one from an interrupted run is copied again.
pub(super) fn keep_login(path: &str, source: &Path, plan: &Plan) -> Result<(), Error> {
    let backup = plan.report.backup.as_ref().ok_or(StorageError::Corrupt)?;
    let home = inventory::native(backup, path);
    let entry = inventory::hash(source, &CancellationToken::new())?;
    let expected = Hashed {
        sha256: entry.sha256,
        size: entry.size,
    };
    if present(&home)? {
        if holds(&home, &expected)? {
            return Ok(());
        }
        std::fs::remove_file(&home)?;
    }
    if let Some(parent) = home.parent() {
        std::fs::create_dir_all(parent)?;
    }
    inventory::copy(source, &home, &entry, &CancellationToken::new())?;
    tracing::debug!(target: "adoption", path, original = %home.display(), "in-place adoption: the login file's original was kept in the backup");
    Ok(())
}

/// Abandon drops the backup's copy of a login file once the folder holds the same
/// bytes again. A copy whose original is not back in the folder stays.
pub(super) fn drop_login_copy(path: &str, restored: &Path, plan: &Plan) -> Result<(), Error> {
    let backup = plan.report.backup.as_ref().ok_or(StorageError::Corrupt)?;
    let home = inventory::native(backup, path);
    if !present(&home)? {
        return Ok(());
    }
    let entry = inventory::hash(&home, &CancellationToken::new())?;
    let expected = Hashed {
        sha256: entry.sha256,
        size: entry.size,
    };
    if present(restored)? && holds(restored, &expected)? {
        std::fs::remove_file(&home)?;
    }
    Ok(())
}

/// The work folder's replacement copies and parked originals are spent once the
/// adoption is published. Best effort: a folder left behind is logged and kept.
/// The backup is a sibling of this folder and is never touched here.
pub(super) fn remove_work_folder(plan: &Plan) {
    let stage = plan.stage();
    match std::fs::remove_dir_all(&stage) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => {
            tracing::warn!(target: "adoption", ?error, directory = %stage.display(), "in-place adoption: could not remove the work folder; it is left in place")
        }
    }
}

/// Remove the directories the moves emptied, deepest first. A directory that
/// still holds anything (the player's own files) stays.
pub(super) fn remove_emptied(root: &Path, files: &[&str]) -> Result<(), Error> {
    let mut directories = BTreeSet::new();
    for file in files {
        let mut parts: Vec<&str> = file.split('/').collect();
        parts.pop();
        while !parts.is_empty() {
            directories.insert(parts.join("/"));
            parts.pop();
        }
    }
    let mut directories: Vec<String> = directories.into_iter().collect();
    directories.sort_by_key(|d| std::cmp::Reverse(d.matches('/').count()));
    for directory in directories {
        let path = inventory::native(root, &directory);
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !super::super::failed_cleanup::plain(&meta) || !meta.is_dir() {
            continue;
        }
        if std::fs::read_dir(&path)?.next().is_none() {
            std::fs::remove_dir(&path)?;
        }
    }
    Ok(())
}
