//! Light adoption's comparison (LA-11, D-LA9). There is no stock seed to verify
//! against: the folder's own files are the base, pending patches replay over them
//! tolerantly, and the published index is only compared. Nothing here blocks on a
//! changed input; only an empty folder is refused (`no_reusable_files`).
//! A folder with only the stock cooked-data tier is reviewed as placed
//! (`SourceCache.en-us`), as a fresh install places it; the moves rename the directory.
use super::in_place::SETUP_FILES;
use super::light_order::capped_index_files;
use super::verification::{published_digest, Compared, Tree};
use super::*;
use crate::client_setup::{aslr, login_servers, stock_case};
use crate::{
    install_progress::{ProgressReporter, ProgressSink},
    unpack::UnpackSink,
};
use cimmeria_launcher_core::install_layout::bundled_cooked_alias;
use cimmeria_launcher_core::state::IncompletePatch;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// What a light preview keeps beyond the shared `Preview` fields.
#[derive(Debug, Clone)]
pub(super) struct Light {
    /// The patches replayed with skipped ops; admission writes them into the plan.
    pub incomplete: Vec<IncompletePatch>,
    pub summary: LightSummary,
}

impl Preview {
    /// A light preview is checked lightly; every other adoption verifies each file.
    pub(super) fn file_check(&self) -> FileCheck {
        match self.light {
            Some(_) => FileCheck::Light,
            None => FileCheck::Full,
        }
    }

    pub(super) fn incomplete_patches(&self) -> Vec<IncompletePatch> {
        self.light
            .as_ref()
            .map(|light| light.incomplete.clone())
            .unwrap_or_default()
    }

    /// The review's light-only figures; `None` for the other modes.
    pub fn light_summary(&self) -> Option<&LightSummary> {
        self.light.as_ref().map(|light| &light.summary)
    }
}

/// A folder scan that reports hashed bytes and honours `cancel`.
pub(super) fn scan_reporting<'a>(
    progress: &'a ProgressSink,
    cancel: &'a CancellationToken,
) -> impl Fn(&Path) -> Result<inventory::Index, Error> + 'a {
    move |root| {
        inventory::scan_with_progress(root, cancel, &|done, total| {
            progress.report(crate::install::Progress::Extracting {
                label: "Scanning".into(),
                current: done as usize,
                total: total as usize,
                filename: String::new(),
            })
        })
    }
}

fn no_reusable(of: usize) -> Error {
    rejection::reject(
        Error::NoReusableFiles,
        Rejection::new("no_reusable_files")
            .expected("at least 1 matching file")
            .found(format!("0 of {of}")),
    )
}

