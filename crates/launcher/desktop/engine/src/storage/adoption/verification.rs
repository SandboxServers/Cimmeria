//! Classification compares the adopted folder with the per-file index:
//! [`acquire_index`] yields the signed index, or the one built locally. A built
//! index also reconstructs the reference for the moves; a published one does not,
//! so its adoption fetches only what the replay needs. The game directory is
//! `Folder.root` for a folder source or the imported game folder for a legacy
//! one; only a legacy source has an import to verify.
use super::setup_digest::setup_bytes;
use super::*;
use crate::{install_progress::ProgressSink, unpack::UnpackSink};
use cimmeria_launcher_core::file_index::{FileIndex, Origin, SetupRule};
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// A file's size and lower-case SHA-256 hex.
type Bytes = (u64, String);

/// The game-root ledger the adopted tree carries; synthesized, never a release file.
const LEDGER: &str = "launcher-installed.json";

#[allow(clippy::too_many_arguments)]
pub(super) fn reference_and_comparison(
    state: &Arc<Mutex<DesktopState>>,
    request: &PreviewRequest,
    backend: Backend,
    transport: Option<artifacts::Transport>,
    preparation: &mut preparation::Ownership,
    adopted: AdoptionSourceRef<'_>,
    mode: AdoptionMode,
    scanned: &inventory::Index,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
    progress: crate::install_progress::ProgressSink,
    context: crate::phase_timing::Context,
) -> Result<Compared, Error> {
    let (folder, legacy) = match adopted {
        AdoptionSourceRef::Legacy(imported) => (&imported.source.game_directory, Some(imported)),
        AdoptionSourceRef::Folder(picked) => (&picked.root, None),
    };
    if mode == AdoptionMode::Copy {
        super::comparison::copy_layout(folder, scanned)?;
    }
    let root = state
        .lock()
        .map_err(|_| StorageError::Io)?
        .state_root()
        .to_path_buf();
    let transport = match transport {
        Some(transport) => transport,
        None => artifacts::Transport::production()?,
    };
    // The chosen local archive, if any, is the seed a fetch of this release uses.
    let seed_source = state
        .lock()
        .map_err(|_| StorageError::Io)?
        .seed_source_for(&request.release);
    // The index comes first: a published one needs no seed or patch for the reference.
    let (index, origin) = context.measure(crate::phase_timing::Phase::AdoptionIndex, || {
        acquire_index(
            &root,
            &request.release,
            &transport,
            &seed_source,
            request.artifacts.as_ref(),
            cancel,
            &progress,
        )
    })?;
    let reference = match origin {
        IndexOrigin::Published => None,
        IndexOrigin::Built(_) => Some(reconstructed(
            state,
            request,
            backend,
            &transport,
            &seed_source,
            preparation,
            servers,
            cancel,
            &progress,
        )?),
    };
    // Read after the index is known, so the estimate counts only what the replay
    // must still download. A built index fetched every blob it used, so nothing
    // is left to download and no held set is read.
    let held = match origin {
        IndexOrigin::Published => Some(artifacts::held(&root, &request.release, cancel)?),
        IndexOrigin::Built(_) => None,
    };
    let is_held = |sha: &str| held.as_ref().is_none_or(|held| held.contains(sha));
    let (expected_digest, expected_paths): ([u8; 32], Vec<String>) = match &reference {
        Some(reference) => (
            inventory::content_digest(&reference.index)?,
            reference.index.keys().cloned().collect(),
        ),
        None => {
            let tree = expected_tree(&index, servers)?;
            (
                published_digest(&tree, &request.release, &[])?,
                tree.keys().cloned().collect(),
            )
        }
    };
    let files = classify(scanned, &index, servers);
    let login_replaced = replaces_login(&index, scanned, servers);
    if !files.iter().any(|d| {
        matches!(
            d.classification,
            Classification::Matched | Classification::KnownTransform
        )
    }) {
        return Err(rejection::reject(
            Error::NoReusableFiles,
            Rejection::new("no_reusable_files")
                .expected("at least 1 matching file")
                .found(format!("0 of {}", files.len())),
        ));
    }
    if timing::scan(
        context,
        crate::phase_timing::Phase::AdoptionReviewScan,
        || inventory::scan(folder, cancel),
    )? != *scanned
    {
        return Err(rejection::reject(
            Error::SourceChanged,
            Rejection::new("folder_changed_during_check"),
        ));
    }
    if let Some(imported) = legacy {
        source::verify_import(imported)?;
    }
    let plan = replay::plan(&index, &files, scanned);
    let estimate = replay::estimate(
        &plan,
        request.release.manifest(),
        seed_source
            .current_for(&request.release.manifest().seed.sha256)
            .is_some(),
        is_held,
    );
    let download = download_for(&origin, estimate);
    let mut replacements = Vec::new();
    if !plan.targets.is_empty() {
        // Local artifacts are hash-checked in place; a signed fetch downloads only
        // the blobs the plan needs.
        let fetched = match &request.artifacts {
            Some(local) => artifacts::local_some(
                local,
                &request.release,
                plan.needs_seed,
                &plan.patches,
                cancel,
            )?,
            None => artifacts::fetch_some(
                &root,
                &transport,
                &request.release,
                plan.needs_seed,
                &plan.patches,
                &seed_source,
                cancel,
                &progress,
            )?,
        };
        let sink = UnpackSink {
            progress: progress.clone(),
            label: "Adoption replay".into(),
            cancel: cancel.clone(),
        };
        let stored = replay::replay(
            &plan,
            &index,
            &request.release,
            &fetched,
            folder,
            scanned,
            &preparation.record.directory,
            cancel,
            &sink,
        )?;
        for (path, dest) in stored {
            let entry = inventory::hash(&dest, cancel)?;
            let reason = files
                .iter()
                .find(|d| d.path == path)
                .map(|d| d.classification)
                .ok_or_else(|| {
                    rejection::reject(
                        Error::InvalidArtifact,
                        Rejection::new("file_unclassified").entry(&path),
                    )
                })?;
            replacements.push(Replacement {
                path,
                size: entry.size,
                sha256: entry.sha256,
                reason,
            });
        }
    }
    // The published index is the signed blob the manifest names, so its digest is the
    // reference's. A built index is not a blob, so it records no digest.
    let file_index_sha256 = match origin {
        IndexOrigin::Published => request
            .release
            .manifest()
            .file_index
            .as_ref()
            .map(|reference| reference.sha256.to_ascii_lowercase()),
        IndexOrigin::Built(_) => None,
    };
    let index_fallback = match origin {
        IndexOrigin::Published => None,
        IndexOrigin::Built(fallback) => Some(fallback),
    };
    Ok(Compared {
        reference,
        expected_digest,
        expected_paths,
        expected_tree: None,
        files,
        download,
        replacements,
        login_replaced,
        file_index_sha256,
        index_fallback,
    })
}

