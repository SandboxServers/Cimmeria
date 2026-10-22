//! Seedless incremental Update (LA-13, D-LA14).
//!
//! A light-adopted install never held the stock seed, so rebuilding it from the
//! 4.1 GB seed would throw away what the player already has. When the install
//! qualifies, the replacement tree is built from the live `game/` tree instead
//! (hard links, copying only what the update will write) and the new patches
//! are applied on top, tolerantly like the adoption itself (D-LA9).
//!
//! Only the installed-content record's adoption provenance decides whether this
//! runs (`check = light`). Fresh, downloaded and copy-adopted installs keep the
//! seed rebuild byte for byte (R-LA 10); [`plan`] then only checks that the
//! live ledger really matches the target release.
use super::preparation::Failure;
use super::Plan;
use crate::install::{self, InstallError, Progress};
use crate::install_progress::{ProgressReporter, ProgressSink};
use crate::manifest::{blob_url, PatchEntry};
use crate::state::InstalledState;
use crate::storage::adoption::FileCheck;
use crate::{client_setup, install_layout};
use cimmeria_launcher_core::stage_seed::{self, SeedStats};
use cimmeria_launcher_core::{patch_dest, patch_summary};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// The progress label while the stage is seeded from the live tree.
const PREPARING: &str = "Preparing update";

/// True only for an install adopted with the light check (D-LA14). A downloaded
/// install (`None`) and a copy-adopted one (`Full`) keep the seed rebuild.
pub(super) fn seedless_allowed(check: Option<FileCheck>) -> bool {
    matches!(check, Some(FileCheck::Light))
}

/// What the seedless Update has left to apply.
#[derive(Debug)]
pub(super) struct IncrementalPlan {
    /// The target's patches the live ledger has not recorded, in manifest order.
    pending: Vec<PatchEntry>,
}

/// Whether the live ledger at `game` can take `target` without the seed.
///
/// All of these must hold, else `None` and the caller rebuilds from the seed:
/// the ledger names the target's seed; it is a real record, not a bare adoption
/// flag; its applied patches are, in order, a prefix of the target's patches;
/// and each of those patches is the same blob in the previous release. The
/// ledger stores keys only, so a republished patch with the same id but new
/// bytes must not be taken as already applied.
pub(super) fn plan(
    game: &Path,
    previous: &[PatchEntry],
    target_seed: &str,
    target: &[PatchEntry],
) -> Option<IncrementalPlan> {
    let ledger = InstalledState::load(game);
    if ledger.seed_adopted {
        return None;
    }
    if !ledger
        .seed_sha256
        .as_deref()
        .is_some_and(|sha| sha.eq_ignore_ascii_case(target_seed))
    {
        return None;
    }
    let applied = &ledger.applied_patches;
    if applied.len() > target.len() {
        return None;
    }
    for (key, entry) in applied.iter().zip(target) {
        if *key != entry.state_key() {
            return None;
        }
        let before = previous.iter().find(|p| p.state_key() == *key)?;
        if !before.sha256.eq_ignore_ascii_case(&entry.sha256) {
            return None;
        }
    }
    Some(IncrementalPlan {
        pending: target[applied.len()..].to_vec(),
    })
}

