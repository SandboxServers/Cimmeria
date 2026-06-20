//! Debug-log collection + single-shot Azure Blob upload.
//!
//! Inputs (in `<install_dir>/Binaries/`):
//! - any file whose name starts with `sgwdebuglog` (BigWorld Mercury
//!   unicode log; may have no extension or be `sgwdebuglog.txt`)
//! - everything recursively under `sessions/`
//!
//! Output: a single zip uploaded via one PUT to the SAS URL, named
//! `logs/<hostname>-<utc>-<digest-prefix>.zip`.
//!
//! Wallet protection:
//! - We compute a content digest over the **inputs** (filename + bytes,
//!   sorted), not the zip itself — so a re-zip of unchanged logs (with
//!   different per-entry timestamps) still dedupes against the ledger.
//! - On match: zero HTTP requests. The button reports "nothing new".
//! - On miss: exactly one PUT request, regardless of input file count.

use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;
use walkdir::WalkDir;
use zip::write::FileOptions;

#[derive(Debug, Error)]
pub enum LogError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Storage server returned {status}: {body}")]
    BadResponse { status: u16, body: String },
    #[error("Refusing to upload over non-HTTPS URL")]
    InsecureUrl,
    #[error("Failed to walk sessions directory: {0}")]
    Walk(#[from] walkdir::Error),
}

