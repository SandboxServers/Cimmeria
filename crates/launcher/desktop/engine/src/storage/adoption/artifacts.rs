//! Signed seed and patch blobs for a reference, held in one launcher-owned store.
//! The signed size bounds every response; the signed hash admits every reuse.
use super::*;
use crate::{
    catalog,
    install::Progress,
    install_progress::{ProgressReporter, ProgressSink},
    manifest,
};
use cimmeria_launcher_core::seed_source::{LocalSeedError, SeedSourceHandle};
use futures_util::StreamExt;
use std::{collections::BTreeSet, io::Write, time::Duration};

const STORE: &str = "adoption-artifacts";

/// The blobs one [`fetch_some`] call holds. `seed` is `None` when the seed was
/// not wanted; `patches[k]` is `None` for each manifest patch left out.
pub(super) struct Fetched {
    pub seed: Option<PathBuf>,
    pub patches: Vec<Option<PathBuf>>,
}

/// Origin of signed blobs. Production is the fixed HTTPS catalog; another origin
/// exists only through the explicit loopback test entry point.
pub struct Transport {
    http: reqwest::Client,
    manifest_url: String,
}
impl Transport {
    pub(super) fn production() -> Result<Self, StorageError> {
        let http = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|_| StorageError::Io)?;
        Ok(Self {
            http,
            manifest_url: catalog::URL.into(),
        })
    }
    /// Fixture servers only: anything but a literal loopback origin is refused, and
    /// redirects are not followed, so a test can never reach a real endpoint.
    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn loopback(manifest_url: String) -> Result<Self, StorageError> {
        let url = reqwest::Url::parse(&manifest_url).map_err(|_| StorageError::InvalidDirectory)?;
        if !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) {
            return Err(StorageError::InvalidDirectory);
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| StorageError::Io)?;
        Ok(Self { http, manifest_url })
    }
}

