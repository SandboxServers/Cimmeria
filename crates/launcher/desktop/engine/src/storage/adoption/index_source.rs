//! The per-file index a verification compares against: the signed blob when the
//! manifest names one and it verifies, else the index built from the seed and
//! patches. [`acquire_index`] reports which one it used.
use super::*;
use crate::{
    install_progress::ProgressSink,
    manifest::{Manifest, PatchEntry},
    unpack::UnpackSink,
};
use cimmeria_launcher_core::file_index::{
    self, FileIndex, FileIndexError, FileIndexRef, MAX_FILE_INDEX_BYTES,
};
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
use std::path::PathBuf;

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
