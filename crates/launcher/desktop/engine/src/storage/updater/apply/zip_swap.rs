//! Windows self-update by zip swap (D-LX3). The release zip holds one root
//! `.exe` beside its `windows/` and `graphics/` folders. It is extracted into
//! `<exe dir>\.update-<version>-<owner>\`, every file it replaces is renamed to
//! `<name>.old` (Windows lets a running image be renamed, not overwritten),
//! and the new files are renamed in. The executable moves last, so a launcher
//! that starts from the installed path with the new compiled version proves
//! every other file is already in place.
use super::{
    bundle::{self, MAX_ENTRIES, MAX_EXPANDED},
    Error,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const TARGET: &str = "launcher.update";
const OWNER_MARKER: &str = ".cimmeria-owner";
const OLD_SUFFIX: &str = ".old";
pub(super) const STAGE_PREFIX: &str = ".update-";

/// One file the swap places. Paths are `/`-separated and relative: `source`
/// to the stage, `path` to the install directory. They differ only for the
/// executable, which keeps whatever name the player gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::storage::updater) struct SwapFile {
    source: String,
    path: String,
    /// An installed file existed and is renamed to `<path>.old` first.
    replaced: bool,
}
/// Durable before the first rename, so rollback and cleanup never guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::storage::updater) struct Plan {
    /// Stage marker owner. Kept here because the record's owner is cleared on
    /// acknowledgment, while a later start may still need to retry cleanup.
    owner: Uuid,
    files: Vec<SwapFile>,
}

/// Named per attempt: a stage an earlier attempt could not delete (an
/// antivirus scan holding a new `.exe`) never blocks a retry of the version.
pub(super) fn stage_name(version: &str, owner: Uuid) -> String {
    format!("{STAGE_PREFIX}{version}-{owner}")
}