/// Blocking: call from a retained worker. Blobs that already match the signed
/// release are reused, so a dismissed review does not repeat the download.
pub(super) fn fetch(
    state_root: &Path,
    transport: &Transport,
    release: &VerifiedRelease,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<Artifacts, Error> {
    let all: Vec<usize> = (0..release.manifest().patches.len()).collect();
    let fetched = fetch_some(
        state_root,
        transport,
        release,
        true,
        &all,
        &SeedSourceHandle::download(),
        cancel,
        progress,
    )?;
    let patches = fetched
        .patches
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(Error::InvalidArtifact)?;
    Ok(Artifacts {
        seed: fetched.seed.ok_or(Error::InvalidArtifact)?,
        patches,
    })
}

/// The SHA-256 of each signed blob of `release` already in the store and
/// verifying. Read before a fetch, so the review counts only what must download.
pub(super) fn held(
    state_root: &Path,
    release: &VerifiedRelease,
    cancel: &CancellationToken,
) -> Result<BTreeSet<String>, Error> {
    let manifest = release.manifest();
    let store = state_root.join(STORE);
    let mut held = BTreeSet::new();
    let blobs = std::iter::once((manifest.seed.size, manifest.seed.sha256.as_str())).chain(
        manifest
            .patches
            .iter()
            .map(|patch| (patch.size, patch.sha256.as_str())),
    );
    for (size, sha) in blobs {
        if stored(&store.join(name(sha)?), size, sha, cancel)? {
            held.insert(sha.to_string());
        }
    }
    Ok(held)
}

/// Blocking, like [`fetch`], for the blobs a replay needs: the seed when `seed`
/// is set, and the manifest patches at `patches` (indices in signed order). A
/// stored blob that verifies is reused, and the release's other stored blobs are
/// kept. With a chosen local seed archive the seed is copied from it and never
/// downloaded. Every download comes from the signed blob URL.
#[allow(clippy::too_many_arguments)]
pub(super) fn fetch_some(
    state_root: &Path,
    transport: &Transport,
    release: &VerifiedRelease,
    seed: bool,
    patches: &[usize],
    seed_source: &SeedSourceHandle,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<Fetched, Error> {
    let manifest = release.manifest();
    // Slot 0 is the seed, slot `k + 1` is manifest patch `k`, in signed order.
    let mut blobs = vec![(
        "seed".to_string(),
        manifest.seed.blob.as_str(),
        manifest.seed.size,
        manifest.seed.sha256.as_str(),
    )];
    for patch in &manifest.patches {
        blobs.push((
            format!("patch {}", patch.id),
            patch.blob.as_str(),
            patch.size,
            patch.sha256.as_str(),
        ));
    }
    let names = blobs
        .iter()
        .map(|(_, _, _, sha)| name(sha))
        .collect::<Result<Vec<_>, _>>()?;
    let store = open_store(state_root, &names)?;
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let mut fetched = Fetched {
        seed: None,
        patches: vec![None; manifest.patches.len()],
    };
    for (slot, ((label, blob, size, sha), name)) in blobs.iter().zip(&names).enumerate() {
        let wanted = match slot.checked_sub(1) {
            None => seed,
            Some(k) => patches.contains(&k),
        };
        if !wanted {
            continue;
        }
        let path = store.join(name);
        if !stored(&path, *size, sha, cancel)? {
            let partial = store.join(format!("{name}.download"));
            let local = if slot == 0 {
                seed_source.current()
            } else {
                None
            };
            let result = match local {
                Some(local) => local
                    .copy_into(&partial, cancel, progress)
                    .map_err(local_seed_error),
                None => {
                    let url = manifest::blob_url(&transport.manifest_url, blob);
                    runtime.block_on(download(
                        &transport.http,
                        &url,
                        &partial,
                        *size,
                        label,
                        cancel,
                        progress,
                    ))
                }
            }
            .and_then(|()| matches(&partial, *size, sha, cancel));
            match result {
                Ok(true) => std::fs::rename(&partial, &path)?,
                Ok(false) => {
                    let _ = std::fs::remove_file(&partial);
                    return Err(Error::InvalidArtifact);
                }
                Err(error) => {
                    let _ = std::fs::remove_file(&partial);
                    return Err(error);
                }
            }
        }
        match slot.checked_sub(1) {
            None => fetched.seed = Some(path),
            Some(k) => fetched.patches[k] = Some(path),
        }
    }
    Ok(fetched)
}

/// A refused local archive is not retried by a download: the review names it.
fn local_seed_error(error: LocalSeedError) -> Error {
    match error {
        LocalSeedError::Cancelled => Error::Cancelled,
        LocalSeedError::Io => StorageError::Io.into(),
        _ => Error::InvalidArtifact,
    }
}

/// Blocking, like [`fetch`]: the signed file index, bounded by its signed size
/// (the same TLS and redirect policy as the seed and patches) and read back into
/// memory. `partial` is scratch and is removed on every outcome.
#[allow(dead_code)] // AI-08 part b wires it
pub(super) fn index_blob(
    transport: &Transport,
    blob: &str,
    size: u64,
    partial: &Path,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<Vec<u8>, Error> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let url = manifest::blob_url(&transport.manifest_url, blob);
    let _ = std::fs::remove_file(partial);
    let fetched = runtime
        .block_on(download(
            &transport.http,
            &url,
            partial,
            size,
            "file index",
            cancel,
            progress,
        ))
        .and_then(|()| std::fs::read(partial).map_err(Error::from));
    let _ = std::fs::remove_file(partial);
    fetched
}

/// Best effort after publication: the adopted copy no longer needs its archives.
pub(super) fn discard_store(state_root: &Path) {
    if open_store(state_root, &[]).is_ok() {
        let _ = std::fs::remove_dir(state_root.join(STORE));
    }
}

fn name(sha256: &str) -> Result<String, Error> {
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::InvalidArtifact);
    }
    Ok(format!("{}.artifact", sha256.to_ascii_lowercase()))
}

