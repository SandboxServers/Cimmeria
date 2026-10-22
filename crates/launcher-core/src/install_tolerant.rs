//! Tolerant installs: recording the ops a patch skipped, and retrying them on
//! a later run (LA-04). Only the `install_all_tolerant` entry points reach
//! this module. The strict paths never record or retry, so their ledger
//! and files stay as they were.

use std::collections::BTreeSet;
use std::path::Path;

use cimmeria_patchset::SkippedOp;
use tracing::{info, warn};

use super::{apply_patch, InstallContext, InstallError, InstallOptions};
use crate::manifest::PatchEntry;
use crate::patch_dest::patch_dest;
use crate::state::{InstalledState, SkippedTarget};

/// The client-root relative folder `dest` is in, `/` separated. Empty for
/// the install dir itself.
fn client_prefix(install_dir: &Path, dest: &Path) -> Result<String, InstallError> {
    let rel = dest.strip_prefix(install_dir).map_err(|_| {
        InstallError::Io(std::io::Error::other("patch dest outside the install dir"))
    })?;
    Ok(rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}

/// The client-root relative ledger path of `target`, a recipe target
/// relative to the patch's dest.
fn ledger_path(prefix: &str, target: &str) -> String {
    if prefix.is_empty() {
        target.to_string()
    } else {
        format!("{prefix}/{target}")
    }
}

/// The lower-case target, relative to the patch's dest, of a ledger `path`.
fn relative_target(prefix: &str, path: &str) -> String {
    let rel = if prefix.is_empty() {
        path
    } else {
        path.strip_prefix(prefix)
            .and_then(|p| p.strip_prefix('/'))
            .unwrap_or(path)
    };
    rel.to_lowercase()
}

/// The ledger entries for the ops `skipped` of a patch whose dest is in `prefix`.
fn to_ledger(prefix: &str, skipped: &[SkippedOp]) -> Vec<SkippedTarget> {
    skipped
        .iter()
        .map(|op| SkippedTarget {
            path: ledger_path(prefix, &op.target),
            reason: op.reason.code().to_string(),
        })
        .collect()
}

/// Record the skipped ops of `patch`, which applied. An empty list removes
/// the patch's incomplete entry.
pub(super) fn record_skipped(
    ctx: &InstallContext<'_>,
    patch: &PatchEntry,
    state: &mut InstalledState,
    skipped: &[SkippedOp],
) -> Result<(), InstallError> {
    let dest = patch_dest(ctx.install_dir, patch)?;
    let prefix = client_prefix(ctx.install_dir, &dest)?;
    let list = to_ledger(&prefix, skipped);
    if !list.is_empty() {
        info!(target: "launcher.install", event = "patch_incomplete", patch = %format!("patch:{}", patch.id), ops_skipped = list.len(), "patch applied with skipped ops");
    }
    state.record_incomplete(&patch.state_key(), list);
    Ok(())
}

/// Retry the skipped ops of every incomplete patch, in ledger order, before
/// the manifest patches are applied. An entry whose patch has left the
/// manifest is kept. A retry that fails keeps its entry for the next run.
pub(super) async fn retry_incomplete(
    ctx: &InstallContext<'_>,
    state: &mut InstalledState,
) -> Result<(), InstallError> {
    for incomplete in state.incomplete_patches.clone() {
        let Some(patch) = ctx
            .manifest
            .patches
            .iter()
            .find(|p| p.state_key() == incomplete.state_key)
        else {
            continue;
        };
        match retry_one(ctx, patch, &incomplete.skipped).await {
            Ok(list) => {
                state.record_incomplete(&incomplete.state_key, list);
                state.save(ctx.install_dir)?;
            }
            Err(InstallError::Cancelled) => return Err(InstallError::Cancelled),
            Err(e) => {
                warn!(target: "launcher.install", event = "patch_retry_failed", patch = %format!("patch:{}", patch.id), reason = e.code(), "incomplete patch kept for the next run");
            }
        }
    }
    Ok(())
}

/// Re-apply only the `skipped` targets of `patch` and return what still skips.
async fn retry_one(
    ctx: &InstallContext<'_>,
    patch: &PatchEntry,
    skipped: &[SkippedTarget],
) -> Result<Vec<SkippedTarget>, InstallError> {
    let dest = patch_dest(ctx.install_dir, patch)?;
    let prefix = client_prefix(ctx.install_dir, &dest)?;
    let only: BTreeSet<String> = skipped
        .iter()
        .map(|s| relative_target(&prefix, &s.path))
        .collect();
    let still = apply_patch(ctx, patch, InstallOptions::TOLERANT, Some(only)).await?;
    Ok(to_ledger(&prefix, &still))
}