/// The light counterpart of `verification::reference_and_comparison`: scan (done by
/// the caller), plan, compare with the published index, replay.
#[allow(clippy::too_many_arguments)]
pub(super) fn light_comparison(
    state: &Arc<Mutex<DesktopState>>,
    request: &PreviewRequest,
    transport: Option<artifacts::Transport>,
    preparation: &preparation::Ownership,
    folder: &Path,
    scanned: &inventory::Index,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
    progress: ProgressSink,
) -> Result<(Compared, Light), Error> {
    if scanned.is_empty() {
        return Err(no_reusable(0));
    }
    // An invalid list is refused before any file moves: the setup would not write it.
    login_servers::validate(servers).map_err(|_| {
        rejection::reject(
            Error::UnsupportedConfiguration,
            Rejection::new("login_servers_invalid").expected("a valid login server list"),
        )
    })?;
    let root = state
        .lock()
        .map_err(|_| StorageError::Io)?
        .state_root()
        .to_path_buf();
    let transport = match transport {
        Some(transport) => transport,
        None => artifacts::Transport::production()?,
    };
    let release = &request.release;
    let plan = light_plan::plan(&transport, release, &root, scanned, cancel)?;
    // The comparison never blocks; only a cancel comes back as an error.
    let compared = index_compare::compare_to_published(
        &root, release, &transport, scanned, cancel, &progress,
    )?;
    let prep = &preparation.record.directory;
    let sink = UnpackSink {
        progress,
        label: "Adoption replay".into(),
        cancel: cancel.clone(),
    };
    let replayed = light_replay::replay(&plan, release, folder, scanned, prep, cancel, &sink)?;

    let mut replacements = replayed.replacements;
    let store = prep.join("replacements");
    let mut tree = Tree::new();
    let mut files = Vec::new();
    // The rule `place_bundled_cooked_data` applies: a stock tier with no placed tier
    // beside it is placed. The moves then rename the directory once (`in_place`).
    let place = scanned.keys().any(|k| bundled_cooked_alias(k).is_some())
        && !scanned.keys().any(|k| {
            k.to_ascii_lowercase()
                .starts_with("working/sgwgame/sourcecache.en-us/")
        });
    let placed_name = |name: &str| -> String {
        // The slices are ASCII-safe: `bundled_cooked_alias` matched those bytes.
        if place && bundled_cooked_alias(name).is_some() {
            format!("{}SourceCache.en-us/{}", &name[..16], &name[28..])
        } else {
            name.to_string()
        }
    };
    let by_placed: BTreeMap<String, &String> = scanned
        .keys()
        .map(|k| (placed_name(k).to_ascii_lowercase(), k))
        .collect();
    // A cooked file the folder holds under the stock spelling moves to its placed
    // path as a stored copy; in the both-tiers case the original stays where the
    // player left it. When the tier is placed (place), the stock directory is
    // renamed at adoption, so no stock-spelled original remains and no per-file
    // move is needed.
    let placed = |to: &str| scanned.contains_key(to) || replacements.iter().any(|r| r.path == to);
    let moves: Vec<&(String, String)> = if place {
        Vec::new()
    } else {
        replayed
            .moves
            .iter()
            .filter(|(from, to)| scanned.contains_key(from) && !placed(to))
            .collect()
    };
    let moved: BTreeSet<&str> = moves.iter().map(|(from, _)| from.as_str()).collect();
    let replaced: BTreeSet<String> = replacements
        .iter()
        .map(|r| r.path.to_ascii_lowercase())
        .collect();
    for (name, entry) in scanned {
        let path = placed_name(name);
        if moved.contains(name.as_str()) || replaced.contains(&path.to_ascii_lowercase()) {
            continue;
        }
        tree.insert(path.clone(), (entry.size, entry.sha256));
        let classification = if path == *name {
            Classification::Matched
        } else {
            Classification::KnownTransform
        };
        files.push(Difference {
            path,
            source_path: Some(name.clone()),
            classification,
        });
    }
    for r in &replacements {
        tree.insert(r.path.clone(), (r.size, r.sha256));
        let source_path = if scanned.contains_key(&r.path) {
            Some(r.path.clone())
        } else if place {
            by_placed
                .get(&r.path.to_ascii_lowercase())
                .map(|name| (*name).clone())
        } else {
            None
        };
        files.push(Difference {
            path: r.path.clone(),
            source_path,
            classification: Classification::NeedsPatching,
        });
    }
    let mut stored = Vec::new();
    for (from, to) in moves {
        let entry = &scanned[from];
        inventory::copy(
            &inventory::native(folder, from),
            &inventory::native(&store, to),
            entry,
            cancel,
        )?;
        tree.insert(to.clone(), (entry.size, entry.sha256));
        files.push(Difference {
            path: to.clone(),
            source_path: Some(from.clone()),
            classification: Classification::NeedsPatching,
        });
        stored.push(Replacement {
            path: to.clone(),
            size: entry.size,
            sha256: entry.sha256,
            reason: Classification::NeedsPatching,
        });
    }
    replacements.extend(stored);

    restore_stock_case(&mut tree);
    apply_setup(&mut tree, &store, folder, prep, servers, cancel)?;
    let expected_digest = published_digest(&tree, release, &replayed.incomplete)?;
    let expected_paths: Vec<String> = tree.keys().cloned().collect();

    let manifest = release.manifest();
    let index_state = compared.state;
    let (index_counts, index_all) = match compared.comparison {
        Some(comparison) => (comparison.counts, comparison.files),
        None => (BTreeMap::new(), Vec::new()),
    };
    let skipped: Vec<LightSkip> = replayed
        .incomplete
        .iter()
        .flat_map(|patch| {
            patch.skipped.iter().map(|target| LightSkip {
                path: target.path.clone(),
                patch: patch.state_key.clone(),
                reason: target.reason.clone(),
            })
        })
        .collect();
    let summary = LightSummary {
        patches_total: plan.summaries.len() as u32,
        patches_current: (plan.summaries.len() - plan.pending.len()) as u32,
        patches_pending: plan.pending.len() as u32,
        download_bytes: plan.download_bytes,
        skipped_total: skipped.len() as u32,
        skipped: skipped.into_iter().take(LIGHT_LIST_CAP).collect(),
        index_state,
        index_counts,
        index_files_total: index_all.len() as u32,
        index_files: capped_index_files(index_all),
    };
    let download = DownloadEstimate {
        seed: false,
        patches: plan
            .pending
            .iter()
            .map(|&i| manifest.patches[i].state_key())
            .collect(),
        bytes: plan.download_bytes,
        local_seed: false,
    };
    let file_index_sha256 = (index_state == index_compare::IndexState::Compared)
        .then(|| {
            manifest
                .file_index
                .as_ref()
                .map(|reference| reference.sha256.to_ascii_lowercase())
        })
        .flatten();
    let light = Light {
        incomplete: replayed.incomplete,
        summary,
    };
    let compared = Compared {
        reference: None,
        expected_digest,
        expected_paths,
        // The same tree that gave `expected_digest`, after setup (C-LF3).
        expected_tree: Some(tree),
        files,
        download,
        replacements,
        // A light adoption has no index to say the login file differs, and no
        // backup: the launcher's setup rewrites it after the move (D-AI13 is full-check).
        login_replaced: false,
        file_index_sha256,
        index_fallback: None,
    };
    Ok((compared, light))
}

