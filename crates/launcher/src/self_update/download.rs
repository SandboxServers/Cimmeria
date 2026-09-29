//! Download a launcher release beside the running exe and verify it.
//!
//! The new exe goes to `.sgw-launcher-update-<tag>.exe.part` in the exe's
//! own directory (the swap is then a same-volume rename). The download
//! resumes an earlier partial file through the install pipeline's
//! Range-aware downloader. Nothing touches the running exe until the file
//! matches both the size the release lists and the SHA-256 in the
//! release's `.sha256` asset; a file that fails either is deleted.

use std::path::{Path, PathBuf};

use thiserror::Error;
use tokio_util::sync::CancellationToken;

use super::releases::{LauncherRelease, UpdateEndpoints};
use crate::install::{download_to_file, hash_file, InstallError, Progress};

/// Largest `.sha256` file the updater reads. `sha256sum` output for one
/// file is ~100 bytes.
const MAX_SHA_FILE: usize = 4096;

const PART_PREFIX: &str = ".sgw-launcher-update-";
const PART_SUFFIX: &str = ".exe.part";

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("refusing to download from an untrusted address: {0}")]
    UntrustedUrl(String),
    #[error("the release's checksum file is unusable: {0}")]
    BadChecksumFile(String),
    #[error("download failed: {0}")]
    Http(String),
    #[error("could not write the download: {0}")]
    Io(#[from] std::io::Error),
    #[error("the download is {actual} bytes, the release lists {expected}; deleted it")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("the download does not match the release's SHA-256 (expected {expected}, got {actual}); deleted it")]
    HashMismatch { expected: String, actual: String },
}

impl DownloadError {
    /// Stable `reason` value for the telemetry row.
    pub fn reason(&self) -> &'static str {
        match self {
            DownloadError::UntrustedUrl(_) => "untrusted_url",
            DownloadError::BadChecksumFile(_) => "bad_checksum_file",
            DownloadError::Http(_) => "http_failed",
            DownloadError::Io(_) => "io_failed",
            DownloadError::SizeMismatch { .. } => "size_mismatch",
            DownloadError::HashMismatch { .. } => "sha256_mismatch",
        }
    }
}

/// Where the download for `tag` is written.
pub fn part_path(dir: &Path, tag: &str) -> PathBuf {
    dir.join(format!("{PART_PREFIX}{tag}{PART_SUFFIX}"))
}

/// Parse `sha256sum` output (`<64 hex>  <name>`, or `<64 hex> *<name>`, or
/// the bare hash). When a name is present it must be the exe's, so a
/// checksum published for another file is not trusted for this one.
pub fn parse_sha256_file(text: &str, exe_name: &str) -> Result<String, DownloadError> {
    let mut parts = text.split_whitespace();
    let hash = parts
        .next()
        .ok_or_else(|| DownloadError::BadChecksumFile("empty".into()))?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(DownloadError::BadChecksumFile(
            "first field is not a 64-character hex SHA-256".into(),
        ));
    }
    if let Some(name) = parts.next() {
        let name = name.trim_start_matches('*');
        if name != exe_name {
            return Err(DownloadError::BadChecksumFile(format!(
                "it names {name}, not {exe_name}"
            )));
        }
    }
    Ok(hash.to_ascii_lowercase())
}

/// Download `release`'s exe into `dir` and verify it. Returns the verified
/// file's path.
pub async fn download_verified(
    http: &reqwest::Client,
    endpoints: &UpdateEndpoints,
    release: &LauncherRelease,
    dir: &Path,
    progress: &tokio::sync::mpsc::UnboundedSender<Progress>,
) -> Result<PathBuf, DownloadError> {
    for url in [&release.exe.url, &release.sha256.url] {
        let parsed =
            reqwest::Url::parse(url).map_err(|_| DownloadError::UntrustedUrl(url.clone()))?;
        if !endpoints.url_allowed(&parsed) {
            return Err(DownloadError::UntrustedUrl(url.clone()));
        }
    }

    let expected_sha = fetch_expected_sha(http, release).await?;
    remove_other_parts(dir, &release.tag);

    let part = part_path(dir, &release.tag);
    download_to_file(
        http,
        &release.exe.url,
        &part,
        CancellationToken::new(),
        release.exe.size,
        "Launcher update",
        progress,
    )
    .await
    .map_err(|e| match e {
        InstallError::Io(io) => DownloadError::Io(io),
        other => DownloadError::Http(describe(&other)),
    })?;

    let part_for_check = part.clone();
    let expected_size = release.exe.size;
    tokio::task::spawn_blocking(move || verify_file(&part_for_check, expected_size, &expected_sha))
        .await
        .map_err(|e| DownloadError::Io(std::io::Error::other(e.to_string())))??;
    Ok(part)
}

/// Check the downloaded file's size and SHA-256; delete it on a mismatch.
pub fn verify_file(
    path: &Path,
    expected_size: u64,
    expected_sha: &str,
) -> Result<(), DownloadError> {
    let actual_size = std::fs::metadata(path)?.len();
    if actual_size != expected_size {
        let _ = std::fs::remove_file(path);
        return Err(DownloadError::SizeMismatch {
            expected: expected_size,
            actual: actual_size,
        });
    }
    let actual = hash_file(path)?;
    if !actual.eq_ignore_ascii_case(expected_sha) {
        let _ = std::fs::remove_file(path);
        return Err(DownloadError::HashMismatch {
            expected: expected_sha.to_string(),
            actual,
        });
    }
    Ok(())
}

async fn fetch_expected_sha(
    http: &reqwest::Client,
    release: &LauncherRelease,
) -> Result<String, DownloadError> {
    let resp = http
        .get(&release.sha256.url)
        .send()
        .await
        .map_err(|e| DownloadError::Http(describe(&e)))?;
    if !resp.status().is_success() {
        return Err(DownloadError::Http(format!(
            "HTTP {} for the checksum file",
            resp.status().as_u16()
        )));
    }
    let body = resp
        .bytes()
        .await
        .map_err(|e| DownloadError::Http(describe(&e)))?;
    if body.len() > MAX_SHA_FILE {
        return Err(DownloadError::BadChecksumFile("too large".into()));
    }
    let text = std::str::from_utf8(&body)
        .map_err(|_| DownloadError::BadChecksumFile("not text".into()))?;
    parse_sha256_file(text, &release.exe.name)
}

/// An error and its causes on one line. reqwest's own message leaves out
/// the cause ("error following redirect" without saying which host was
/// refused), and the player needs the cause.
pub(crate) fn describe(e: &dyn std::error::Error) -> String {
    let mut out = e.to_string();
    let mut src = e.source();
    while let Some(s) = src {
        let s_text = s.to_string();
        if !out.contains(&s_text) {
            out.push_str(": ");
            out.push_str(&s_text);
        }
        src = s.source();
    }
    out
}

/// Delete partial downloads of other releases; they can never resume.
fn remove_other_parts(dir: &Path, keep_tag: &str) {
    let keep = format!("{PART_PREFIX}{keep_tag}{PART_SUFFIX}");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(PART_PREFIX) && name.ends_with(PART_SUFFIX) && name != keep {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
