//! Seed + patch download and extraction pipeline.
//!
//! Flow:
//! 1. If `InstalledState.seed_sha256 != manifest.seed.sha256`: download the
//!    seed archive (resumable via HTTP Range), verify sha256, unpack it into
//!    the install dir, then reset the applied-patches list. The seed may be a
//!    zip, or a RAR holding the original installer's cabinet set; see
//!    [`crate::unpack`].
//! 2. For each patch in declared order: skip if already applied; else
//!    download → verify → unpack (overlay-style) → record in state. A
//!    patch unpacks into the install dir, or into the client's
//!    `SGWGame/` directory when its manifest entry says `"root":
//!    "sgw_game"` (see [`crate::patch_dest`]). A patch zip carrying a
//!    `cimmeria-patch.json` recipe rebuilds files from the player's own
//!    stock files by delta (`cimmeria-patchset`).
//! 3. Client setup ([`crate::client_setup`]): put back the stock spelling
//!    of patched files (`EULA.lua`), write `LoginInternal.lua` from the
//!    configured login servers and switch ASLR off in SGW.exe.
//!    Idempotent; it also runs before every launch.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

#[path = "install_seed.rs"]
mod seed;
pub use seed::{SeedBackend, SeedExtraction, SeedExtractor};

use crate::install_layout;
use crate::install_progress::{ProgressReporter, ProgressSink};
use crate::install_report::{InstallReport, PatchOutcomeKind};
use crate::manifest::{blob_url, Manifest, PatchEntry};
use crate::patch_dest::patch_dest;
use crate::seed_source::{LocalSeedError, SeedSourceHandle};
use crate::state::{InstalledState, StateError};
use crate::unpack::{self, UnpackError, UnpackSink};

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unpack error: {0}")]
    Unpack(#[source] UnpackError),
    #[error("State error: {0}")]
    State(#[from] StateError),
    #[error("Hash mismatch for {what}: expected {expected}, got {actual}")]
    HashMismatch {
        what: String,
        expected: String,
        actual: String,
    },
    #[error("Unexpected HTTP {status} for {url}")]
    UnexpectedStatus { status: u16, url: String },
    #[error("Manifest sha256 is not valid hex: {0:?}")]
    InvalidSha256(String),
    #[error("Cancelled")]
    Cancelled,
    /// A running seed download was switched to a local archive; the caller
    /// removes the partial download and copies that archive instead.
    #[error("seed switched to a local archive")]
    SeedSwitched,
    /// The player's chosen seed archive cannot be used.
    #[error("local seed archive: {0}")]
    LocalSeed(#[source] LocalSeedError),
    #[error("External seed extraction requires reconciliation")]
    SeedExtractionUncertain,
    #[error(transparent)]
    PatchDest(#[from] crate::patch_dest::NoSgwGameDir),
    /// Some patches failed; every other patch was still applied.
    #[error("{}", patches_failed_message(.0))]
    PatchesFailed(Vec<PatchFailure>),
}

impl From<UnpackError> for InstallError {
    fn from(error: UnpackError) -> Self {
        match error {
            UnpackError::Cancelled => Self::Cancelled,
            other => Self::Unpack(other),
        }
    }
}

impl From<LocalSeedError> for InstallError {
    fn from(error: LocalSeedError) -> Self {
        match error {
            LocalSeedError::Cancelled => Self::Cancelled,
            other => Self::LocalSeed(other),
        }
    }
}

/// One patch that did not apply, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchFailure {
    pub id: String,
    pub reason: String,
}

fn patches_failed_message(failures: &[PatchFailure]) -> String {
    let list: Vec<String> = failures
        .iter()
        .map(|f| format!("{}: {}", f.id, f.reason))
        .collect();
    format!(
        "{} patch(es) not applied, the others were: {}",
        failures.len(),
        list.join("; ")
    )
}

/// A patch whose `after` names a patch that did not apply this run is
/// skipped: it was built on top of that patch.
fn blocked_by_failure(patch: &PatchEntry, failures: &[PatchFailure]) -> Option<String> {
    let after = patch.after.as_deref()?;
    failures
        .iter()
        .any(|f| f.id == after)
        .then(|| after.to_string())
}

/// Returns the first 12 chars of `sha` after confirming the whole string
/// is ASCII hex. Manifest fields are external input (the signature
/// verifies the *bytes* of the manifest, not the well-formedness of
/// individual fields), and `sha256` ends up spliced into temp-file
/// paths — splicing `../foo` into a path fragment escapes the install
/// directory. Validate at the boundary.
fn safe_sha_prefix(sha: &str) -> Result<&str, InstallError> {
    if sha.is_empty() || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(InstallError::InvalidSha256(sha.to_string()));
    }
    Ok(&sha[..12.min(sha.len())])
}

/// Progress events emitted during install, forwarded to the UI through an
/// [`crate::install_progress`] reporter.
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
    pub login_servers: &'a [crate::client_setup::LoginServer],
    pub cancel: CancellationToken,
    pub progress: ProgressSink,
    /// Shared HTTP client owned by the worker — reused across the
    /// seed download, every patch download, and the post-install
    /// manifest revalidation. Avoids rebuilding the connection pool
    /// per blob. Configured with `https_only(true)`.
    pub http: &'a reqwest::Client,
    /// Which archive the seed step uses: a download, or the player's own
    /// verified copy. The desktop engine wires its real handle; others pass
    /// `&SeedSourceHandle::download()`.
    pub seed_source: &'a SeedSourceHandle,
}

