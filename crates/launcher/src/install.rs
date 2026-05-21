//! Seed + patch download and extraction pipeline.
//!
//! Flow:
//! 1. If `InstalledState.seed_sha256 != manifest.seed.sha256`: download the
//!    seed zip (resumable via HTTP Range), verify sha256, extract into the
//!    install dir, then reset the applied-patches list.
//! 2. For each patch in declared order: skip if already applied; else
//!    download → verify → extract (overlay-style) → record in state.
//! 3. Patch SGW.exe `.rdata` hostname if it still contains the original CME
//!    string. Idempotent.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::manifest::{blob_url, Manifest, PatchEntry, SeedEntry};
use crate::patch_rdata;
use crate::state::{InstalledState, StateError};

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("State error: {0}")]
    State(#[from] StateError),
    #[error("Hostname patch error: {0}")]
    Patch(#[from] patch_rdata::PatchError),
    #[error("Hash mismatch for {what}: expected {expected}, got {actual}")]
    HashMismatch {
        what: String,
        expected: String,
        actual: String,
    },
    #[error("Unexpected HTTP {status} for {url}")]
    UnexpectedStatus { status: u16, url: String },
    #[error("Cancelled")]
    Cancelled,
}

/// Progress events emitted during install. Forwarded to the UI thread by
/// the worker, which wraps these in [`crate::worker::Event::Progress`].
#[derive(Debug, Clone)]
pub enum Progress {
    Downloading {
        label: String,
        downloaded: u64,
        total: u64,
    },
    Extracting {
        label: String,
        current: usize,
        total: usize,
        filename: String,
    },
}

pub struct InstallContext<'a> {
    pub manifest_url: &'a str,
    pub install_dir: &'a Path,
    pub manifest: &'a Manifest,
    pub server_host: &'a str,
    pub cancel: CancellationToken,
    pub progress: tokio::sync::mpsc::UnboundedSender<Progress>,
    /// Shared HTTP client owned by the worker — reused across the
    /// seed download, every patch download, and the post-install
    /// manifest revalidation. Avoids rebuilding the connection pool
    /// per blob. Configured with `https_only(true)`.
    pub http: &'a reqwest::Client,
}

pub async fn install_all(ctx: InstallContext<'_>) -> Result<(), InstallError> {
    std::fs::create_dir_all(ctx.install_dir)?;
    let mut state = InstalledState::load(ctx.install_dir);

    let seed_matches = state.seed_sha256.as_deref() == Some(ctx.manifest.seed.sha256.as_str());
    if !seed_matches {
        apply_seed(&ctx, &ctx.manifest.seed).await?;
        // Re-seeding invalidates the applied-patches list, but we keep
        // patched_host across re-seeds — the seed always contains an
        // unpatched SGW.exe, so we'll patch fresh below.
        state = InstalledState {
            seed_sha256: Some(ctx.manifest.seed.sha256.clone()),
            applied_patches: Vec::new(),
            patched_host: None,
        };
        state.save(ctx.install_dir)?;
    }

    for patch in &ctx.manifest.patches {
        if state.has_applied(&patch.id) {
            continue;
        }
        apply_patch(&ctx, patch).await?;
        state.applied_patches.push(patch.id.clone());
        state.save(ctx.install_dir)?;
    }

    let exe = ctx.install_dir.join("SGW.exe");
    if exe.exists() {
        let data = std::fs::read(&exe)?;
        // Re-patch when:
        //   - the binary still has the original CME literal (fresh install
        //     or re-seed), OR
        //   - the previously-patched host no longer matches `server_host`
        //     (user edited the config between installs).
        if patch_rdata::host_differs(&data, ctx.server_host, state.patched_host.as_deref()) {
            patch_rdata::patch_exe_any(&exe, ctx.server_host, state.patched_host.as_deref())?;
            state.patched_host = Some(ctx.server_host.to_string());
            state.save(ctx.install_dir)?;
            info!("Patched SGW.exe hostname to '{}'", ctx.server_host);
        }
    } else {
        warn!("SGW.exe missing after install — verify the seed manifest entry");
    }

    Ok(())
}