/// `client_setup::prepare` renames a stock-named file whose case differs, keeping
/// the directory's spelling; the digest sees the tree after that rename.
fn restore_stock_case(tree: &mut Tree) {
    for target in stock_case::PATCH_TARGETS {
        let name = target.rsplit('/').next().unwrap_or(target);
        let hits: Vec<String> = tree
            .keys()
            .filter(|key| key.eq_ignore_ascii_case(target))
            .cloned()
            .collect();
        if hits.iter().any(|key| key.rsplit('/').next() == Some(name)) {
            continue;
        }
        if let Some(key) = hits.into_iter().next() {
            let renamed = match key.rsplit_once('/') {
                Some((dir, _)) => format!("{dir}/{name}"),
                None => name.to_string(),
            };
            if let Some(value) = tree.remove(&key) {
                tree.insert(renamed, value);
            }
        }
    }
}

/// The two files the launcher's setup writes: `LoginInternal.lua` (always rendered)
/// and `SGW.exe` with ASLR off. The ASLR bytes come from a probe copy, so the
/// digest is the one `prepare` will produce.
fn apply_setup(
    tree: &mut Tree,
    store: &Path,
    folder: &Path,
    prep: &Path,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
) -> Result<(), Error> {
    let [exe, login, _] = SETUP_FILES;
    let spelled = |tree: &Tree, wanted: &str| {
        tree.keys()
            .find(|key| key.eq_ignore_ascii_case(wanted))
            .cloned()
    };
    let lua = login_servers::render(servers);
    let key = spelled(tree, login).unwrap_or_else(|| login.to_string());
    tree.insert(
        key,
        (lua.len() as u64, Sha256::digest(lua.as_bytes()).into()),
    );
    let Some(key) = spelled(tree, exe) else {
        return Ok(());
    };
    let stored = inventory::native(store, &key);
    let source = if stored.is_file() {
        stored
    } else {
        inventory::native(folder, &key)
    };
    let probe = prep.join("probe").join("SGW.exe");
    if let Some(parent) = probe.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&source, &probe)?;
    // `finish` runs the same step and would fail the adoption on a bad executable.
    let outcome = aslr::disable(&probe);
    let entry = outcome.is_ok().then(|| inventory::hash(&probe, cancel));
    let _ = std::fs::remove_file(&probe);
    match entry {
        Some(entry) => {
            let entry = entry?;
            tree.insert(key, (entry.size, entry.sha256));
            Ok(())
        }
        None => Err(rejection::reject(
            Error::VerificationFailed,
            Rejection::new("setup_rule_invalid").entry(&key),
        )),
    }
}
