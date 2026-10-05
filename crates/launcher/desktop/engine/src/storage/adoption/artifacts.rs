//! Signed seed and patch blobs for a reference, held in one launcher-owned store.
//! The signed size bounds every response; the signed hash admits every reuse.
use super::*;
use crate::{
    catalog,
    install::Progress,
    install_progress::{ProgressReporter, ProgressSink},
    manifest,
};
use futures_util::StreamExt;
use std::{io::Write, time::Duration};

const STORE: &str = "adoption-artifacts";

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
    let manifest = release.manifest();
    let mut wanted = vec![(
        "seed".to_string(),
        manifest.seed.blob.as_str(),
        manifest.seed.size,
        manifest.seed.sha256.as_str(),
    )];
    for patch in &manifest.patches {
        wanted.push((
            format!("patch {}", patch.id),
            patch.blob.as_str(),
            patch.size,
            patch.sha256.as_str(),
        ));
    }
    let names = wanted
        .iter()
        .map(|(_, _, _, sha)| name(sha))
        .collect::<Result<Vec<_>, _>>()?;
    let store = open_store(state_root, &names)?;
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let mut paths = Vec::with_capacity(wanted.len());
    for ((label, blob, size, sha), name) in wanted.iter().zip(&names) {
        let path = store.join(name);
        if !stored(&path, *size, sha, cancel)? {
            let partial = store.join(format!("{name}.download"));
            let url = manifest::blob_url(&transport.manifest_url, blob);
            let result = runtime
                .block_on(download(
                    &transport.http,
                    &url,
                    &partial,
                    *size,
                    label,
                    cancel,
                    progress,
                ))
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
        paths.push(path);
    }
    let patches = paths.split_off(1);
    Ok(Artifacts {
        seed: paths.remove(0),
        patches,
    })
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