/// Downloads one pending patch into `work` under the name the install step
/// uses for its own partial download, and checks its hash. The install step
/// then finds a complete file and asks the server only for the missing tail.
async fn fetch(
    entry: &PatchEntry,
    manifest_url: &str,
    work: &Path,
    http: &reqwest::Client,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<PathBuf, Failure> {
    let dest = work.join(install::patch_download_name(entry).map_err(|_| Failure::Failed)?);
    install::download_to_file(
        http,
        &blob_url(manifest_url, &entry.blob),
        &dest,
        cancel.clone(),
        CancellationToken::new(),
        entry.size,
        &entry.id,
        progress,
    )
    .await
    .map_err(|error| match error {
        InstallError::Cancelled => Failure::Cancelled,
        _ => Failure::Failed,
    })?;
    let hashed = dest.clone();
    let actual = tokio::task::spawn_blocking(move || install::hash_file(&hashed))
        .await
        .map_err(|_| Failure::ReconciliationRequired)?
        .map_err(|_| Failure::Failed)?;
    if !actual.eq_ignore_ascii_case(&entry.sha256) {
        let _ = std::fs::remove_file(&dest);
        return Err(Failure::Failed);
    }
    Ok(dest)
}

/// Builds the stage from the live tree and returns how it was seeded. The new
/// patch zips are fetched first, because the write set comes from their
/// contents. Afterwards each zip sits in the stage root, where the install step
/// looks for it.
pub(super) async fn stage(
    plan: &Plan,
    incremental: &IncrementalPlan,
    manifest_url: &str,
    http: &reqwest::Client,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<SeedStats, Failure> {
    let work = plan.work_directory();
    let mut zips = Vec::new();
    for entry in &incremental.pending {
        let zip = fetch(entry, manifest_url, &work, http, cancel, progress).await?;
        zips.push((entry.clone(), zip));
    }
    let live = plan.owner.destination.join("game");
    let target = plan.stage();
    let cancel = cancel.clone();
    let progress = progress.clone();
    tokio::task::spawn_blocking(move || seed(&live, &target, &zips, &cancel, &progress))
        .await
        .map_err(|_| Failure::ReconciliationRequired)?
}

fn seed(
    live: &Path,
    target: &Path,
    zips: &[(PatchEntry, PathBuf)],
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<SeedStats, Failure> {
    let write_set = write_set(live, zips).ok_or(Failure::Failed)?;
    let step = |current| {
        progress.report(Progress::Extracting {
            label: PREPARING.to_owned(),
            current,
            total: 1,
            filename: String::new(),
        });
    };
    step(0);
    // Hard links on Windows only; elsewhere a copy keeps the stage independent.
    let stats = stage_seed::seed_stage(live, target, &write_set, cfg!(windows), &|| {
        cancel.is_cancelled()
    })
    .map_err(|error| match error.kind() {
        std::io::ErrorKind::Interrupted => Failure::Cancelled,
        _ => Failure::Failed,
    })?;
    step(1);
    for (entry, zip) in zips {
        let name = install::patch_download_name(entry).map_err(|_| Failure::Failed)?;
        std::fs::rename(zip, target.join(name)).map_err(|_| Failure::Failed)?;
    }
    Ok(stats)
}

/// Every live file the update will write. All of them are copied into the
/// stage; any other file is a hard link, so a write to it would change the
/// live tree.
///
/// That is the new patches' outputs, the targets an earlier tolerant apply
/// skipped (the install step retries them), the ledger, and the two files the
/// client setup rewrites in place (the ASLR flag in `SGW.exe` and
/// `LoginInternal.lua`). Anything else the install step writes goes through a
/// temporary file and a rename, which replaces the link rather than its
/// content. `None` when a pending patch zip cannot be read.
fn write_set(live: &Path, zips: &[(PatchEntry, PathBuf)]) -> Option<BTreeSet<String>> {
    let mut set = BTreeSet::new();
    for (entry, zip) in zips {
        let summary = patch_summary::summarize(zip, entry).ok()?;
        // Where the patch really lands, in case the tree is not the stock layout.
        let landing = patch_dest::patch_dest(live, entry)
            .ok()
            .and_then(|dest| relative(live, &dest));
        for output in summary.outputs {
            if let (Some(landing), Some(rest)) = (
                landing.as_deref(),
                output
                    .path
                    .strip_prefix(patch_summary::SGW_GAME_DIR)
                    .map(|rest| rest.trim_start_matches('/')),
            ) {
                set.insert(key(&format!("{landing}/{rest}")));
            }
            set.insert(key(&output.path));
        }
    }
    for incomplete in InstalledState::load(live).incomplete_patches {
        set.extend(incomplete.skipped.iter().map(|skipped| key(&skipped.path)));
    }
    for fixed in [
        InstalledState::path(live),
        install_layout::sgw_exe(live),
        client_setup::login_servers::path(live),
    ] {
        set.extend(relative(live, &fixed).map(|rel| key(&rel)));
    }
    Some(set)
}

/// `path` below `live`, `/` separated.
fn relative(live: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(live).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// The write-set key `stage_seed` compares against.
fn key(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches('/')
        .to_lowercase()
}

#[cfg(test)]
mod tests;