/// Best-effort removal of stages earlier attempts left behind. Only a folder
/// whose owner marker names an attempt other than `keep` is removed; a folder
/// without a readable marker was never ours and is left alone.
pub(super) fn sweep_stale_stages(install: &Path, keep: Option<Uuid>) {
    let Ok(entries) = fs::read_dir(install) else {
        return;
    };
    for entry in entries.flatten() {
        let stage = entry.path();
        let is_stage = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.to_ascii_lowercase().starts_with(STAGE_PREFIX));
        let Some(owner) = is_stage.then(|| stage_owner(&stage)).flatten() else {
            continue;
        };
        if Some(owner) == keep {
            continue;
        }
        match remove_marked(&stage) {
            Ok(()) => {
                tracing::info!(target: TARGET, event = "zip_stale_stage_removed", stage = %stage.display(), %owner, "removed an earlier update attempt's stage")
            }
            Err(error) => {
                tracing::warn!(target: TARGET, event = "zip_cleanup_failed", %error, stage = %stage.display(), "could not remove an earlier update attempt's stage; the next start retries")
            }
        }
    }
}
fn stage_owner(stage: &Path) -> Option<Uuid> {
    let meta = fs::symlink_metadata(stage).ok()?;
    if !meta.is_dir() || reparse(&meta) {
        return None;
    }
    let owner = Uuid::parse_str(fs::read_to_string(stage.join(OWNER_MARKER)).ok()?.trim()).ok()?;
    // The same check removal of a current stage uses: plain folder, exact marker.
    bundle::owned_stage(stage, owner).ok().map(|_| owner)
}
/// Delete a stage's contents before its owner marker, so a delete that fails
/// part-way (a held file) leaves a stage a later sweep still recognizes.
fn remove_marked(stage: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(stage)? {
        let entry = entry?;
        if entry.file_name() == OWNER_MARKER {
            continue;
        }
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    fs::remove_dir_all(stage)
}

/// Two passes: the whole namespace is validated before the stage exists.
/// Returns the stage-relative files with the root executable's name first.
pub(in crate::storage::updater) fn extract(
    bytes: &[u8],
    stage: &Path,
    owner: Uuid,
) -> Result<Vec<String>, Error> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
        tracing::warn!(target: TARGET, event = "zip_refused", reason = "not_a_zip", %error, "update package is not a readable zip");
        Error::Package
    })?;
    if archive.len() > MAX_ENTRIES {
        return Err(refuse("too_many_entries", ""));
    }
    let mut seen = BTreeSet::new();
    let mut dirs = BTreeSet::new();
    let mut files = Vec::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| refuse("unreadable_entry", ""))?;
        let name = entry.name().to_owned();
        if entry.is_symlink() {
            return Err(refuse("link_entry", &name));
        }
        let parts = super::zip_names::components(&name, entry.is_dir())?;
        let key = parts.join("/").to_lowercase();
        if !seen.insert(key.clone()) {
            return Err(refuse("duplicate_entry", &name));
        }
        // Every ancestor of every entry is a directory, never also a file.
        for depth in 1..parts.len() {
            dirs.insert(parts[..depth].join("/").to_lowercase());
        }
        if entry.is_dir() {
            dirs.insert(key);
            continue;
        }
        if !entry.is_file() {
            return Err(refuse("special_entry", &name));
        }
        let last = parts.last().map(|p| p.to_lowercase()).unwrap_or_default();
        if last.ends_with(OLD_SUFFIX) || (parts.len() == 1 && last == OWNER_MARKER) {
            return Err(refuse("reserved_name", &name));
        }
        total = total
            .checked_add(entry.size())
            .filter(|n| *n <= MAX_EXPANDED)
            .ok_or_else(|| {
                tracing::warn!(target: TARGET, event = "zip_refused", reason = "too_large", "update package expands past the limit");
                Error::Size
            })?;
        files.push((index, parts.join("/"), key));
    }
    if let Some((_, name, _)) = files.iter().find(|(_, _, key)| dirs.contains(key)) {
        return Err(refuse("file_and_directory", name));
    }
    let exes: Vec<_> = files
        .iter()
        .filter(|(_, name, _)| !name.contains('/') && name.to_lowercase().ends_with(".exe"))
        .collect();
    if exes.len() != 1 {
        return Err(refuse("root_executable_count", ""));
    }
    let exe = exes[0].1.clone();

    let parent = stage.parent().ok_or(Error::Target)?;
    bundle::plain(parent)?;
    // Exclusive: a colliding directory is never adopted because of its name.
    fs::create_dir(stage).map_err(|error| {
        tracing::warn!(target: TARGET, event = "zip_stage_failed", %error, stage = %stage.display(), "could not create the update stage");
        Error::Replace
    })?;
    fs::write(stage.join(OWNER_MARKER), owner.to_string()).map_err(|_| Error::Replace)?;
    for (index, name, _) in &files {
        let mut entry = archive
            .by_index(*index)
            .map_err(|_| refuse("unreadable_entry", name))?;
        let out = join(stage, name);
        if let Some(dir) = out.parent() {
            fs::create_dir_all(dir).map_err(|_| Error::Replace)?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out)
            .map_err(|_| Error::Replace)?;
        let declared = entry.size();
        // The CRC check runs as the entry reaches its end, inside io::copy.
        let copied = std::io::copy(&mut (&mut entry).take(declared + 1), &mut file)
            .map_err(|_| refuse("corrupt_entry", name))?;
        if copied != declared {
            return Err(refuse("size_mismatch", name));
        }
        file.flush()
            .and_then(|_| file.sync_all())
            .map_err(|_| Error::Replace)?;
    }
    let mut names: Vec<String> = files.into_iter().map(|(_, name, _)| name).collect();
    names.retain(|name| *name != exe);
    names.insert(0, exe);
    Ok(names)
}

