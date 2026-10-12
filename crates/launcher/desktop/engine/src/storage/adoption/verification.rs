//! The signed reference is reconstructed and compared with the adopted source.
//! Only a legacy source can be compared so far; a folder source is refused.
//! [`acquire_index`] yields the signed per-file index, or the one built locally.
use super::*;
use crate::{
    install_progress::ProgressSink,
    manifest::{Manifest, PatchEntry},
    unpack::UnpackSink,
};
use cimmeria_launcher_core::file_index::{
    self, FileIndex, FileIndexError, FileIndexRef, MAX_FILE_INDEX_BYTES,
};
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub(super) fn reference_and_comparison(
    state: &Arc<Mutex<DesktopState>>,
    request: &PreviewRequest,
    backend: Backend,
    transport: Option<artifacts::Transport>,
    preparation: &mut preparation::Ownership,
    adopted: AdoptionSourceRef<'_>,
    scanned: &inventory::Index,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
    progress: crate::install_progress::ProgressSink,
) -> Result<(reference::Reference, Vec<Difference>), Error> {
    let AdoptionSourceRef::Legacy(imported) = adopted else {
        return Err(Error::UnsupportedMode);
    };
    let fetched;
    let artifacts = match &request.artifacts {
        Some(artifacts) => artifacts,
        None => {
            let transport = match transport {
                Some(transport) => transport,
                None => artifacts::Transport::production()?,
            };
            let root = state
                .lock()
                .map_err(|_| StorageError::Io)?
                .state_root()
                .to_path_buf();
            fetched = artifacts::fetch(&root, &transport, &request.release, cancel, &progress)?;
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
    let reference = reference::reconstruct(
        &preparation.record.directory,
        &request.release,
        artifacts,
        servers,
        cancel,
        progress,
        adapter.as_deref(),
    )?;
    preparation.uncertain = false;
    drop(adapter);
    let files = comparison::compare(&imported.source.game_directory, scanned, &reference)?;
    if !files.iter().any(|d| {
        matches!(
            d.classification,
            Classification::Matched | Classification::KnownTransform
        )
    }) {
        return Err(Error::NoReusableFiles);
    }
    if inventory::scan(&imported.source.game_directory, cancel)? != *scanned {
        return Err(Error::SourceChanged);
    }
    source::verify_import(imported)?;
    Ok((reference, files))
}

/// Where the per-file index came from.
#[allow(dead_code)] // AI-08 part b wires it
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IndexOrigin {
    /// The signed blob, verified against the manifest.
    Published,
    /// Built locally from the seed and patches.
    Built,
}

/// The per-file index for `release`: the signed blob when the manifest names one
/// and it verifies, otherwise the index built from the seed and patches. Blocking,
/// like [`artifacts::fetch`]: call it from a retained worker.
#[allow(dead_code)] // AI-08 part b wires it
pub(super) fn acquire_index(
    state_root: &Path,
    release: &VerifiedRelease,
    transport: &artifacts::Transport,
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
    let index = built(state_root, release, transport, cancel, progress)?;
    Ok((index, IndexOrigin::Built))
}

/// The signed blob, from the cache when it verifies, else downloaded and cached.
/// The outer error aborts; the inner one is a refusal that falls back to the build.
#[allow(dead_code)] // AI-08 part b wires it
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
#[allow(dead_code)] // AI-08 part b wires it
fn read_cache(cache: &Path, reference: &FileIndexRef, manifest: &Manifest) -> Option<FileIndex> {
    if std::fs::metadata(cache).ok()?.len() > MAX_FILE_INDEX_BYTES {
        return None;
    }
    let bytes = std::fs::read(cache).ok()?;
    verify(&bytes, reference, manifest).ok()
}

/// Size and digest first, then parse, then the binding to the manifest.
#[allow(dead_code)] // AI-08 part b wires it
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
#[allow(dead_code)] // AI-08 part b wires it
fn built(
    state_root: &Path,
    release: &VerifiedRelease,
    transport: &artifacts::Transport,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<FileIndex, Error> {
    let fetched = artifacts::fetch(state_root, transport, release, cancel, progress)?;
    let manifest = release.manifest();
    let patches: Vec<(PatchEntry, PathBuf)> = manifest
        .patches
        .iter()
        .cloned()
        .zip(fetched.patches)
        .collect();
    let work = tempfile::tempdir_in(state_root)?;
    let sink = UnpackSink {
        progress: progress.clone(),
        label: "File index".into(),
        cancel: cancel.clone(),
    };
    file_index::build(&fetched.seed, &patches, manifest, work.path(), &sink).map_err(build_error)
}

#[allow(dead_code)] // AI-08 part b wires it
fn build_error(error: FileIndexError) -> Error {
    match error {
        FileIndexError::Io(_) => Error::Storage(StorageError::Io),
        _ => Error::InvalidArtifact,
    }
}

#[allow(dead_code)] // AI-08 part b wires it
fn is_sha256(hex: &str) -> bool {
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())
}