/// Install the seed and every patch, and report what happened to each
/// patch (for the install-result telemetry event).
pub async fn install_all(ctx: InstallContext<'_>) -> (Result<(), InstallError>, InstallReport) {
    let mut report = InstallReport::default();
    let result = install_all_into(ctx, &mut report, None).await;
    report.finish(&result);
    (result, report)
}

/// Native platform adapters may replace only seed extraction. Downloads, hashes,
/// installed-state transitions, patch ordering and preparation remain shared.
pub async fn install_all_with_seed_extractor(
    ctx: InstallContext<'_>,
    backend: SeedBackend<'_>,
) -> (Result<(), InstallError>, InstallReport) {
    let mut report = InstallReport::default();
    let result = install_all_into(ctx, &mut report, Some(backend)).await;
    report.finish(&result);
    (result, report)
}

async fn install_all_into(
    ctx: InstallContext<'_>,
    report: &mut InstallReport,
    backend: Option<SeedBackend<'_>>,
) -> Result<(), InstallError> {
    if backend.is_none() {
        std::fs::create_dir_all(ctx.install_dir)?;
    }
    let mut state = InstalledState::load(ctx.install_dir);

    let seed_matches = state.seed_sha256.as_deref() == Some(ctx.manifest.seed.sha256.as_str());
    if !seed_matches {
        seed::apply(&ctx, &ctx.manifest.seed, backend).await?;
        report.seed_applied = true;
        // Re-seeding invalidates the applied-patches list.
        state = InstalledState {
            seed_sha256: Some(ctx.manifest.seed.sha256.clone()),
            applied_patches: Vec::new(),
            seed_adopted: false,
            ..Default::default()
        };
        state.save(ctx.install_dir)?;
    }

    // After a fresh seed, and for adopted installs that never had it: the
    // patches below expect the bundled PAKs in SourceCache.en-us.
    if install_layout::place_bundled_cooked_data(ctx.install_dir)? {
        info!(target: "launcher.install", event = "bundled_paks_moved", "moved the bundled cooked-data PAKs into SourceCache");
    }

    // One patch that cannot apply (a hand-edited file an adopted install
    // carries, say) must not cost the player every patch after it: apply
    // the rest and report the failures together at the end.
    let mut failures: Vec<PatchFailure> = Vec::new();
    for patch in &ctx.manifest.patches {
        if state.has_applied_patch(patch) {
            report.push(&patch.id, PatchOutcomeKind::Already, None);
            continue;
        }
        if let Some(after) = blocked_by_failure(patch, &failures) {
            warn!(target: "launcher.install", event = "patch_skipped", patch = %patch.id, reason = "dependency_failed", "patch skipped: a patch it builds on did not apply");
            let reason = format!("skipped, it builds on {after}, which did not apply");
            report.push(
                &patch.id,
                PatchOutcomeKind::SkippedDependency,
                Some(reason.clone()),
            );
            failures.push(PatchFailure {
                id: patch.id.clone(),
                reason,
            });
            continue;
        }
        match apply_patch(&ctx, patch).await {
            Ok(()) => {
                state.applied_patches.push(patch.state_key());
                state.save(ctx.install_dir)?;
                report.push(&patch.id, PatchOutcomeKind::Applied, None);
            }
            Err(InstallError::Cancelled) => return Err(InstallError::Cancelled),
            Err(e) => {
                warn!(target: "launcher.install", event = "patch_failed", patch = %patch.id, reason = e.code(), http_status = e.http_status(), "patch not applied");
                report.push_failure(&patch.id, &e);
                failures.push(PatchFailure {
                    id: patch.id.clone(),
                    reason: e.to_string(),
                });
            }
        }
    }

    if install_layout::sgw_exe(ctx.install_dir).is_file() {
        let report = crate::client_setup::prepare(ctx.install_dir, ctx.login_servers)?;
        info!(?report, "Client setup done");
    } else {
        warn!(target: "launcher.install", event = "game_exe_missing", "SGW.exe missing after install");
    }

    if !failures.is_empty() {
        return Err(InstallError::PatchesFailed(failures));
    }
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
    // Include a sha prefix in the tmp filename so a republished patch
    // with the same `id` but a new sha256 doesn't accidentally resume
    // against the stale bytes — different sha → different tmp path →
    // fresh download. `safe_sha_prefix` rejects non-hex characters at
    // this boundary so a malformed manifest can't escape `install_dir`
    // via crafted sha256 strings.
    let safe_sha = safe_sha_prefix(&patch.sha256)?;
    let tmp = ctx
        .install_dir
        .join(format!(".tmp-patch-{safe_id}-{safe_sha}.download"));
    let label = format!("patch {}", patch.id);
    // Resolve the destination before downloading, so a client tree with
    // no SGWGame directory fails fast instead of after the download.
    let dest = patch_dest(ctx.install_dir, patch)?;
    download_to_file(
        ctx.http,
        &url,
        &tmp,
        ctx.cancel.clone(),
        // Patches never switch to a local archive.
        CancellationToken::new(),
        patch.size,
        &label,
        &ctx.progress,
    )
    .await?;
    info!("Applying {} into {}", label, dest.display());
    verify_and_unpack(ctx, &tmp, &dest, &patch.sha256, &label).await
}

