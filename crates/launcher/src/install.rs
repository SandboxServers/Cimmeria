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
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::install_layout;
use crate::install_progress::{ProgressReporter, ProgressSink};
use crate::install_report::{InstallReport, PatchOutcomeKind};
use crate::manifest::{blob_url, Manifest, PatchEntry, SeedEntry};
use crate::patch_dest::patch_dest;
use crate::state::{InstalledState, StateError};
use crate::unpack::{self, UnpackError, UnpackSink};

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unpack error: {0}")]
    Unpack(#[from] UnpackError),
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
    #[error(transparent)]
    PatchDest(#[from] crate::patch_dest::NoSgwGameDir),
    /// Some patches failed; every other patch was still applied.
    #[error("{}", patches_failed_message(.0))]
    PatchesFailed(Vec<PatchFailure>),
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
    pub login_servers: &'a [crate::client_setup::LoginServer],
    pub cancel: CancellationToken,
    pub progress: ProgressSink,
    /// Shared HTTP client owned by the worker — reused across the
    /// seed download, every patch download, and the post-install
    /// manifest revalidation. Avoids rebuilding the connection pool
    /// per blob. Configured with `https_only(true)`.
    pub http: &'a reqwest::Client,
}

/// Install the seed and every patch, and report what happened to each
/// patch (for the install-result telemetry event).
pub async fn install_all(ctx: InstallContext<'_>) -> (Result<(), InstallError>, InstallReport) {
    let mut report = InstallReport::default();
    let result = install_all_into(ctx, &mut report).await;
    report.finish(&result);
    (result, report)
}

async fn install_all_into(
    ctx: InstallContext<'_>,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    std::fs::create_dir_all(ctx.install_dir)?;
    let mut state = InstalledState::load(ctx.install_dir);

    let seed_matches = state.seed_sha256.as_deref() == Some(ctx.manifest.seed.sha256.as_str());
    if !seed_matches {
        apply_seed(&ctx, &ctx.manifest.seed).await?;
        report.seed_applied = true;
        // Re-seeding invalidates the applied-patches list.
        state = InstalledState {
            seed_sha256: Some(ctx.manifest.seed.sha256.clone()),
            applied_patches: Vec::new(),
            seed_adopted: false,
        };
        state.save(ctx.install_dir)?;
    }

    // After a fresh seed, and for adopted installs that never had it: the
    // patches below expect the bundled PAKs in SourceCache.en-us.
    if install_layout::place_bundled_cooked_data(ctx.install_dir)? {
        info!("Moved the bundled cooked-data PAKs to Working\\SGWGame\\SourceCache.en-us");
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
            warn!(patch = %patch.id, after = %after, reason = "dependency_failed", "patch skipped");
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
                warn!(patch = %patch.id, error = %e, reason = "apply_failed", "patch not applied");
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
        warn!("SGW.exe missing after install — verify the seed manifest entry");
    }

    if !failures.is_empty() {
        return Err(InstallError::PatchesFailed(failures));
    }
    Ok(())
}

async fn apply_seed(ctx: &InstallContext<'_>, seed: &SeedEntry) -> Result<(), InstallError> {
    let url = blob_url(ctx.manifest_url, &seed.blob);
    let short_hash = safe_sha_prefix(&seed.sha256)?;
    let tmp = ctx
        .install_dir
        .join(format!(".tmp-seed-{short_hash}.download"));
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
    info!("Unpacking seed into {}", ctx.install_dir.display());
    verify_and_unpack(ctx, &tmp, ctx.install_dir, &seed.sha256, "seed").await
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

pub(crate) async fn download_to_file(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
    cancel: CancellationToken,
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
    let resp = req.send().await?;

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
    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(InstallError::Cancelled);
        }
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

pub(crate) fn hash_file(path: &Path) -> std::io::Result<String> {
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

/// Outcome of an "Adopt existing install" attempt — the launcher
/// inspects the install directory and decides whether the user's
/// pre-existing copy of the game looks plausible enough to mark as
/// managed (skipping the full seed re-download) and let patches apply
/// on top.
#[derive(Debug, Error)]
pub enum AdoptError {
    /// The install directory doesn't contain `SGW.exe`. Either the path
    /// is wrong or this is a genuinely empty install — the user should
    /// run a normal Install / Update instead.
    #[error(
        "No SGW.exe found in {0} — this directory doesn't look like a Stargate Worlds install. \
         Point the install path at a directory containing SGW.exe and try again, \
         or run Install / Update to download a fresh copy."
    )]
    NoGameExe(PathBuf),
    /// `launcher-installed.json` already exists. Adopt is a one-way
    /// transition from "unmanaged by the launcher" → "managed"; once
    /// the launcher has state for this install we don't need to adopt
    /// it again.
    #[error("Install at {0} is already managed by the launcher — nothing to adopt.")]
    AlreadyManaged(PathBuf),
    /// Persisting the marker file failed.
    #[error("State error: {0}")]
    State(#[from] StateError),
}

/// Mark a pre-existing game install as managed by the launcher without
/// downloading / re-extracting the seed.
///
/// **Trust model:** we record `manifest.seed.sha256` as the install's
/// `seed_sha256` *without* hashing on-disk bytes. The seed blob is
/// gigabytes; rehashing it at adopt-time would freeze the UI for
/// minutes. Instead we set `seed_adopted = true` so the install state
/// carries an "unverified" flag that the UI surfaces. If the user's
/// existing files don't match what the manifest's seed would have
/// produced, patches may corrupt them or no-op surprisingly — the
/// confirm dialog in the UI makes this trade-off explicit.
///
/// Idempotency: refuses to overwrite an existing `launcher-installed.json`
/// (returns [`AdoptError::AlreadyManaged`]). The user must remove the
/// state file by hand to re-adopt, which is the right behaviour because
/// adopt-over-managed would silently discard the real `applied_patches`
/// list.
pub fn adopt_existing_install(
    install_dir: &Path,
    manifest: &crate::manifest::Manifest,
) -> Result<InstalledState, AdoptError> {
    let exe = install_layout::sgw_exe(install_dir);
    // `is_file` rather than `exists` so a directory or junction named
    // SGW.exe doesn't fool us into adopting a non-install.
    if !exe.is_file() {
        return Err(AdoptError::NoGameExe(install_dir.to_path_buf()));
    }
    if InstalledState::path(install_dir).exists() {
        return Err(AdoptError::AlreadyManaged(install_dir.to_path_buf()));
    }
    let state = InstalledState {
        seed_sha256: Some(manifest.seed.sha256.clone()),
        applied_patches: Vec::new(),
        seed_adopted: true,
    };
    state.save(install_dir)?;
    info!(
        install_dir = %install_dir.display(),
        seed_sha = %manifest.seed.sha256,
        "Adopted existing install (unverified — seed bytes not hashed)"
    );
    Ok(state)
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
