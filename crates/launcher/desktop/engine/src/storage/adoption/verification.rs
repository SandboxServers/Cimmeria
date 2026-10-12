//! Classification compares the adopted folder with the per-file index:
//! [`acquire_index`] yields the signed index, or the one built locally. A built
//! index also reconstructs the reference for the moves; a published one does not,
//! so its adoption fetches only what the replay needs. The game directory is
//! `Folder.root` for a folder source or the imported game folder for a legacy
//! one; only a legacy source has an import to verify.
use super::*;
use crate::{
    install_progress::ProgressSink,
    manifest::{Manifest, PatchEntry},
    unpack::UnpackSink,
};
use cimmeria_launcher_core::file_index::{
    self, FileIndex, FileIndexError, FileIndexRef, Origin, SetupRule, MAX_FILE_INDEX_BYTES,
};
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

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
    let (index, origin) = acquire_index(
        &root,
        &request.release,
        &transport,
        &seed_source,
        request.artifacts.as_ref(),
        cancel,
        &progress,
    )?;
    let reference = match origin {
        IndexOrigin::Published => None,
        IndexOrigin::Built => Some(reconstructed(
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
        IndexOrigin::Built => None,
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
                published_digest(&tree, &request.release)?,
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
        return Err(Error::NoReusableFiles);
    }
    if inventory::scan(folder, cancel)? != *scanned {
        return Err(Error::SourceChanged);
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
                .ok_or(Error::InvalidArtifact)?;
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
        IndexOrigin::Built => None,
    };
    Ok(Compared {
        reference,
        expected_digest,
        expected_paths,
        files,
        download,
        replacements,
        login_replaced,
        file_index_sha256,
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
type Tree = BTreeMap<String, (u64, [u8; 32])>;

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
            Some((size, sha)) => (*size, unhex(sha).ok_or(Error::InvalidArtifact)?),
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
            tree.insert(
                path.clone(),
                (*size, unhex(sha).ok_or(Error::InvalidArtifact)?),
            );
        }
    }
    Ok(tree)
}

/// The digest the post-setup tree of a published index must have: `tree` plus the
/// synthesized ledger, hashed the way [`inventory::content_digest`] hashes a scan.
fn published_digest(tree: &Tree, release: &VerifiedRelease) -> Result<[u8; 32], Error> {
    let ledger = serde_json::to_vec_pretty(&reference::ledger(release))
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

/// Decodes exactly 64 hex digits into a digest; `None` for any other text.
fn unhex(hex: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    (hex.len() == 64).then_some(out)
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
    pub files: Vec<Difference>,
    pub download: DownloadEstimate,
    pub replacements: Vec<Replacement>,
    /// `LoginInternal.lua` differs from both its release and prepared bytes (D-AI13).
    pub login_replaced: bool,
    /// The signed index's digest when the index was published; a built index has none.
    pub file_index_sha256: Option<String>,
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
        out.push(Difference {
            path: name.clone(),
            source_path: Some(name.clone()),
            classification: Classification::Extra,
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

/// Where the per-file index came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IndexOrigin {
    /// The signed blob, verified against the manifest.
    Published,
    /// Built locally from the seed and patches.
    Built,
}

/// The per-file index for `release`: the signed blob when the manifest names one
/// and it verifies, otherwise the index built from the seed and patches. Blocking,
/// like [`artifacts::fetch`]: call it from a retained worker. `prefetched` are the
/// seed and patches already fetched; without them the build fetches its own.
pub(super) fn acquire_index(
    state_root: &Path,
    release: &VerifiedRelease,
    transport: &artifacts::Transport,
    seed_source: &SeedSourceHandle,
    prefetched: Option<&Artifacts>,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<(FileIndex, IndexOrigin), Error> {
    let reason = match &release.manifest().file_index {
        None => "the manifest names no file index".to_string(),
        Some(reference) => {
            match published(state_root, release, transport, reference, cancel, progress)? {
                Ok(index) => return Ok((index, IndexOrigin::Published)),
                Err(reason) => reason,
            }
        }
    };
    tracing::warn!(target: "adoption", reason = %reason, "file index unavailable; building it from the seed and patches");
    let index = built(
        state_root,
        release,
        transport,
        seed_source,
        prefetched,
        cancel,
        progress,
    )?;
    Ok((index, IndexOrigin::Built))
}

/// The download a comparison reports. A built index fetched every blob it used,
/// so nothing is left to download; a chosen local archive is still named.
pub(super) fn download_for(origin: &IndexOrigin, estimate: DownloadEstimate) -> DownloadEstimate {
    match origin {
        IndexOrigin::Published => estimate,
        IndexOrigin::Built => DownloadEstimate {
            seed: false,
            patches: Vec::new(),
            bytes: 0,
            local_seed: estimate.local_seed,
        },
    }
}

/// The signed blob, from the cache when it verifies, else downloaded and cached.
/// The outer error aborts; the inner one is a refusal that falls back to the build.
fn published(
    state_root: &Path,
    release: &VerifiedRelease,
    transport: &artifacts::Transport,
    reference: &FileIndexRef,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<Result<FileIndex, String>, Error> {
    let manifest = release.manifest();
    if reference.size > MAX_FILE_INDEX_BYTES || !is_sha256(&reference.sha256) {
        return Ok(Err(
            "the manifest's file index reference is malformed".into()
        ));
    }
    let name = format!("file-index-{}.json", reference.sha256.to_ascii_lowercase());
    let cache = state_root.join(&name);
    if let Some(index) = read_cache(&cache, reference, manifest) {
        return Ok(Ok(index));
    }
    let partial = state_root.join(format!("{name}.download"));
    let bytes = match artifacts::index_blob(
        transport,
        &reference.blob,
        reference.size,
        &partial,
        cancel,
        progress,
    ) {
        Ok(bytes) => bytes,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => return Ok(Err(format!("the index did not download: {error:?}"))),
    };
    let index = match verify(&bytes, reference, manifest) {
        Ok(index) => index,
        Err(error) => return Ok(Err(error.to_string())),
    };
    let staging = state_root.join(format!("{name}.staging"));
    let stored = std::fs::write(&staging, &bytes).and_then(|()| std::fs::rename(&staging, &cache));
    if stored.is_err() {
        let _ = std::fs::remove_file(&staging);
    }
    stored?;
    Ok(Ok(index))
}

/// The cached copy, if it still verifies against the signed reference and manifest.
fn read_cache(cache: &Path, reference: &FileIndexRef, manifest: &Manifest) -> Option<FileIndex> {
    if std::fs::metadata(cache).ok()?.len() > MAX_FILE_INDEX_BYTES {
        return None;
    }
    let bytes = std::fs::read(cache).ok()?;
    verify(&bytes, reference, manifest).ok()
}

/// Size and digest first, then parse, then the binding to the manifest.
fn verify(
    bytes: &[u8],
    reference: &FileIndexRef,
    manifest: &Manifest,
) -> Result<FileIndex, FileIndexError> {
    let index = FileIndex::parse_verified(bytes, reference)?;
    index.bind(manifest)?;
    Ok(index)
}

/// Fetches the seed and patches, then builds the index in a temp dir under the state root.
fn built(
    state_root: &Path,
    release: &VerifiedRelease,
    transport: &artifacts::Transport,
    seed_source: &SeedSourceHandle,
    prefetched: Option<&Artifacts>,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<FileIndex, Error> {
    let fetched;
    let artifacts = match prefetched {
        Some(artifacts) => artifacts,
        None => {
            fetched = artifacts::fetch(
                state_root,
                transport,
                release,
                seed_source,
                cancel,
                progress,
            )?;
            &fetched
        }
    };
    let manifest = release.manifest();
    // A seed that does not match the signed release is an invalid artifact, not
    // an I/O failure, whichever path supplied it.
    let seed = inventory::hash(&artifacts.seed, cancel)?;
    let seed_hex: String = seed.sha256.iter().map(|b| format!("{b:02x}")).collect();
    if seed.size != manifest.seed.size || !seed_hex.eq_ignore_ascii_case(&manifest.seed.sha256) {
        return Err(Error::InvalidArtifact);
    }
    let patches: Vec<(PatchEntry, PathBuf)> = manifest
        .patches
        .iter()
        .cloned()
        .zip(artifacts.patches.iter().cloned())
        .collect();
    let work = tempfile::tempdir_in(state_root)?;
    let sink = UnpackSink {
        progress: progress.clone(),
        label: "File index".into(),
        cancel: cancel.clone(),
    };
    file_index::build(&artifacts.seed, &patches, manifest, work.path(), &sink).map_err(build_error)
}

fn build_error(error: FileIndexError) -> Error {
    match error {
        FileIndexError::Io(_) => Error::Storage(StorageError::Io),
        _ => Error::InvalidArtifact,
    }
}

fn is_sha256(hex: &str) -> bool {
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())
}