async fn apply_seed(ctx: &InstallContext<'_>, seed: &SeedEntry) -> Result<(), InstallError> {
    let url = blob_url(ctx.manifest_url, &seed.blob);
    let short_hash = &seed.sha256[..12.min(seed.sha256.len())];
    let tmp = ctx.install_dir.join(format!(".tmp-seed-{short_hash}.zip"));
    download_to_file(
        ctx.http,
        &url,
        &tmp,
        ctx.cancel.clone(),
        seed.size,
        "seed",
        &ctx.progress,
    )
    .await?;
    verify_sha256(&tmp, &seed.sha256, "seed")?;
    info!("Extracting seed into {}", ctx.install_dir.display());
    extract_zip(&tmp, ctx.install_dir, &ctx.progress, "seed", &ctx.cancel)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

async fn apply_patch(ctx: &InstallContext<'_>, patch: &PatchEntry) -> Result<(), InstallError> {
    let url = blob_url(ctx.manifest_url, &patch.blob);
    let safe_id: String = patch
        .id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    // Include a sha prefix in the tmp filename (Cady #6i) so a republished
    // patch with the same `id` but a new sha256 doesn't accidentally
    // resume against the stale bytes — different sha → different tmp
    // path → fresh download.
    let safe_sha = &patch.sha256[..12.min(patch.sha256.len())];
    let tmp = ctx
        .install_dir
        .join(format!(".tmp-patch-{safe_id}-{safe_sha}.zip"));
    let label = format!("patch {}", patch.id);
    download_to_file(
        ctx.http,
        &url,
        &tmp,
        ctx.cancel.clone(),
        patch.size,
        &label,
        &ctx.progress,
    )
    .await?;
    verify_sha256(&tmp, &patch.sha256, &label)?;
    info!("Applying {} into {}", label, ctx.install_dir.display());
    extract_zip(&tmp, ctx.install_dir, &ctx.progress, &label, &ctx.cancel)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

async fn download_to_file(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
    cancel: CancellationToken,
    expected_size: u64,
    label: &str,
    progress: &tokio::sync::mpsc::UnboundedSender<Progress>,
) -> Result<(), InstallError> {
    use futures_util::StreamExt;

    let existing_len = if dest.exists() {
        tokio::fs::metadata(dest).await?.len()
    } else {
        0
    };
    let mut req = http.get(url);
    if existing_len > 0 {
        req = req.header("Range", format!("bytes={existing_len}-"));
    }
    let resp = req.send().await?;

    let status = resp.status();
    let resumed = status.as_u16() == 206;
    // 206 (Partial Content) is technically 2xx, so `error_for_status_ref`
    // wouldn't flag it — but we keep the explicit check to make the resume
    // path obvious. For everything else, surface the HTTP status as an
    // error. We can't call `error_for_status_ref().unwrap_err()` here
    // because that only returns Err for 4xx/5xx — a stray 1xx/3xx would
    // panic. reqwest follows 3xx redirects by default, so this is mostly
    // theoretical, but the non-panicking path is one line longer and
    // robust to any future redirect-policy change.
    if !status.is_success() && !resumed {
        return Err(InstallError::UnexpectedStatus {
            status: status.as_u16(),
            url: url.to_string(),
        });
    }

    let total = if resumed {
        existing_len + resp.content_length().unwrap_or(0)
    } else {
        resp.content_length().unwrap_or(expected_size)
    };

    let mut file = if resumed && existing_len > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(dest)
            .await?
    } else {
        if let Some(p) = dest.parent() {
            tokio::fs::create_dir_all(p).await?;
        }
        tokio::fs::File::create(dest).await?
    };

    let mut downloaded = if resumed { existing_len } else { 0 };
    let mut stream = resp.bytes_stream();
    let mut last_emit = std::time::Instant::now();
    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(InstallError::Cancelled);
        }
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        // Throttle progress emits so the UI channel isn't flooded by tiny chunks.
        if last_emit.elapsed() >= std::time::Duration::from_millis(33) {
            let _ = progress.send(Progress::Downloading {
                label: label.to_string(),
                downloaded,
                total,
            });
            last_emit = std::time::Instant::now();
        }
    }
    file.flush().await?;
    let _ = progress.send(Progress::Downloading {
        label: label.to_string(),
        downloaded,
        total,
    });
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str, what: &str) -> Result<(), InstallError> {
    let actual = hash_file(path)?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(InstallError::HashMismatch {
            what: what.to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 64];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_zip(
    zip_path: &Path,
    dest: &Path,
    progress: &tokio::sync::mpsc::UnboundedSender<Progress>,
    label: &str,
    cancel: &CancellationToken,
) -> Result<(), InstallError> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let total = archive.len();
    for i in 0..total {
        if cancel.is_cancelled() {
            return Err(InstallError::Cancelled);
        }
        let mut entry = archive.by_index(i)?;
        // SECURITY: `enclosed_name()` is the zip-slip gate. It rejects
        // entries whose normalised path would escape the archive root —
        // absolute paths, `..` traversal, and OS-specific weirdness like
        // drive letters or NTFS reserved names all get filtered here.
        // Do NOT replace with `entry.name()` or `entry.mangled_name()`:
        // both will happily hand back paths like `../../etc/passwd`.
        //
        // Also clone to PathBuf so the borrow on `entry` ends before we
        // use `entry.is_dir()` / `&mut entry` below.
        let rel = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let out = dest.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }
        let _ = progress.send(Progress::Extracting {
            label: label.to_string(),
            current: i + 1,
            total,
            filename: out.display().to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_a_known_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x");
        std::fs::write(&p, b"hello world").unwrap();
        assert_eq!(
            hash_file(&p).unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn verify_sha256_succeeds_on_match() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x");
        std::fs::write(&p, b"hello world").unwrap();
        verify_sha256(
            &p,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
            "test",
        )
        .unwrap();
    }

    #[test]
    fn verify_sha256_fails_on_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x");
        std::fs::write(&p, b"hello world").unwrap();
        let err = verify_sha256(&p, "deadbeef", "test").unwrap_err();
        assert!(matches!(err, InstallError::HashMismatch { .. }));
    }

    #[test]
    fn extract_zip_writes_expected_files() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("a.zip");
        // Build a tiny zip on-the-fly.
        {
            use std::io::Write;
            let f = std::fs::File::create(&zip_path).unwrap();
            let mut zw = zip::ZipWriter::new(f);
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            zw.start_file("hello.txt", opts).unwrap();
            zw.write_all(b"hi").unwrap();
            zw.start_file("nested/deep.txt", opts).unwrap();
            zw.write_all(b"deep").unwrap();
            zw.finish().unwrap();
        }
        let out = dir.path().join("out");
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        extract_zip(&zip_path, &out, &tx, "test", &cancel).unwrap();
        assert_eq!(
            std::fs::read_to_string(out.join("hello.txt")).unwrap(),
            "hi"
        );
        assert_eq!(
            std::fs::read_to_string(out.join("nested/deep.txt")).unwrap(),
            "deep"
        );
    }

    // Regression guard for the non-panicking HTTP-status branch added in
    // PR #343's first fixup. Prior to the fix this path called
    // `error_for_status_ref().unwrap_err()` which would panic on stray
    // 1xx/3xx; the new code returns `InstallError::UnexpectedStatus`
    // carrying the status + url.
    #[tokio::test]
    async fn download_to_file_returns_unexpected_status_on_non_2xx() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/seed.zip"))
            .respond_with(ResponseTemplate::new(418))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(".tmp-test.zip");
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<Progress>();
        let url = format!("{}/seed.zip", server.uri());
        // Test client allows http:// — production worker client is
        // configured with https_only(true).
        let http = reqwest::Client::new();
        let err = download_to_file(&http, &url, &dest, CancellationToken::new(), 0, "seed", &tx)
            .await
            .expect_err("418 must surface as an error, not panic");

        match err {
            InstallError::UnexpectedStatus { status, url: u } => {
                assert_eq!(status, 418);
                assert!(u.ends_with("/seed.zip"));
            }
            other => panic!("expected UnexpectedStatus, got {other:?}"),
        }
    }
}
