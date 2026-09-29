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

    for patch in &ctx.manifest.patches {
        let key = patch.state_key();
        if state.has_applied(&key) {
            continue;
        }
        apply_patch(&ctx, patch).await?;
        state.applied_patches.push(key);
        state.save(ctx.install_dir)?;
    }

    if install_layout::sgw_exe(ctx.install_dir).is_file() {
        let report = crate::client_setup::prepare(ctx.install_dir, ctx.login_servers)?;
        info!(?report, "Client setup done");
    } else {
        warn!("SGW.exe missing after install — verify the seed manifest entry");
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

    // Regression guard for the non-panicking HTTP-status branch. An
    // earlier version called `error_for_status_ref().unwrap_err()` here,
    // which panicked on stray 1xx/3xx because that helper only returns
    // `Err` for 4xx/5xx; the current code returns
    // `InstallError::UnexpectedStatus` carrying the status + url.
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

    fn unpack_ctx_parts() -> (
        tokio::sync::mpsc::UnboundedSender<Progress>,
        tokio::sync::mpsc::UnboundedReceiver<Progress>,
    ) {
        tokio::sync::mpsc::unbounded_channel()
    }

    // Bug shape: a download that failed its hash was kept, so every retry
    // resumed from the bad bytes (or got 416) and failed the same way until
    // someone deleted the .tmp file by hand. It must be deleted.
    #[tokio::test]
    async fn hash_mismatch_deletes_the_download() {
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join(".tmp-seed-abc.download");
        std::fs::write(&tmp, b"corrupt").unwrap();
        let (tx, _rx) = unpack_ctx_parts();
        let manifest = fake_manifest("00");
        let http = reqwest::Client::new();
        let ctx = InstallContext {
            manifest_url: "https://example.invalid/manifest.json",
            install_dir: dir.path(),
            manifest: &manifest,
            login_servers: &[],
            cancel: CancellationToken::new(),
            progress: tx,
            http: &http,
        };
        let err = verify_and_unpack(&ctx, &tmp, ctx.install_dir, "00", "seed")
            .await
            .unwrap_err();
        assert!(matches!(err, InstallError::HashMismatch { .. }), "{err:?}");
        assert!(!tmp.exists(), "a bad download must not be resumed");
    }

    // A full-length download left over from a run that stopped before
    // verifying gets 416 for its Range request. That is "already
    // downloaded", not an error; the hash check decides.
    #[tokio::test]
    async fn download_to_file_treats_416_on_a_complete_file_as_done() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/seed.rar"))
            .respond_with(ResponseTemplate::new(416))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(".tmp-seed.download");
        std::fs::write(&dest, b"0123456789").unwrap();
        let (tx, _rx) = unpack_ctx_parts();
        let url = format!("{}/seed.rar", server.uri());
        download_to_file(
            &reqwest::Client::new(),
            &url,
            &dest,
            CancellationToken::new(),
            10,
            "seed",
            &tx,
        )
        .await
        .expect("416 on a complete file is success");
        assert_eq!(std::fs::read(&dest).unwrap(), b"0123456789");

        // A short partial file getting 416 is still an error.
        std::fs::write(&dest, b"01234").unwrap();
        let err = download_to_file(
            &reqwest::Client::new(),
            &url,
            &dest,
            CancellationToken::new(),
            10,
            "seed",
            &tx,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            InstallError::UnexpectedStatus { status: 416, .. }
        ));
    }

    #[test]
    fn safe_sha_prefix_accepts_lower_and_upper_hex() {
        assert_eq!(safe_sha_prefix("abcdef0123456789").unwrap(), "abcdef012345");
        assert_eq!(safe_sha_prefix("ABCDEF0123456789").unwrap(), "ABCDEF012345");
    }

    #[test]
    fn safe_sha_prefix_rejects_path_traversal() {
        // The bug shape: a malformed-but-still-signed manifest with a
        // sha256 of "../foo" would, prior to validation, produce a tmp
        // file path that escapes the install directory.
        for bad in &[
            "../foo",
            "/etc/passwd",
            "..\\evil",
            "0123/4567",
            "abc def",
            "",
            "abcg",
        ] {
            let err = safe_sha_prefix(bad).expect_err(&format!("expected reject for {bad:?}"));
            assert!(matches!(err, InstallError::InvalidSha256(_)));
        }
    }

    #[test]
    fn safe_sha_prefix_short_input_passes() {
        // Anything fewer than 12 chars still passes if every char is hex
        // — the prefix is min(len, 12). A real sha256 is always 64 chars
        // so this only matters for malformed manifests; reject those
        // separately via the all-hex check rather than a length check.
        assert_eq!(safe_sha_prefix("abc").unwrap(), "abc");
    }

    fn fake_manifest(seed_hash: &str) -> crate::manifest::Manifest {
        crate::manifest::Manifest {
            schema: 1,
            seed: crate::manifest::SeedEntry {
                blob: "seed/x.zip".into(),
                size: 1,
                sha256: seed_hash.into(),
            },
            patches: vec![],
        }
    }

    // Happy path: directory has SGW.exe, no prior launcher-installed.json
    // → adopt writes the marker file with seed_adopted=true and copies
    // the manifest seed hash. Subsequent Install/Update would then apply
    // patches on top without re-downloading the seed.
    #[test]
    fn adopt_existing_install_writes_marker_when_sgw_exe_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"fake-game-binary").unwrap();
        let manifest = fake_manifest("seed-hash-from-manifest");
        let state = adopt_existing_install(dir.path(), &manifest).unwrap();
        assert!(state.seed_adopted);
        assert_eq!(
            state.seed_sha256.as_deref(),
            Some("seed-hash-from-manifest")
        );
        assert!(state.applied_patches.is_empty());
        // Persistence path: load-back must produce the same shape so a
        // restart of the launcher sees the adopted install.
        let loaded = InstalledState::load(dir.path());
        assert!(loaded.seed_adopted);
        assert_eq!(
            loaded.seed_sha256.as_deref(),
            Some("seed-hash-from-manifest")
        );
    }

    // Empty directory: NoGameExe rejects with a path-naming error so the
    // UI can surface "this isn't a game install" instead of silently
    // marking an empty directory as adopted.
    #[test]
    fn adopt_existing_install_rejects_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = fake_manifest("h");
        let err = adopt_existing_install(dir.path(), &manifest).unwrap_err();
        match err {
            AdoptError::NoGameExe(p) => assert_eq!(p, dir.path()),
            other => panic!("expected NoGameExe, got {other:?}"),
        }
    }

    // SGW.exe as a directory (or junction) must not trick adopt into
    // marking a non-install as managed. is_file() is the gate.
    #[test]
    fn adopt_existing_install_rejects_when_sgw_exe_is_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("SGW.exe")).unwrap();
        let manifest = fake_manifest("h");
        let err = adopt_existing_install(dir.path(), &manifest).unwrap_err();
        assert!(matches!(err, AdoptError::NoGameExe(_)));
    }

    // Adopt-over-managed: the second call refuses because overwriting
    // a real install's state would silently discard the applied-patches
    // list. User must delete the marker
    // by hand to re-adopt.
    #[test]
    fn adopt_existing_install_rejects_already_managed_install() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        let manifest = fake_manifest("h");
        adopt_existing_install(dir.path(), &manifest).unwrap();
        let err = adopt_existing_install(dir.path(), &manifest).unwrap_err();
        assert!(matches!(err, AdoptError::AlreadyManaged(_)));
    }
}
