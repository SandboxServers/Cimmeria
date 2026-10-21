//! The per-file index a verification compares against: the signed blob when the
//! manifest names one and it verifies, else the index built from the seed and
//! patches. [`acquire_index`] reports which one it used and, for a build, why the
//! signed one was refused (C15).
use super::rejection::{self, Rejection};
use super::*;
use crate::{
    install_progress::ProgressSink,
    manifest::{Manifest, PatchEntry},
    storage::game_telemetry::launcher_event,
    unpack::UnpackSink,
};
use cimmeria_launcher_core::file_index::{
    self, FileIndex, FileIndexError, FileIndexRef, MAX_FILE_INDEX_BYTES,
};
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
use std::path::PathBuf;

/// Why the signed index was not used, and what the build downloads instead.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct IndexFallback {
    /// `index_not_published`, `index_reference_malformed`, `index_download_failed`,
    /// `index_size_mismatch`, `index_digest_mismatch`, `index_parse_failed`,
    /// `index_bind_failed` or `index_read_failed`.
    pub reason: &'static str,
    /// The `check` of a rejection recorded while the signed index was fetched or verified.
    pub check: Option<&'static str>,
    /// The HTTP status a recorded rejection carried, if any.
    pub http_status: Option<u16>,
    /// The seed size the build downloads (`manifest.seed.size`).
    pub seed_bytes: u64,
}

/// Where the per-file index came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum IndexOrigin {
    /// The signed blob, verified against the manifest.
    Published,
    /// Built locally from the seed and patches, for the given reason.
    Built(IndexFallback),
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
    let manifest = release.manifest();
    let fallback = match &manifest.file_index {
        None => refusal("index_not_published", None, manifest.seed.size),
        Some(reference) => {
            match published(state_root, release, transport, reference, cancel, progress)? {
                Ok((index, cached)) => {
                    published_event(cached);
                    return Ok((index, IndexOrigin::Published));
                }
                Err(fallback) => fallback,
            }
        }
    };
    fallback_event(&fallback);
    let index = built(
        state_root,
        release,
        transport,
        seed_source,
        prefetched,
        cancel,
        progress,
    )?;
    Ok((index, IndexOrigin::Built(fallback)))
}

/// The download a comparison reports. A built index fetched every blob it used,
/// so nothing is left to download; a chosen local archive is still named.
pub(super) fn download_for(origin: &IndexOrigin, estimate: DownloadEstimate) -> DownloadEstimate {
    match origin {
        IndexOrigin::Published => estimate,
        IndexOrigin::Built(_) => DownloadEstimate {
            seed: false,
            patches: Vec::new(),
            bytes: 0,
            local_seed: estimate.local_seed,
        },
    }
}