/// The reference a built index needs: the seed and patches are fetched (a stored
/// blob is reused), then the release is rebuilt and set up. Blocking, like
/// [`artifacts::fetch`].
#[allow(clippy::too_many_arguments)]
fn reconstructed(
    state: &Arc<Mutex<DesktopState>>,
    request: &PreviewRequest,
    backend: Backend,
    transport: &artifacts::Transport,
    seed_source: &SeedSourceHandle,
    preparation: &mut preparation::Ownership,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<reference::Reference, Error> {
    let root = state
        .lock()
        .map_err(|_| StorageError::Io)?
        .state_root()
        .to_path_buf();
    let fetched;
    let artifacts = match &request.artifacts {
        Some(artifacts) => artifacts,
        None => {
            fetched = artifacts::fetch(
                &root,
                transport,
                &request.release,
                seed_source,
                cancel,
                progress,
            )?;
            &fetched
        }
    };
    let adapter: Option<Box<dyn crate::install::SeedExtractor>> = match backend {
        Backend::Native => None,
        #[cfg(target_os = "macos")]
        Backend::Wine(helper) => {
            preparation.uncertain = true;
            Some(Box::new(
                tokio::runtime::Handle::try_current()
                    .map_err(|_| StorageError::Io)?
                    .block_on(crate::mac_wine::WineSeedExtractor::prepare(
                        state.clone(),
                        preparation.record.id,
                        helper.path().to_path_buf(),
                        cancel.clone(),
                        progress.clone(),
                    ))
                    .map_err(|_| StorageError::PersistenceUncertain)?,
            ))
        }
    };
    // The in-place moves still read the reconstructed bytes of a built index.
    let reference = reference::reconstruct(
        &preparation.record.directory,
        &request.release,
        artifacts,
        servers,
        cancel,
        progress.clone(),
        adapter.as_deref(),
    )?;
    preparation.uncertain = false;
    Ok(reference)
}

/// A tree of release paths, each with its size and SHA-256.
pub(super) type Tree = BTreeMap<String, (u64, [u8; 32])>;

/// The release tree after setup, keyed by release spelling: every path's release
/// bytes, or the setup rule's bytes for `SGW.exe` and `LoginInternal.lua`.
fn expected_tree(
    index: &FileIndex,
    servers: &[crate::client_setup::LoginServer],
) -> Result<Tree, Error> {
    let (setup, _) = expected_prepared(index, servers);
    let mut tree = Tree::new();
    for (key, history) in index.histories() {
        let Some(release) = history.states.last() else {
            continue;
        };
        let bytes = match setup.get(&key) {
            Some((size, sha)) => (*size, setup_bytes(sha, &history.path)?),
            None => (release.size, release.sha256),
        };
        tree.insert(history.path.clone(), bytes);
    }
    // A setup rule can create a file no history names (`LoginInternal.lua` is not
    // in the seed), so the post-setup tree has it too.
    for rule in &index.setup {
        let path = match rule {
            SetupRule::AslrOff { path, .. } | SetupRule::LoginServers { path } => path,
        };
        if tree.keys().any(|known| known.eq_ignore_ascii_case(path)) {
            continue;
        }
        if let Some((size, sha)) = setup.get(&path.to_ascii_lowercase()) {
            tree.insert(path.clone(), (*size, setup_bytes(sha, path)?));
        }
    }
    Ok(tree)
}

/// The digest the post-setup tree of a published index must have: `tree` plus the
/// synthesized ledger, hashed the way [`inventory::content_digest`] hashes a scan.
/// A light adoption passes the patches it applied with skipped ops (D-LA18).
pub(super) fn published_digest(
    tree: &Tree,
    release: &VerifiedRelease,
    incomplete: &[cimmeria_launcher_core::state::IncompletePatch],
) -> Result<[u8; 32], Error> {
    let ledger = serde_json::to_vec_pretty(&reference::ledger(release, incomplete))
        .map_err(|_| StorageError::Corrupt)?;
    let mut all = tree.clone();
    all.insert(
        LEDGER.into(),
        (ledger.len() as u64, Sha256::digest(&ledger).into()),
    );
    let rows: Vec<(&String, u64, [u8; 32])> = all
        .iter()
        .map(|(path, (size, sha))| (path, *size, *sha))
        .collect();
    digest(&rows)
}

/// What a preview learns from the signed reference and the folder: the reference
/// for the moves when the index was built, the digest and paths the adopted tree
/// must have, the per-file review, the download the replay implies, and the files
/// it stored for the adopted tree.
pub(super) struct Compared {
    /// Built indexes only; a published index needs none (AI-11 part c).
    pub reference: Option<reference::Reference>,
    /// The digest the post-setup tree must have (the provenance's reference digest).
    pub expected_digest: [u8; 32],
    /// Every release path of the post-setup tree, in release spelling.
    pub expected_paths: Vec<String>,
    /// The post-setup tree itself, for the light check's per-file diff; `None` otherwise.
    pub expected_tree: Option<Tree>,
    pub files: Vec<Difference>,
    pub download: DownloadEstimate,
    pub replacements: Vec<Replacement>,
    /// `LoginInternal.lua` differs from both its release and prepared bytes (D-AI13).
    pub login_replaced: bool,
    /// The signed index's digest when the index was published; a built index has none.
    pub file_index_sha256: Option<String>,
    /// Why the signed index was not used; `None` when it was published (C15).
    pub index_fallback: Option<super::index_source::IndexFallback>,
}

/// Classifies every path of the index's histories against the folder scan, then
/// the folder files no history names. Names are case-blind; `path` is the
/// release spelling.
pub(super) fn classify(
    scanned: &inventory::Index,
    index: &FileIndex,
    servers: &[crate::client_setup::LoginServer],
) -> Vec<Difference> {
    let (setup, login) = expected_prepared(index, servers);
    let histories = index.histories();
    let folded: BTreeMap<String, &String> = scanned
        .keys()
        .map(|name| (name.to_ascii_lowercase(), name))
        .collect();
    let mut out = Vec::new();
    for (key, history) in &histories {
        let Some(release) = history.states.last() else {
            continue;
        };
        let earlier = &history.states[..history.states.len() - 1];
        let release_bytes = (release.size, hex(&release.sha256));
        let found = folded.get(key).copied();
        let classification = match found {
            None if matches!(history.states[0].origin, Origin::Patch(_)) => {
                Classification::NeedsPatching
            }
            None => Classification::Missing,
            Some(name) => {
                let entry = &scanned[name];
                let bytes = (entry.size, hex(&entry.sha256));
                let prepared = setup
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| release_bytes.clone());
                let spelled = if name == &history.path {
                    Classification::Matched
                } else {
                    Classification::KnownTransform
                };
                if bytes == prepared {
                    spelled
                } else if bytes == release_bytes {
                    // A setup-rule file in its pre-setup form (ASLR still on) is a transform.
                    if setup.contains_key(key) {
                        Classification::KnownTransform
                    } else {
                        spelled
                    }
                } else if login.contains(key) {
                    Classification::KnownTransform
                } else if earlier
                    .iter()
                    .any(|state| (state.size, hex(&state.sha256)) == bytes)
                {
                    Classification::NeedsPatching
                } else {
                    Classification::Modified
                }
            }
        };
        out.push(Difference {
            path: history.path.clone(),
            source_path: found.cloned(),
            classification,
        });
    }
    for name in scanned
        .keys()
        .filter(|name| !histories.contains_key(&name.to_ascii_lowercase()))
    {
        // A setup rule can create a file no history names (`LoginInternal.lua` is
        // not in the seed); in its prepared bytes it is part of the post-setup tree.
        let entry = &scanned[name];
        let prepared = setup
            .get(&name.to_ascii_lowercase())
            .filter(|prepared| **prepared == (entry.size, hex(&entry.sha256)));
        let rule_path = index.setup.iter().find_map(|rule| {
            let path = match rule {
                SetupRule::AslrOff { path, .. } | SetupRule::LoginServers { path } => path,
            };
            path.eq_ignore_ascii_case(name).then_some(path)
        });
        let (path, classification) = match (prepared, rule_path) {
            (Some(_), Some(path)) if path == name => (path.clone(), Classification::Matched),
            (Some(_), Some(path)) => (path.clone(), Classification::KnownTransform),
            _ => (name.clone(), Classification::Extra),
        };
        out.push(Difference {
            path,
            source_path: Some(name.clone()),
            classification,
        });
    }
    out
}

