//! The final check of an in-place adoption, before the receipt. A full check compares
//! one digest of the owned tree; a light check compares the tree file by file against
//! the one stored at admission, so a mismatch names what differs and a spelling-only
//! difference (the directory's case) no longer fails an otherwise correct adoption.
use super::in_place::SETUP_FILES;
use super::publication::read_large;
#[cfg(all(test, not(target_os = "macos")))]
use super::publication::write_large;
use super::tree_diff::{diff, log_tree_checked, TreeOutcome};
use super::verification::Tree;
use super::*;
use std::collections::BTreeMap;

/// Where admission stores a light adoption's expected tree, inside the work folder
/// (C-LF3). It is not part of `Plan`, which is digested and rejects unknown fields.
pub(super) const EXPECTED_TREE: &str = "expected-tree.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    version: u32,
    tree: BTreeMap<String, (u64, String)>,
}

/// Writes `<stage>/expected-tree.json` atomically. The work folder may not exist
/// yet: it is created by the first replacement copy, and a light adoption can have none.
#[cfg(all(test, not(target_os = "macos")))]
pub(super) fn write_expected(stage: &Path, tree: &Tree) -> Result<(), Error> {
    std::fs::create_dir_all(stage)?;
    let stored = Stored {
        version: 1,
        tree: tree
            .iter()
            .map(|(path, (size, sha))| (path.clone(), (*size, to_hex(sha))))
            .collect(),
    };
    write_large(stage, EXPECTED_TREE, &stored)
}

/// Write the expected tree through the retained work-directory descriptor.
pub(super) fn write_expected_guarded(plan: &Plan, tree: &Tree) -> Result<(), Error> {
    let path = plan.stage().join(EXPECTED_TREE);
    let guard = super::direct_paths::Parents::new(plan, &[&path], true)?;
    let stored = Stored {
        version: 1,
        tree: tree
            .iter()
            .map(|(path, (size, sha))| (path.clone(), (*size, to_hex(sha))))
            .collect(),
    };
    guard.write(
        &path,
        &serde_json::to_vec(&stored).map_err(|_| StorageError::Corrupt)?,
    )
}

/// The stored tree, or `None` when the file is absent, unreadable or not what
/// `write_expected` writes: an adoption an earlier build began has none, and the
/// check then has nothing to compare (it reports `unrecorded`).
pub(super) fn read_expected(stage: &Path) -> Option<Tree> {
    let stored: Stored = read_large(&stage.join(EXPECTED_TREE)).ok()?;
    if stored.version != 1 {
        return None;
    }
    stored
        .tree
        .into_iter()
        .map(|(path, (size, hex))| Some((path, (size, from_hex(&hex)?))))
        .collect()
}

pub(super) fn validate(game: &Path, plan: &Plan) -> Result<(), Error> {
    inventory::same_directory(game, &plan.stage_identity)?;
    let scanned = if plan.schema_version == 2 {
        super::direct::scan(game, plan)?
    } else {
        inventory::scan(game, &CancellationToken::new())?
    };
    if !matches!(plan.provenance.check, FileCheck::Light) {
        if inventory::content_digest(&scanned)? != plan.provenance.reference_digest {
            log_tree_checked("verified", TreeOutcome::Mismatch, None);
            return Err(StorageError::Corrupt.into());
        }
        return Ok(());
    }
    let Some(stored) = read_expected(&plan.stage()) else {
        log_tree_checked("light", TreeOutcome::Unrecorded, None);
        return Ok(());
    };
    // The ledger is synthesized after the tree was predicted and is not in it. A
    // ledger the folder already held was predicted as a release-side entry, so it
    // is dropped from both trees: the comparison never judges it.
    let [_, _, ledger] = SETUP_FILES;
    let expected: Tree = stored
        .into_iter()
        .filter(|(path, _)| !path.eq_ignore_ascii_case(ledger))
        .collect();
    let actual: Tree = scanned
        .iter()
        .filter(|(path, _)| !path.eq_ignore_ascii_case(ledger))
        .map(|(path, entry)| (path.clone(), (entry.size, entry.sha256)))
        .collect();
    let found = diff(&expected, &actual, &SETUP_FILES);
    let outcome = if found.is_empty() {
        TreeOutcome::Matched
    } else if found.blocking() {
        TreeOutcome::Mismatch
    } else {
        TreeOutcome::Accepted
    };
    log_tree_checked("light", outcome, Some(&found));
    if found.blocking() {
        return Err(StorageError::Corrupt.into());
    }
    Ok(())
}

/// Where a light adoption keeps the setup files as they were before setup rewrote
/// them (C-LF3), inside the work folder.
pub(super) const SETUP_ORIGINALS: &str = "setup-originals";
/// File name of the manifest that marks a complete set of setup originals; written last.
pub(super) const SETUP_MANIFEST: &str = "setup-originals.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Originals {
    version: u32,
    files: Vec<Original>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Original {
    path: String,
    existed: bool,
}

/// The spelling `key` has on disk under `game`, or `None` when no such file is there.
/// The disk folds case, so a lookup by the key would succeed under another spelling
/// than the one the file keeps; the copy and the manifest record the real one.
fn found_on_disk(game: &Path, key: &str) -> Option<String> {
    let mut dir = game.to_path_buf();
    let mut spelled = Vec::new();
    for part in key.split('/') {
        let names: Vec<String> = std::fs::read_dir(&dir)
            .ok()?
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .collect();
        let name = names
            .iter()
            .find(|name| *name == part)
            .or_else(|| names.iter().find(|name| name.eq_ignore_ascii_case(part)))?
            .clone();
        dir.push(&name);
        spelled.push(name);
    }
    std::fs::symlink_metadata(&dir)
        .ok()
        .filter(|meta| meta.is_file())
        .map(|_| spelled.join("/"))
}