/// The signed blob, from the cache when it verifies, else downloaded and cached.
/// The flag is true for a cache hit. The outer error aborts; the inner one is a
/// refusal that falls back to the build.
fn published(
    state_root: &Path,
    release: &VerifiedRelease,
    transport: &artifacts::Transport,
    reference: &FileIndexRef,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<Result<(FileIndex, bool), IndexFallback>, Error> {
    let manifest = release.manifest();
    let seed_bytes = manifest.seed.size;
    if reference.size > MAX_FILE_INDEX_BYTES || !is_sha256(&reference.sha256) {
        return Ok(Err(refusal("index_reference_malformed", None, seed_bytes)));
    }
    let name = format!("file-index-{}.json", reference.sha256.to_ascii_lowercase());
    let cache = state_root.join(&name);
    if let Some(index) = read_cache(&cache, reference, manifest) {
        return Ok(Ok((index, true)));
    }
    let partial = state_root.join(format!("{name}.download"));
    // The nested scope keeps what the download recorded (a check, an HTTP status)
    // for the fallback instead of letting it escape as this operation's cause.
    let (fetched, recorded) = rejection::scope(|| {
        artifacts::index_blob(
            transport,
            &reference.blob,
            reference.size,
            &partial,
            cancel,
            progress,
        )
    });
    let bytes = match fetched {
        Ok(bytes) => bytes,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(_) => return Ok(Err(refusal("index_download_failed", recorded, seed_bytes))),
    };
    let (verified, recorded) = rejection::scope(|| verify(&bytes, reference, manifest));
    let index = match verified {
        Ok(index) => index,
        Err(error) => return Ok(Err(refusal(verify_reason(&error), recorded, seed_bytes))),
    };
    let staging = state_root.join(format!("{name}.staging"));
    let stored = std::fs::write(&staging, &bytes).and_then(|()| std::fs::rename(&staging, &cache));
    if let Err(error) = stored {
        let _ = std::fs::remove_file(&staging);
        return Err(rejection::reject(
            Error::Storage(StorageError::Io),
            Rejection::new("folder_unreadable")
                .entry("file_index")
                .found(launcher_event::kind(&error)),
        ));
    }
    Ok(Ok((index, false)))
}

/// The fallback for a refused signed index, with what its nested scope recorded.
fn refusal(reason: &'static str, recorded: Option<Rejection>, seed_bytes: u64) -> IndexFallback {
    IndexFallback {
        reason,
        check: recorded.as_ref().map(|r| r.check),
        http_status: recorded.and_then(|r| r.http_status),
        seed_bytes,
    }
}

/// The reason a signed index that fails [`verify`] is refused.
fn verify_reason(error: &FileIndexError) -> &'static str {
    match error {
        FileIndexError::TooLarge => "index_size_mismatch",
        FileIndexError::Digest => "index_digest_mismatch",
        FileIndexError::Schema(_)
        | FileIndexError::SetupPolicy(_)
        | FileIndexError::InvalidPath(_)
        | FileIndexError::Duplicate(_) => "index_parse_failed",
        FileIndexError::Binding(_) => "index_bind_failed",
        FileIndexError::Io(_) => "index_read_failed",
    }
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
        return Err(rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("seed_mismatch")
                .entry("seed")
                .expected(rejection::digest(manifest.seed.size, &manifest.seed.sha256))
                .found(rejection::digest(seed.size, &seed_hex)),
        ));
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

/// The refusal for a failed build; each `FileIndexError` names its own check.
fn build_error(error: FileIndexError) -> Error {
    match error {
        FileIndexError::Io(_) => rejection::reject(
            Error::Storage(StorageError::Io),
            Rejection::new("folder_unreadable")
                .entry("file_index")
                .found("io"),
        ),
        FileIndexError::TooLarge => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("folder_too_large").entry("file_index"),
        ),
        FileIndexError::Digest => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("artifact_hash_mismatch").entry("file_index"),
        ),
        FileIndexError::Schema(version) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("launcher_too_old").found(format!("schema {version}")),
        ),
        FileIndexError::SetupPolicy(policy) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("unsupported_configuration").found(format!("setup policy {policy}")),
        ),
        FileIndexError::Binding(_) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("identity_conflict").entry("file_index"),
        ),
        FileIndexError::InvalidPath(_) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("path_not_canonical").entry("file_index"),
        ),
        FileIndexError::Duplicate(_) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("file_unclassified").entry("file_index"),
        ),
    }
}

/// The signed index is in use. Info: one per adoption; `cached` is true when the
/// cached copy verified and nothing was downloaded.
fn published_event(cached: bool) {
    tracing::info!(
        target: "adoption",
        event = "adoption_index_published",
        cached,
        "adoption: the signed file index verified; using it, so only the replay's files are fetched"
    );
}

/// The signed index was refused and the build replaces it. Warn: the player sees it.
fn fallback_event(fallback: &IndexFallback) {
    tracing::warn!(
        target: "adoption",
        event = "adoption_index_fallback",
        reason = fallback.reason,
        check = fallback.check,
        http_status = fallback.http_status,
        seed_bytes = fallback.seed_bytes,
        "adoption: the signed file index was refused ({}); building it from the seed and patches, which downloads {} seed bytes",
        fallback.reason,
        fallback.seed_bytes
    );
}

fn is_sha256(hex: &str) -> bool {
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())
}