/// Only plain files this module wrote live here. Blobs of another release and
/// interrupted downloads are removed; anything else is refused, never followed.
fn open_store(state_root: &Path, keep: &[String]) -> Result<PathBuf, Error> {
    let store = state_root.join(STORE);
    match std::fs::create_dir(&store) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error.into()),
    }
    if !std::fs::symlink_metadata(&store)?.is_dir() {
        return Err(StorageError::UnsafeFile.into());
    }
    for entry in std::fs::read_dir(&store)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err(StorageError::UnsafeFile.into());
        }
        if !keep.iter().any(|name| entry.file_name() == name.as_str()) {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(store)
}

fn stored(path: &Path, size: u64, sha256: &str, cancel: &CancellationToken) -> Result<bool, Error> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
        Ok(_) => (),
    }
    if matches(path, size, sha256, cancel)? {
        return Ok(true);
    }
    std::fs::remove_file(path)?;
    Ok(false)
}

/// The local-artifact counterpart of [`fetch_some`]: nothing is downloaded, but
/// each wanted blob is hash-checked against the signed manifest, so a corrupt or
/// swapped local artifact is still [`Error::InvalidArtifact`].
pub(super) fn local_some(
    local: &Artifacts,
    release: &VerifiedRelease,
    seed: bool,
    patches: &[usize],
    cancel: &CancellationToken,
) -> Result<Fetched, Error> {
    let manifest = release.manifest();
    let mut fetched = Fetched {
        seed: None,
        patches: vec![None; manifest.patches.len()],
    };
    if seed {
        if !matches(
            &local.seed,
            manifest.seed.size,
            &manifest.seed.sha256,
            cancel,
        )? {
            return Err(Error::InvalidArtifact);
        }
        fetched.seed = Some(local.seed.clone());
    }
    for &k in patches {
        let path = local.patches.get(k).ok_or(Error::InvalidArtifact)?;
        let patch = manifest.patches.get(k).ok_or(Error::InvalidArtifact)?;
        if !matches(path, patch.size, &patch.sha256, cancel)? {
            return Err(Error::InvalidArtifact);
        }
        fetched.patches[k] = Some(path.clone());
    }
    Ok(fetched)
}

fn matches(
    path: &Path,
    size: u64,
    sha256: &str,
    cancel: &CancellationToken,
) -> Result<bool, Error> {
    let entry = inventory::hash(path, cancel)?;
    let actual: String = entry.sha256.iter().map(|b| format!("{b:02x}")).collect();
    Ok(entry.size == size && actual.eq_ignore_ascii_case(sha256))
}

/// No resume and no trust in the response: a declared length other than the
/// signed size is refused before any byte is written, and a body that runs past
/// the signed size stops the transfer instead of filling the disk.
async fn download(
    http: &reqwest::Client,
    url: &str,
    partial: &Path,
    size: u64,
    label: &str,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<(), Error> {
    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(Error::Cancelled),
        response = http.get(url).send() => response.map_err(|_| Error::Network)?,
    };
    if response.status() != reqwest::StatusCode::OK {
        return Err(Error::Network);
    }
    if response
        .content_length()
        .is_some_and(|declared| declared != size)
    {
        return Err(Error::InvalidArtifact);
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(partial)?;
    let mut received = 0u64;
    let mut stream = response.bytes_stream();
    let mut reported = std::time::Instant::now();
    loop {
        let next = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(Error::Cancelled),
            next = stream.next() => next,
        };
        let Some(chunk) = next else {
            break;
        };
        let chunk = chunk.map_err(|_| Error::Network)?;
        received += chunk.len() as u64;
        if received > size {
            return Err(Error::InvalidArtifact);
        }
        output.write_all(&chunk)?;
        if reported.elapsed() >= Duration::from_millis(33) {
            progress.report(Progress::Downloading {
                label: label.to_string(),
                downloaded: received,
                total: size,
            });
            reported = std::time::Instant::now();
        }
    }
    if received != size {
        return Err(Error::InvalidArtifact);
    }
    output.sync_all()?;
    progress.report(Progress::Downloading {
        label: label.to_string(),
        downloaded: received,
        total: size,
    });
    Ok(())
}