/// Hash `tmp` against `sha256`, unpack it into `dest`, then delete
/// it. Both steps are blocking file work over multi-gigabyte files, so they
/// run on a blocking thread.
///
/// A download that fails its hash is deleted. Keeping it would make the next
/// attempt resume from the bad bytes (or get HTTP 416 for a full-length
/// file) and fail the same way forever.
async fn verify_and_unpack(
    ctx: &InstallContext<'_>,
    tmp: &Path,
    dest: &Path,
    sha256: &str,
    label: &str,
) -> Result<(), InstallError> {
    let tmp = tmp.to_path_buf();
    let dest = dest.to_path_buf();
    let expected = sha256.to_string();
    let sink = UnpackSink {
        progress: ctx.progress.clone(),
        label: label.to_string(),
        cancel: ctx.cancel.clone(),
    };
    let label = label.to_string();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = verify_sha256(&tmp, &expected, &label) {
            if matches!(e, InstallError::HashMismatch { .. }) {
                let _ = std::fs::remove_file(&tmp);
            }
            return Err(e);
        }
        unpack::unpack(&tmp, &dest, &sink)?;
        let _ = std::fs::remove_file(&tmp);
        Ok(())
    })
    .await
    .map_err(|e| InstallError::Io(std::io::Error::other(format!("unpack task failed: {e}"))))?
}

/// Downloads `url` to `dest`, resuming a partial file there with a `Range` request.
///
/// Firing `switch` stops the download and returns [`InstallError::SeedSwitched`];
/// the partial file is left for the caller to remove.
pub async fn download_to_file(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
    cancel: CancellationToken,
    switch: CancellationToken,
    expected_size: u64,
    label: &str,
    progress: &impl ProgressReporter,
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
    let resp = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(InstallError::Cancelled),
        _ = switch.cancelled() => return Err(InstallError::SeedSwitched),
        response = req.send() => response?,
    };

    let status = resp.status();
    // 416 on a Range request for a file we already hold in full: the
    // previous run finished the download but stopped before verifying it.
    // Hand it to the hash check, which deletes it if it is bad.
    if status.as_u16() == 416 && existing_len > 0 && existing_len >= expected_size {
        return Ok(());
    }
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
    loop {
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(InstallError::Cancelled),
            _ = switch.cancelled() => return Err(InstallError::SeedSwitched),
            chunk = stream.next() => chunk,
        };
        let Some(chunk) = chunk else {
            break;
        };
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        // Throttle progress emits so the UI channel isn't flooded by tiny chunks.
        if last_emit.elapsed() >= std::time::Duration::from_millis(33) {
            progress.report(Progress::Downloading {
                label: label.to_string(),
                downloaded,
                total,
            });
            last_emit = std::time::Instant::now();
        }
    }
    file.flush().await?;
    progress.report(Progress::Downloading {
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

/// Lower-case hex SHA-256 of the file at `path`.
pub fn hash_file(path: &Path) -> std::io::Result<String> {
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
    // digest 0.11's `Array` output no longer implements `LowerHex`, so the old
    // `{:x}` no longer compiles — hex-encode the bytes explicitly.
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