/// The setup rules' expected bytes, keyed by lower-case path: `SGW.exe` with ASLR
/// off and `LoginInternal.lua` as rendered for `servers`. Also the login paths.
fn expected_prepared(
    index: &FileIndex,
    servers: &[crate::client_setup::LoginServer],
) -> (BTreeMap<String, Bytes>, BTreeSet<String>) {
    let mut expected = BTreeMap::new();
    let mut login = BTreeSet::new();
    let rendered = crate::client_setup::login_servers::render(servers).into_bytes();
    for rule in &index.setup {
        match rule {
            SetupRule::AslrOff { path, size, sha256 } => {
                expected.insert(path.to_ascii_lowercase(), (*size, sha256.clone()));
            }
            SetupRule::LoginServers { path } => {
                let key = path.to_ascii_lowercase();
                let sha = hex(Sha256::digest(&rendered).as_slice());
                expected.insert(key.clone(), (rendered.len() as u64, sha));
                login.insert(key);
            }
        }
    }
    (expected, login)
}

/// Whether the folder's `LoginInternal.lua` differs from both its release bytes
/// and its expected prepared bytes (D-AI13). Such a file is a known transform,
/// but the adoption still backs up the original before it replaces it.
fn replaces_login(
    index: &FileIndex,
    scanned: &inventory::Index,
    servers: &[crate::client_setup::LoginServer],
) -> bool {
    let (setup, login) = expected_prepared(index, servers);
    let histories = index.histories();
    scanned.iter().any(|(name, entry)| {
        let key = name.to_ascii_lowercase();
        if !login.contains(&key) {
            return false;
        }
        let bytes = (entry.size, hex(&entry.sha256));
        let release = histories
            .get(&key)
            .and_then(|history| history.states.last())
            .map(|state| (state.size, hex(&state.sha256)));
        setup.get(&key) != Some(&bytes) && release != Some(bytes)
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) use super::index_source::{acquire_index, download_for, IndexOrigin};
