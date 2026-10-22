//! The tolerant replay of a light adoption (LA-10). A light adoption has no
//! stock seed: each pending patch runs in a scratch tree that holds only the
//! folder's own copies of the files that patch reads or writes. An op whose
//! input is not what the patch expects is skipped and recorded
//! (`IncompletePatch`), never an error (D-LA9). What differs from the folder
//! afterwards becomes the replacement store, as in [`super::replay`].
use super::inventory;
use super::light_plan::{stock_spelling, LightPlan};
use super::*;
use crate::unpack::UnpackSink;
use cimmeria_launcher_core::manifest::PatchRoot;
use cimmeria_launcher_core::patch_summary::SGW_GAME_DIR;
use cimmeria_launcher_core::state::{IncompletePatch, SkippedTarget};
use cimmeria_patchset::ApplyOptions;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What the tolerant replay produced.
#[derive(Debug, Default)]
pub(super) struct LightReplay {
    /// Files whose replayed bytes differ from the folder's, stored under
    /// `<prep>/replacements`.
    pub replacements: Vec<Replacement>,
    /// Patches that ran with at least one skipped op.
    pub incomplete: Vec<IncompletePatch>,
    /// `(from, to)` renames of the cooked-data tier: the folder holds the file,
    /// byte for byte, under the stock spelling `from`, and the replayed tree
    /// places it at `to`. Pure renames only; a changed file is a replacement.
    pub moves: Vec<(String, String)>,
}

/// Runs every pending patch of `plan` over a scratch copy of the files it
/// touches, and stores what changed under `<prep>/replacements`. The scratch
/// tree is removed on every outcome. Only cancellation, an unreadable folder
/// file or an unusable archive is an error: a changed input skips its op.
pub(super) fn replay(
    plan: &LightPlan,
    release: &VerifiedRelease,
    folder: &Path,
    scan: &inventory::Index,
    prep: &Path,
    cancel: &CancellationToken,
    sink: &UnpackSink,
) -> Result<LightReplay, Error> {
    let scratch = prep.join("scratch");
    std::fs::create_dir_all(&scratch)?;
    let outcome = run(
        plan,
        release,
        (folder, scan),
        (prep, &scratch),
        cancel,
        sink,
    );
    // Best effort: the scratch tree holds only copies.
    let _ = std::fs::remove_dir_all(&scratch);
    match &outcome {
        Ok(done) => replayed_event(plan.pending.len(), done),
        // A player cancel is not a failure and logs nothing.
        Err(Error::Cancelled) => {}
        Err(error) => {
            tracing::warn!(target: "adoption", event = "adoption_light_replay_failed", code = rejection::code(error), "light adoption replay failed; the replacement files were not built");
        }
    }
    outcome
}

fn run(
    plan: &LightPlan,
    release: &VerifiedRelease,
    (folder, scan): (&Path, &inventory::Index),
    (prep, scratch): (&Path, &Path),
    cancel: &CancellationToken,
    sink: &UnpackSink,
) -> Result<LightReplay, Error> {
    let manifest = release.manifest();
    let in_folder: BTreeMap<String, &String> = scan
        .keys()
        .map(|name| (name.to_ascii_lowercase(), name))
        .collect();
    let mut present: BTreeMap<String, String> = BTreeMap::new();
    let mut incomplete = Vec::new();
    for &k in &plan.pending {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let (Some(summary), Some(zip), Some(patch)) = (
            plan.summaries.get(k),
            plan.zips.get(k),
            manifest.patches.get(k),
        ) else {
            return Err(Error::InvalidArtifact);
        };
        let label = format!("patch:{}", patch.id);
        for pinned in summary.sources.iter().chain(&summary.outputs) {
            stage(
                &pinned.path,
                (folder, scan, &in_folder),
                (scratch, &mut present),
                cancel,
            )?;
        }
        super::reference::strict_zip(zip).map_err(|e| archive_refused(&label, e))?;
        let (dest, prefix) = match patch.root {
            PatchRoot::InstallDir => (scratch.to_path_buf(), String::new()),
            PatchRoot::SgwGame => (
                inventory::native(scratch, SGW_GAME_DIR),
                format!("{SGW_GAME_DIR}/"),
            ),
        };
        let options = ApplyOptions {
            tolerate_ops: true,
            only_targets: None,
        };
        let skipped = crate::unpack::unpack_with(zip, &dest, sink, &options).map_err(|_| {
            if cancel.is_cancelled() {
                Error::Cancelled
            } else {
                archive_refused(&label, Error::InvalidArtifact)
            }
        })?;
        for output in &summary.outputs {
            if inventory::native(scratch, &output.path).is_file() {
                present
                    .entry(output.path.to_ascii_lowercase())
                    .or_insert_with(|| output.path.clone());
            }
        }
        if !skipped.is_empty() {
            incomplete.push(IncompletePatch {
                state_key: summary.state_key.clone(),
                skipped: skipped
                    .iter()
                    .map(|op| SkippedTarget {
                        path: format!("{prefix}{}", op.target.replace('\\', "/")),
                        reason: op.reason.code().to_string(),
                    })
                    .collect(),
            });
        }
    }
    let (replacements, moves) = collect(scan, &in_folder, (prep, scratch), cancel)?;
    Ok(LightReplay {
        replacements,
        incomplete,
        moves,
    })
}