/// Preflight before any rename: every destination is absent or a plain file,
/// and a leftover `<name>.old` from an earlier update is deleted now, or the
/// swap is refused with nothing changed. The executable is placed last.
pub(super) fn plan(
    install: &Path,
    executable: &Path,
    owner: Uuid,
    staged: Vec<String>,
) -> Result<Plan, Error> {
    let exe_name = executable
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(Error::Target)?
        .to_owned();
    let mut iter = staged.into_iter();
    let exe_source = iter.next().ok_or(Error::Package)?;
    let mut files = Vec::new();
    for source in iter {
        if source.eq_ignore_ascii_case(&exe_name) {
            return Err(refuse("executable_name_collision", &source));
        }
        files.push((source.clone(), source));
    }
    files.push((exe_source, exe_name));
    let mut plan = Plan {
        owner,
        files: Vec::with_capacity(files.len()),
    };
    for (source, path) in files {
        let dest = join(install, &path);
        check_ancestors(install, &dest)?;
        let replaced = match fs::symlink_metadata(&dest) {
            Ok(meta) if meta.is_file() && !reparse(&meta) => true,
            Ok(_) => return Err(refuse_target("destination_not_a_file", &dest)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(refuse_target("destination_unreadable", &dest)),
        };
        let old = old_path(&dest);
        match fs::symlink_metadata(&old) {
            Ok(meta) if meta.is_file() && !reparse(&meta) => {
                fs::remove_file(&old).map_err(|error| {
                    tracing::warn!(target: TARGET, event = "zip_refused", reason = "leftover_old_locked", %error, path = %old.display(), "a previous update's backup could not be removed; nothing was changed");
                    Error::Target
                })?;
            }
            Ok(_) => return Err(refuse_target("leftover_old_not_a_file", &old)),
            Err(_) => {}
        }
        plan.files.push(SwapFile {
            source,
            path,
            replaced,
        });
    }
    Ok(plan)
}

/// Rename every replaced file aside, then its new copy in, one file at a time.
/// On the first failure the caller rolls the whole plan back.
pub(super) fn swap_in(install: &Path, stage: &Path, plan: &Plan) -> Result<(), Error> {
    for file in &plan.files {
        let dest = join(install, &file.path);
        if file.replaced {
            move_file(&dest, &old_path(&dest))?;
        } else if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir).map_err(|error| {
                tracing::warn!(target: TARGET, event = "zip_swap_failed", %error, path = %dir.display(), "could not create a launcher folder");
                Error::Replace
            })?;
        }
        move_file(&join(stage, &file.source), &dest)?;
    }
    Ok(())
}

/// Restore the installed files byte for byte from their `.old` copies and
/// remove files the update added. Safe on any prefix of [`swap_in`], so it
/// also recovers a process that died mid-swap.
pub(super) fn roll_back(install: &Path, plan: &Plan) -> Result<(), Error> {
    let mut result = Ok(());
    for file in plan.files.iter().rev() {
        let dest = join(install, &file.path);
        let discarded = discard_path(&dest, plan.owner);
        let step = if file.replaced {
            let old = old_path(&dest);
            if fs::symlink_metadata(&old).is_ok() {
                // Preflight removed any leftover, so an existing `.old` is the
                // original this swap moved aside, and `dest` is the new copy.
                set_aside(&dest, &discarded).and_then(|_| {
                    fs::rename(&old, &dest).map_err(|error| {
                        tracing::error!(target: TARGET, event = "zip_rollback_failed", %error, path = %dest.display(), "could not put an original launcher file back");
                        Error::Reconciliation
                    })
                })
            } else {
                Ok(())
            }
        } else {
            set_aside(&dest, &discarded)
        };
        // A new file something has loaded (a DLL) can be renamed but not
        // deleted; its discarded copy is removed when it is no longer in use.
        if let Err(error) = remove_file_if_present(&discarded) {
            tracing::warn!(target: TARGET, event = "zip_cleanup_failed", %error, path = %discarded.display(), "could not delete a rolled-back launcher file; it is harmless and can be deleted by hand");
        }
        if step.is_ok() && !file.replaced {
            prune_empty(install, &dest);
        }
        if step.is_err() {
            // Keep going: every file restored now is one less to fix by hand.
            result = Err(Error::Reconciliation);
        }
    }
    result
}

/// Startup cleanup after acknowledgment: the `.old` copies and the stage.
pub(super) fn clean_up(install: &Path, stage: &Path, plan: &Plan) -> Result<(), Error> {
    let mut result = Ok(());
    for file in plan.files.iter().filter(|f| f.replaced) {
        let old = old_path(&join(install, &file.path));
        if let Err(error) = remove_file_if_present(&old) {
            tracing::warn!(target: TARGET, event = "zip_cleanup_failed", %error, path = %old.display(), "could not remove a replaced launcher file; the next start or update retries");
            result = Err(Error::Replace);
        }
    }
    remove_stage(stage, plan.owner).and(result)
}

/// Remove a stage only when its marker names this owner. A foreign stage is
/// an error and is preserved. Failing to delete our own stage is only logged:
/// the installed files are already consistent, the stage is never adopted
/// (every attempt stages under its own name), and the marker is kept so the
/// next start or update sweeps it.
pub(super) fn remove_stage(stage: &Path, owner: Uuid) -> Result<(), Error> {
    if fs::symlink_metadata(stage).is_err() {
        return Ok(());
    }
    bundle::owned_stage(stage, owner).inspect_err(|_| {
        tracing::warn!(target: TARGET, event = "zip_stage_foreign", stage = %stage.display(), "update stage is not owned by this attempt; preserved");
    })?;
    if let Err(error) = remove_marked(stage) {
        tracing::warn!(target: TARGET, event = "zip_cleanup_failed", %error, stage = %stage.display(), "could not remove the update stage");
    }
    Ok(())
}

