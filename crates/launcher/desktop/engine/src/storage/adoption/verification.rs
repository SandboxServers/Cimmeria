//! The signed reference is reconstructed for the in-place moves. Classification
//! compares the adopted folder with the per-file index: [`acquire_index`] yields
//! the signed index, or the one built locally. The game directory is
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
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// A file's size and lower-case SHA-256 hex.
type Bytes = (u64, String);

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
    // Read before the full fetch below, so the estimate counts only what the
    // replay must download.
    let held = artifacts::held(&root, &request.release, cancel)?;
    let fetched;
    let artifacts = match &request.artifacts {
        Some(artifacts) => artifacts,
        None => {
            fetched = artifacts::fetch(&root, &transport, &request.release, cancel, &progress)?;
            &fetched
        }
    };
    let (index, _origin) = acquire_index(
        &root,
        &request.release,
        &transport,
        Some(artifacts),
        cancel,
        &progress,
    )?;
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
    // The in-place moves still read the reconstructed bytes.
    // AI-11: drop when the moves use replacements
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
    drop(adapter);
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
    let plan = replay::plan(&index, &files, scanned, mode);
    let seed_source = state
        .lock()
        .map_err(|_| StorageError::Io)?
        .seed_source_for(&request.release);
    let download = replay::estimate(
        &plan,
        request.release.manifest(),
        seed_source.current().is_some(),
        |sha| held.contains(sha),
    );
    let mut replacements = Vec::new();
    // Local fixture artifacts carry no patch list, so only a signed fetch replays.
    if request.artifacts.is_none() && !plan.targets.is_empty() {
        let fetched = artifacts::fetch_some(
            &root,
            &transport,
            &request.release,
            plan.needs_seed,
            &plan.patches,
            &seed_source,
            cancel,
            &progress,
        )?;
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
    Ok(Compared {
        reference,
        files,
        download,
        replacements,
        login_replaced,
    })
}

/// What a preview learns from the signed reference and the folder: the reference
/// for the moves, the per-file review, the download the replay implies, and the
/// files it stored for the adopted tree.
pub(super) struct Compared {
    pub reference: reference::Reference,
    pub files: Vec<Difference>,
    pub download: DownloadEstimate,
    pub replacements: Vec<Replacement>,
    /// `LoginInternal.lua` differs from both its release and prepared bytes (D-AI13).
    pub login_replaced: bool,
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
    let index = built(state_root, release, transport, prefetched, cancel, progress)?;
    Ok((index, IndexOrigin::Built))
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
    prefetched: Option<&Artifacts>,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<FileIndex, Error> {
    let fetched;
    let artifacts = match prefetched {
        Some(artifacts) => artifacts,
        None => {
            fetched = artifacts::fetch(state_root, transport, release, cancel, progress)?;
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
