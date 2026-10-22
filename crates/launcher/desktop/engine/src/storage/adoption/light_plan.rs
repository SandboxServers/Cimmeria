//! The light plan: which patches of a signed release a player's folder has not
//! yet taken, and what fetching them costs. A light adoption has no stock seed,
//! so nothing here fetches the seed. Each patch's contract is read from its zip
//! (LA-01) and matched against the folder's scanned hashes.
use super::artifacts::{self, Transport};
use super::inventory;
use super::rejection::{self, Rejection};
use super::*;
use crate::install_progress::ProgressSink;
use cimmeria_launcher_core::install_layout;
use cimmeria_launcher_core::patch_summary::{self, PatchSummary};
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The seed's bundled cooked tier as the install places it.
const ALIAS: &str = "Working/SGWGame/SourceCache.en-us/";
/// The same tier under the stock spelling the seed ships it with.
const STOCK: &str = "Working/SGWGame/Cache.en-US/";

/// What a light adoption must do: each patch's contract, its zip, the signed-order
/// indices of the patches the folder has not taken, and the bytes fetching costs.
pub(super) struct LightPlan {
    pub summaries: Vec<PatchSummary>,
    pub zips: Vec<PathBuf>,
    pub pending: Vec<usize>,
    pub download_bytes: u64,
}

/// Fetches every patch (never the seed), summarizes each one and returns the
/// patches `scan` does not show as taken. `cache` is the state root the blobs
/// are stored under.
pub(super) fn plan(
    transport: &Transport,
    release: &VerifiedRelease,
    cache: &Path,
    scan: &inventory::Index,
    cancel: &CancellationToken,
) -> Result<LightPlan, Error> {
    let manifest = release.manifest();
    let all: Vec<usize> = (0..manifest.patches.len()).collect();
    let progress = ProgressSink::latest().0;
    // Read before the fetch, so the review counts only what must download.
    let held = artifacts::held(cache, release, cancel)?;
    let fetched = artifacts::fetch_some(
        cache,
        transport,
        release,
        false,
        &all,
        &SeedSourceHandle::download(),
        cancel,
        &progress,
    )?;
    let mut zips = Vec::with_capacity(all.len());
    for (path, patch) in fetched.patches.into_iter().zip(&manifest.patches) {
        let Some(path) = path else {
            return Err(rejection::reject(
                Error::InvalidArtifact,
                Rejection::new("artifact_missing").entry(format!("patch:{}", patch.id)),
            ));
        };
        zips.push(path);
    }
    let mut summaries = Vec::with_capacity(zips.len());
    for (zip, patch) in zips.iter().zip(&manifest.patches) {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        // The blobs verified against the signed manifest, so a failure here is an
        // unreadable, invalid or unsafe archive, not a bad download.
        let summary = patch_summary::summarize(zip, patch).map_err(|error| {
            let label = format!("patch:{}", patch.id);
            match error {
                patch_summary::PatchSummaryError::Path(_) => rejection::reject(
                    Error::UnsupportedLayout,
                    Rejection::new("patch_layout_unsupported").entry(label),
                ),
                patch_summary::PatchSummaryError::Io(_) => rejection::reject(
                    Error::InvalidArtifact,
                    Rejection::new("patch_archive_unsupported")
                        .entry(label)
                        .found("io"),
                ),
                patch_summary::PatchSummaryError::Zip(_) => rejection::reject(
                    Error::InvalidArtifact,
                    Rejection::new("patch_archive_unsupported")
                        .entry(label)
                        .found("zip"),
                ),
                patch_summary::PatchSummaryError::Recipe(_) => rejection::reject(
                    Error::InvalidArtifact,
                    Rejection::new("patch_archive_unsupported")
                        .entry(label)
                        .found("recipe"),
                ),
            }
        })?;
        summaries.push(summary);
    }
    let folder = folder_hashes(scan);
    let pending = patch_summary::pending(&summaries, |path| have(&folder, path));
    let download_bytes = manifest
        .patches
        .iter()
        .filter(|patch| !held.contains(&patch.sha256))
        .map(|patch| patch.size)
        .sum();
    planned_event(summaries.len(), pending.len(), download_bytes);
    Ok(LightPlan {
        summaries,
        zips,
        pending,
        download_bytes,
    })
}

/// The folder's lower-hex hashes keyed by lower-case path, so lookups are case-blind.
fn folder_hashes(scan: &inventory::Index) -> BTreeMap<String, String> {
    scan.iter()
        .map(|(name, entry)| {
            let hex: String = entry.sha256.iter().map(|b| format!("{b:02x}")).collect();
            (name.to_ascii_lowercase(), hex)
        })
        .collect()
}

/// The folder's hash at `path`, or at the stock spelling when `path` is a placed
/// bundled cooked file the folder still holds under its stock name.
fn have(folder: &BTreeMap<String, String>, path: &str) -> Option<String> {
    folder
        .get(&path.to_ascii_lowercase())
        .or_else(|| folder.get(&stock_spelling(path)?.to_ascii_lowercase()))
        .cloned()
}

/// The reverse of `bundled_cooked_alias`: the stock spelling of a placed path, or
/// `None` when `path` is not under the placed tier.
pub(super) fn stock_spelling(path: &str) -> Option<String> {
    let head = path.get(..ALIAS.len())?;
    if !head.eq_ignore_ascii_case(ALIAS) {
        return None;
    }
    let stock = format!("{STOCK}{}", &path[ALIAS.len()..]);
    install_layout::bundled_cooked_alias(&stock)
        .is_some_and(|alias| alias.eq_ignore_ascii_case(path))
        .then_some(stock)
}

/// Info: one per plan. Counts only, so no path leaves the folder.
fn planned_event(total: usize, pending: usize, download_bytes: u64) {
    tracing::info!(
        target: "adoption",
        event = "adoption_light_planned",
        patches_total = total as u64,
        patches_current = (total - pending) as u64,
        patches_pending = pending as u64,
        download_bytes,
        "adoption: {pending} of {total} patches are not in the folder yet; a light adoption downloads {download_bytes} bytes"
    );
}