pub(super) fn plan_owner(plan: &Plan) -> Uuid {
    plan.owner
}
/// The staged executable: [`plan`] always places it last.
pub(super) fn executable_source(plan: &Plan) -> Option<&str> {
    plan.files.last().map(|file| file.source.as_str())
}

#[cfg(test)]
thread_local! {
    /// Fail the Nth rename of the next swap (1-based).
    pub(super) static FAIL_MOVE: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}
fn move_file(from: &Path, to: &Path) -> Result<(), Error> {
    #[cfg(test)]
    if FAIL_MOVE.with(|slot| match slot.get() {
        Some(1) => {
            slot.set(None);
            true
        }
        Some(n) => {
            slot.set(Some(n - 1));
            false
        }
        None => false,
    }) {
        tracing::warn!(target: TARGET, event = "zip_swap_failed", from = %from.display(), "injected rename failure");
        return Err(Error::Replace);
    }
    super::process::rename_new(from, to).inspect_err(|_| {
        tracing::warn!(target: TARGET, event = "zip_swap_failed", from = %from.display(), to = %to.display(), "could not rename a launcher file");
    })
}

fn join(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(root.to_path_buf(), |p, c| p.join(c))
}
fn old_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(OLD_SUFFIX);
    dest.with_file_name(name)
}
fn reparse(_meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        _meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        _meta.file_type().is_symlink()
    }
}
/// Every existing folder between the install directory and a destination is
/// a real directory, never a link or junction the rename could follow.
fn check_ancestors(install: &Path, dest: &Path) -> Result<(), Error> {
    let mut current = dest.parent();
    while let Some(dir) = current.filter(|d| d.starts_with(install) && *d != install) {
        match fs::symlink_metadata(dir) {
            Ok(meta) if meta.is_dir() && !reparse(&meta) && !meta.file_type().is_symlink() => {}
            Ok(_) => return Err(refuse_target("destination_folder_not_plain", dir)),
            Err(_) => {}
        }
        current = dir.parent();
    }
    Ok(())
}
fn remove_file_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
/// `<name>.new-<owner>`: where rollback moves a new file before deleting it.
fn discard_path(dest: &Path, owner: Uuid) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".new-{owner}"));
    dest.with_file_name(name)
}
/// Rename a new file off its real name. Rename works on a loaded image where
/// delete does not, so the original name is always freed.
fn set_aside(dest: &Path, discarded: &Path) -> Result<(), Error> {
    if matches!(fs::symlink_metadata(dest), Err(ref e) if e.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(());
    }
    fs::rename(dest, discarded).map_err(|error| {
        tracing::error!(target: TARGET, event = "zip_rollback_failed", %error, path = %dest.display(), "could not move a new launcher file aside during rollback");
        Error::Reconciliation
    })
}
/// Folders the update created are removed once empty; others stay.
fn prune_empty(install: &Path, dest: &Path) {
    let mut current = dest.parent();
    while let Some(dir) = current.filter(|d| d.starts_with(install) && *d != install) {
        if fs::remove_dir(dir).is_err() {
            break;
        }
        current = dir.parent();
    }
}
pub(super) fn refuse(reason: &'static str, entry: &str) -> Error {
    tracing::warn!(target: TARGET, event = "zip_refused", reason, entry, "update package refused before any write");
    Error::Package
}
fn refuse_target(reason: &'static str, path: &Path) -> Error {
    tracing::warn!(target: TARGET, event = "zip_refused", reason, path = %path.display(), "launcher folder refused the update; nothing was changed");
    Error::Target
}

/// Build a release-shaped zip for tests: `(name, bytes)` pairs, a trailing
/// `/` making a directory.
#[cfg(any(test, feature = "test-support"))]
pub fn zip_fixture(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        if let Some(dir) = name.strip_suffix('/') {
            writer.add_directory(dir, options).unwrap();
        } else {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
    }
    writer.finish().unwrap().into_inner()
}