fn collect_log_inputs(install_dir: &Path) -> Result<Vec<PathBuf>, LogError> {
    let binaries = install_dir.join("Binaries");
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&binaries) {
        for e in entries.flatten() {
            let p = e.path();
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("sgwdebuglog") && p.is_file() {
                    files.push(p);
                }
            }
        }
    }

    let sessions = binaries.join("sessions");
    if sessions.is_dir() {
        // Surface walk errors as a hard failure rather than dropping them
        // silently. The launcher is uploading *diagnostic* logs — losing
        // half the session tree to a permission glitch and reporting
        // success would actively mislead anyone triaging.
        for entry in WalkDir::new(&sessions) {
            let entry = entry?;
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Stable hash over (rel_path, contents) pairs in canonical sort order.
/// Independent of zip-time so dedup ledger checks survive re-zipping.
pub fn compute_content_digest(install_dir: &Path) -> Result<Option<String>, LogError> {
    let files = collect_log_inputs(install_dir)?;
    if files.is_empty() {
        return Ok(None);
    }
    let binaries = install_dir.join("Binaries");
    let mut hasher = Sha256::new();
    for path in &files {
        let rel = rel_in_archive(path, &binaries);
        hasher.update(rel.as_bytes());
        hasher.update([0u8]);
        let data = std::fs::read(path)?;
        hasher.update(&data);
        hasher.update([0u8]);
    }
    // digest 0.11's `Array` output no longer implements `LowerHex`; hex-encode
    // the bytes explicitly instead of the old `{:x}`.
    Ok(Some(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    ))
}

pub fn build_log_zip(install_dir: &Path) -> Result<Option<Vec<u8>>, LogError> {
    let files = collect_log_inputs(install_dir)?;
    if files.is_empty() {
        return Ok(None);
    }
    let binaries = install_dir.join("Binaries");
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zw = zip::ZipWriter::new(cursor);
        let options: FileOptions<()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for path in &files {
            let rel = rel_in_archive(path, &binaries);
            zw.start_file(rel, options)?;
            let data = std::fs::read(path)?;
            zw.write_all(&data)?;
        }
        zw.finish()?;
    }
    Ok(Some(buf))
}

fn rel_in_archive(path: &Path, binaries: &Path) -> String {
    let rel = match path.strip_prefix(binaries) {
        Ok(r) => Path::new("Binaries").join(r),
        Err(_) => Path::new("Binaries").join(path.file_name().unwrap_or_default()),
    };
    rel.to_string_lossy().replace('\\', "/")
}

/// PUT a block blob to an Azure Blob SAS URL in **one HTTP request**. We do
/// not chunk via PutBlock/PutBlockList — debug-log zips are well under
/// the 256MB single-PutBlob limit and one request is one billable
/// transaction.
pub async fn upload_blob(
    http: &reqwest::Client,
    sas_base: &str,
    blob_name: &str,
    body: Vec<u8>,
) -> Result<(), LogError> {
    if !sas_base.starts_with("https://") {
        return Err(LogError::InsecureUrl);
    }
    let url = insert_blob_path(sas_base, blob_name);
    let resp = http
        .put(&url)
        .header("x-ms-blob-type", "BlockBlob")
        .header("content-type", "application/zip")
        .body(body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(LogError::BadResponse { status, body });
    }
    Ok(())
}

fn insert_blob_path(sas_url: &str, blob_name: &str) -> String {
    match sas_url.find('?') {
        Some(qpos) => {
            let (path_part, qs) = sas_url.split_at(qpos);
            let trimmed = path_part.trim_end_matches('/');
            format!("{trimmed}/{blob_name}{qs}")
        }
        None => {
            let trimmed = sas_url.trim_end_matches('/');
            format!("{trimmed}/{blob_name}")
        }
    }
}

pub fn blob_name_for(digest: &str) -> String {
    let host = gethostname::gethostname().to_string_lossy().into_owned();
    let safe_host: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let prefix = &digest[..digest.len().min(12)];
    format!("logs/{safe_host}-{ts}-{prefix}.zip")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_logs(dir: &Path) {
        let bin = dir.join("Binaries");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("sgwdebuglog"), b"hello").unwrap();
        std::fs::write(bin.join("sgwdebuglog.txt"), b"world").unwrap();
        let sess = bin.join("sessions");
        std::fs::create_dir_all(sess.join("2026-05")).unwrap();
        std::fs::write(sess.join("2026-05").join("session.log"), b"sess").unwrap();
    }

    #[test]
    fn collect_picks_up_logs_and_sessions() {
        let dir = tempfile::tempdir().unwrap();
        setup_logs(dir.path());
        let files = collect_log_inputs(dir.path()).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn collect_is_sorted_for_stable_digest() {
        let dir = tempfile::tempdir().unwrap();
        setup_logs(dir.path());
        let a = collect_log_inputs(dir.path()).unwrap();
        let b = collect_log_inputs(dir.path()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn collect_empty_when_dirs_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(collect_log_inputs(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn build_zip_returns_none_when_no_logs() {
        let dir = tempfile::tempdir().unwrap();
        assert!(build_log_zip(dir.path()).unwrap().is_none());
    }

    #[test]
    fn build_zip_round_trips_through_zip_reader() {
        let dir = tempfile::tempdir().unwrap();
        setup_logs(dir.path());
        let bytes = build_log_zip(dir.path()).unwrap().unwrap();
        assert!(!bytes.is_empty());
        let cursor = std::io::Cursor::new(bytes);
        let mut zr = zip::ZipArchive::new(cursor).unwrap();
        let names: std::collections::HashSet<String> = (0..zr.len())
            .map(|i| zr.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains("Binaries/sgwdebuglog"));
        assert!(names.contains("Binaries/sgwdebuglog.txt"));
        assert!(names.contains("Binaries/sessions/2026-05/session.log"));
    }

    #[test]
    fn content_digest_is_stable() {
        let dir = tempfile::tempdir().unwrap();
        setup_logs(dir.path());
        let a = compute_content_digest(dir.path()).unwrap().unwrap();
        let b = compute_content_digest(dir.path()).unwrap().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn content_digest_changes_when_logs_change() {
        let dir = tempfile::tempdir().unwrap();
        setup_logs(dir.path());
        let a = compute_content_digest(dir.path()).unwrap().unwrap();
        std::fs::write(dir.path().join("Binaries/sgwdebuglog"), b"different").unwrap();
        let b = compute_content_digest(dir.path()).unwrap().unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn content_digest_none_when_no_logs() {
        let dir = tempfile::tempdir().unwrap();
        assert!(compute_content_digest(dir.path()).unwrap().is_none());
    }

    #[test]
    fn insert_blob_path_with_sas_query() {
        let u = insert_blob_path(
            "https://x.blob.core.windows.net/sgw?sv=1&sp=c",
            "logs/a.zip",
        );
        assert_eq!(
            u,
            "https://x.blob.core.windows.net/sgw/logs/a.zip?sv=1&sp=c"
        );
    }

    #[test]
    fn insert_blob_path_handles_trailing_slash() {
        let u = insert_blob_path("https://x.blob.core.windows.net/sgw/?sv=1", "logs/a.zip");
        assert_eq!(u, "https://x.blob.core.windows.net/sgw/logs/a.zip?sv=1");
    }

    #[test]
    fn insert_blob_path_without_query() {
        let u = insert_blob_path("https://x.blob.core.windows.net/sgw", "logs/a.zip");
        assert_eq!(u, "https://x.blob.core.windows.net/sgw/logs/a.zip");
    }

    #[tokio::test]
    async fn upload_blob_rejects_non_https() {
        let http = reqwest::Client::new();
        let err = upload_blob(&http, "http://example.com/sgw", "logs/a.zip", vec![])
            .await
            .unwrap_err();
        assert!(matches!(err, LogError::InsecureUrl));
    }

    #[test]
    fn blob_name_includes_digest_prefix() {
        let n = blob_name_for("abcdef1234567890");
        assert!(n.starts_with("logs/"));
        assert!(n.ends_with("abcdef123456.zip"));
    }
}