/// Light only (R10: a verified adoption does nothing new). Copies each setup file
/// that exists in `game` into the work folder, flushes every copy, and writes the
/// manifest last, so a manifest on disk means the copies are whole. Idempotent: a rerun
/// starts from nothing and overwrites both.
pub(super) fn save_setup_originals(plan: &Plan, game: &Path) -> Result<(), Error> {
    if !matches!(plan.provenance.check, FileCheck::Light) {
        return Ok(());
    }
    let stage = plan.stage();
    let manifest = stage.join(SETUP_MANIFEST);
    let stage_guard = super::direct_paths::Parents::new(plan, &[&manifest], true)?;
    stage_guard.remove(&manifest, false)?;
    stage_guard.remove_tree(&stage.join(SETUP_ORIGINALS))?;
    let copies = stage.join(SETUP_ORIGINALS);
    let mut files = Vec::new();
    for key in SETUP_FILES {
        let Some(found) = found_on_disk(game, key) else {
            files.push(Original {
                path: key.to_string(),
                existed: false,
            });
            continue;
        };
        let to = inventory::native(&copies, &found);
        super::direct_paths::copy(plan, &inventory::native(game, &found), &to)?;
        files.push(Original {
            path: found,
            existed: true,
        });
    }
    stage_guard.write(
        &manifest,
        &serde_json::to_vec(&Originals { version: 1, files }).map_err(|_| StorageError::Corrupt)?,
    )
}

fn read_originals(stage: &Path) -> Option<Originals> {
    let found: Originals = read_large(&stage.join(SETUP_MANIFEST)).ok()?;
    (found.version == 1).then_some(found)
}

/// Whether the work folder holds a complete, readable set of setup originals.
pub(super) fn has_setup_originals(stage: &Path) -> bool {
    read_originals(stage).is_some()
}

/// Puts the setup files in `game` back as `save_setup_originals` found them: a file that
/// existed is copied back over, one that did not is deleted. A file that existed and is
/// no longer under `game` was already moved home by an earlier run, so it is skipped and
/// not counted. Returns how many files it touched. Safe to run again after a crash.
#[cfg(all(test, not(target_os = "macos")))]
pub(super) fn restore_setup_originals(stage: &Path, game: &Path) -> Result<u32, Error> {
    let originals = read_originals(stage).ok_or(StorageError::Corrupt)?;
    let mut touched = 0;
    for row in &originals.files {
        // A row names a file under `game`; a name that climbs out of it is not ours.
        if row
            .path
            .split('/')
            .any(|part| part.is_empty() || part == ".." || part.contains(['\\', ':']))
        {
            return Err(StorageError::Corrupt.into());
        }
        let target = inventory::native(game, &row.path);
        if row.existed {
            // A file no longer under `game` was already moved home by an earlier run.
            if std::fs::symlink_metadata(&target).is_err() {
                continue;
            }
            let from = inventory::native(&stage.join(SETUP_ORIGINALS), &row.path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&from, &target).map_err(|_| StorageError::Corrupt)?;
            touched += 1;
        } else {
            match std::fs::remove_file(&target) {
                Ok(()) => {
                    touched += 1;
                    // Setup made the folders around a file the player never had; an
                    // empty one would keep the game folder from being removed.
                    let mut dir = target.parent();
                    while let Some(empty) = dir.filter(|d| *d != game) {
                        if std::fs::remove_dir(empty).is_err() {
                            break;
                        }
                        dir = empty.parent();
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(touched)
}

/// Plan-aware rollback uses retained parent handles for every setup mutation.
pub(super) fn restore_setup_guarded(plan: &Plan) -> Result<u32, Error> {
    if plan.schema_version == 2 {
        return super::setup_expectations::restore(plan);
    }
    let stage = plan.stage();
    let root = &plan.installation.destination;
    let manifest = stage.join(SETUP_MANIFEST);
    let guard = super::direct_paths::Parents::new(plan, &[&manifest], false)?;
    let originals: Originals =
        serde_json::from_slice(&guard.read(&manifest)?).map_err(|_| StorageError::Corrupt)?;
    if originals.version != 1 {
        return Err(StorageError::Corrupt.into());
    }
    let mut touched = 0;
    for row in originals.files {
        if row
            .path
            .split('/')
            .any(|part| part.is_empty() || part == ".." || part.contains(['\\', ':']))
        {
            return Err(StorageError::Corrupt.into());
        }
        let target = inventory::native(root, &row.path);
        if row.existed {
            let source = inventory::native(&stage.join(SETUP_ORIGINALS), &row.path);
            super::direct_paths::copy(plan, &source, &target)?;
            touched += 1;
        } else {
            if std::fs::symlink_metadata(&target).is_ok() {
                touched += 1;
            }
            super::direct_paths::remove(plan, &target)?;
        }
    }
    Ok(touched)
}

fn to_hex(sha: &[u8; 32]) -> String {
    sha.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn from_hex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(text.as_bytes().chunks(2)) {
        *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}