/// Copies the folder's file for `path` into scratch, unless scratch already
/// holds one (an earlier patch wrote it, or an earlier read staged it). The
/// folder may hold it under the stock cooked-data spelling. A file the folder
/// lacks is not copied.
fn stage(
    path: &str,
    (folder, scan, in_folder): (&Path, &inventory::Index, &BTreeMap<String, &String>),
    (scratch, present): (&Path, &mut BTreeMap<String, String>),
    cancel: &CancellationToken,
) -> Result<(), Error> {
    let lower = path.to_ascii_lowercase();
    if present
        .get(&lower)
        .is_some_and(|name| inventory::native(scratch, name).is_file())
    {
        return Ok(());
    }
    let direct = in_folder.get(&lower).copied();
    let name = direct.or_else(|| {
        let stock = stock_spelling(path)?;
        in_folder.get(&stock.to_ascii_lowercase()).copied()
    });
    let Some(name) = name else {
        return Ok(());
    };
    // The folder's own spelling when it holds the file; the placed cooked-data
    // spelling (`path`) only for a stock-named file that moves.
    let spelling = if direct.is_some() {
        name.as_str()
    } else {
        path
    };
    let dest = inventory::native(scratch, spelling);
    match std::fs::remove_file(&dest) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    inventory::copy(&inventory::native(folder, name), &dest, &scan[name], cancel)?;
    present.insert(lower, spelling.to_string());
    Ok(())
}

/// Every scratch file that differs from the folder becomes a stored
/// replacement; one the folder holds byte for byte under the stock cooked-data
/// spelling is a move instead.
fn collect(
    scan: &inventory::Index,
    in_folder: &BTreeMap<String, &String>,
    (prep, scratch): (&Path, &Path),
    cancel: &CancellationToken,
) -> Result<(Vec<Replacement>, Vec<(String, String)>), Error> {
    let built = inventory::scan(scratch, cancel)?;
    let store: PathBuf = prep.join("replacements");
    let mut replacements = Vec::new();
    let mut moves = Vec::new();
    for (name, entry) in &built {
        let held = |key: &str| in_folder.get(key).map(|folder_name| &scan[*folder_name]);
        let lower = name.to_ascii_lowercase();
        let same = |held: &inventory::Entry| held.sha256 == entry.sha256 && held.size == entry.size;
        if held(&lower).is_some_and(same) {
            continue;
        }
        if held(&lower).is_none() {
            let stock = stock_spelling(name).map(|s| s.to_ascii_lowercase());
            if let Some((folder_name, _)) = stock
                .as_deref()
                .and_then(|s| in_folder.get(s).map(|n| (*n, &scan[*n])))
                .filter(|(_, held)| same(held))
            {
                moves.push((folder_name.clone(), name.clone()));
                continue;
            }
        }
        let dest = inventory::native(&store, name);
        inventory::copy(&inventory::native(scratch, name), &dest, entry, cancel)?;
        replacements.push(Replacement {
            path: name.clone(),
            size: entry.size,
            sha256: entry.sha256,
            reason: Classification::NeedsPatching,
        });
    }
    Ok((replacements, moves))
}

/// The refusal for a patch archive this replay cannot unpack. `found` is a
/// closed code, never the error's text.
fn archive_refused(label: &str, error: Error) -> Error {
    let found = rejection::code(&error);
    rejection::reject(
        error,
        Rejection::new("patch_archive_unsupported")
            .entry(label)
            .found(found),
    )
}

/// Info: one per replay. Counts only, so no path leaves the folder.
fn replayed_event(patches: usize, done: &LightReplay) {
    let ops_skipped: usize = done.incomplete.iter().map(|p| p.skipped.len()).sum();
    tracing::info!(
        target: "adoption",
        event = "adoption_light_replayed",
        patches_replayed = patches as u64,
        replacements = done.replacements.len() as u64,
        ops_skipped = ops_skipped as u64,
        patches_incomplete = done.incomplete.len() as u64,
        "adoption: replayed {patches} patches; {} files differ, {ops_skipped} ops were skipped",
        done.replacements.len()
    );
}

#[cfg(test)]
#[path = "light_replay_tests.rs"]
pub(super) mod tests;
