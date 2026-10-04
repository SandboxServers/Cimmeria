//! Pinned Wine cache preparation. Does not execute Wine, initialize a prefix or
//! claim game readiness. All paths are chosen by the native coordinator.
mod download;
mod tree;
use crate::install_progress::ProgressSink;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tokio_util::sync::CancellationToken;

const URL:&str="https://github.com/WoWSilicon/WoWSilicon/releases/download/wine-runtime-r17/WoWSilicon-WineRuntime-r17.tar.xz";
const SIZE: u64 = 63_744_856;
pub(crate) const ARCHIVE_SHA256: &str =
    "dc67cf0c2dd1e4c1cfaffe924f4737aaa594645b135a7973ac3505c83c70f882";
const TREE: &str = "b46591cfa9e72d197b46851b3c884bdd353293a1ff78df5714860c26fe88ada9";
const NAME: &str = "wine-r17-dc67cf0c2dd1e4c1";
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("runtime cache IO failure")]
    Io,
    #[error("runtime cache is in use")]
    Busy,
    #[error("runtime download failed")]
    Network,
    #[error("runtime archive failed verification")]
    Verification,
    #[error("runtime extraction failed")]
    Extraction,
    #[error("runtime preparation cancelled")]
    Cancelled,
}
impl From<std::io::Error> for RuntimeError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
fn cancelled(cancel: &CancellationToken) -> Result<(), RuntimeError> {
    if cancel.is_cancelled() {
        Err(RuntimeError::Cancelled)
    } else {
        Ok(())
    }
}
fn lock(root: &Path) -> Result<File, RuntimeError> {
    if !root.is_absolute() {
        return Err(RuntimeError::Io);
    }
    std::fs::create_dir_all(root)?;
    if std::fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(RuntimeError::Io);
    }
    let path = root.join("runtime.lock");
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        if !meta.is_file() {
            return Err(RuntimeError::Io);
        }
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.try_lock().map_err(|_| RuntimeError::Busy)?;
    Ok(file)
}
fn verify_archive(file: &mut File) -> Result<(), RuntimeError> {
    if file.metadata()?.len() != SIZE {
        return Err(RuntimeError::Verification);
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if hex(&hash.finalize()) != ARCHIVE_SHA256 {
        return Err(RuntimeError::Verification);
    }
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Caller retains the native operation scope. Download is cancellable; tar
/// extraction is a blocking phase with cancellation checked before promotion.
/// Existing corrupt caches fail closed and are preserved for explicit repair.
pub async fn prepare(
    root: PathBuf,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Result<PathBuf, RuntimeError> {
    cancelled(&cancel)?;
    let root_for_lock = root.clone();
    let owner = tokio::task::spawn_blocking(move || lock(&root_for_lock))
        .await
        .map_err(|_| RuntimeError::Io)??;
    let destination = root.join(NAME);
    if std::fs::symlink_metadata(&destination).is_ok() {
        let candidate = destination.clone();
        let token = cancel.clone();
        return tokio::task::spawn_blocking(move || {
            let _owner = owner;
            tree::verify(&candidate, TREE, &token)?;
            Ok(candidate)
        })
        .await
        .map_err(|_| RuntimeError::Io)?;
    }
    let http = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(300))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| RuntimeError::Network)?;
    let archive = download::fetch(&http, URL, SIZE, &root, &cancel, &progress).await?;
    tokio::task::spawn_blocking(move || {
        // Own lock, archive and staging inside the task: dropping a UI future
        // cannot delete staging while tar is still writing into it.
        let _owner = owner;
        publish(&root, &destination, archive.path(), &cancel)
    })
    .await
    .map_err(|_| RuntimeError::Io)?
}

fn publish(
    root: &Path,
    destination: &Path,
    archive: &Path,
    cancel: &CancellationToken,
) -> Result<PathBuf, RuntimeError> {
    cancelled(cancel)?;
    verify_archive(&mut File::open(archive)?)?;
    let stage = tempfile::Builder::new()
        .prefix(".wine-stage-")
        .tempdir_in(root)?;
    let status = Command::new("/usr/bin/tar")
        .args(["-xpf"])
        .arg(archive)
        .arg("-C")
        .arg(stage.path())
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(RuntimeError::Extraction);
    }
    let extracted = stage.path().join(".wine-runtime");
    tree::verify(&extracted, TREE, cancel)?;
    cancelled(cancel)?;
    if std::fs::symlink_metadata(destination).is_ok() {
        return Err(RuntimeError::Busy);
    }
    std::fs::rename(&extracted, destination)?;
    File::open(root)?.sync_all()?;
    Ok(destination.to_path_buf())
}

#[cfg(test)]
mod tests;
