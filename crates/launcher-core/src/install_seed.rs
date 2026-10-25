//! Seed-only extraction seam for native platform adapters. Patch overlay stays native.
use super::*;
use crate::manifest::SeedEntry;
use crate::seed_source::{LocalSeedError, VerifiedLocalSeed};
use std::{future::Future, pin::Pin};

/// Native-owned cache is separate from content staging, so the helper can
/// extract into a genuinely fresh destination without multi-file promotion.
/// Callers must bind both paths and backend identity into durable ownership.
pub struct SeedBackend<'a> {
    pub extractor: &'a dyn SeedExtractor,
    pub cache_directory: &'a Path,
}

/// All fields come from the native pipeline and authenticated manifest.
/// The destination must be fresh for initial extraction. An uncertain helper
/// exit returns `SeedExtractionUncertain` and retains input/partial output.
pub struct SeedExtraction<'a> {
    pub archive: &'a Path,
    pub destination: &'a Path,
    pub sha256: &'a str,
    pub cancel: CancellationToken,
    pub progress: ProgressSink,
}

pub trait SeedExtractor: Send + Sync {
    fn extract<'a>(
        &'a self,
        request: SeedExtraction<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), InstallError>> + Send + 'a>>;
}

/// How the staged archive is checked before it is unpacked (OD-3). A
/// download, a cached seed and a patch are hashed here. A local copy was
/// hashed while it was written, so it is not read a second time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArchiveCheck<'a> {
    /// Hash the staged file before unpacking: downloads, the seed cache, patches.
    Sha256(&'a str),
    /// `VerifiedLocalSeed::copy_into` hashed the bytes it wrote, fsynced the
    /// copy and renamed it into place only on a match: no second full read.
    HashedWhileCopied,
}

impl ArchiveCheck<'_> {
    /// The closed code the `seed_unpack_started` log line carries.
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::Sha256(_) => "sha256",
            Self::HashedWhileCopied => "hashed_while_copied",
        }
    }
}

pub(super) async fn apply(
    ctx: &InstallContext<'_>,
    seed: &SeedEntry,
    backend: Option<SeedBackend<'_>>,
) -> Result<(), InstallError> {
    let url = blob_url(ctx.manifest_url, &seed.blob);
    let short_hash = safe_sha_prefix(&seed.sha256)?;
    let download_dir = backend
        .as_ref()
        .map_or(ctx.install_dir, |backend| backend.cache_directory);
    std::fs::create_dir_all(download_dir)?;
    let archive = download_dir.join(format!(".tmp-seed-{short_hash}.download"));
    // The player's own verified archive, when they chose one for this seed,
    // replaces the cache check and the download. Another seed's archive is
    // ignored.
    let check = match ctx.seed_source.current_for(&seed.sha256) {
        Some(local) => copy_local(ctx, &local, &archive).await?,
        None => fetch_seed(ctx, &url, seed, &archive).await?,
    };
    info!("Unpacking seed into {}", ctx.install_dir.display());
    let Some(backend) = backend else {
        info!(target: "launcher.install", event = "seed_unpack_started", check = check.code(), bytes = seed.size, "unpacking the seed archive");
        return verify_and_unpack(
            ctx,
            &archive,
            ctx.install_dir,
            check,
            "seed",
            InstallOptions::STRICT,
            None,
        )
        .await
        .map(|_| ());
    };
    // Authenticate before platform code sees the archive. The helper also
    // authenticates through its own open handle before writing output.
    let file = archive.clone();
    let expected = seed.sha256.clone();
    tokio::task::spawn_blocking(move || {
        let result = verify_sha256(&file, &expected, "seed");
        if matches!(result, Err(InstallError::HashMismatch { .. })) {
            let _ = std::fs::remove_file(file);
        }
        result
    })
    .await
    .map_err(|_| InstallError::Io(std::io::Error::other("seed verification task failed")))??;
    if ctx.cancel.is_cancelled() {
        return Err(InstallError::Cancelled);
    }
    backend
        .extractor
        .extract(SeedExtraction {
            archive: &archive,
            destination: ctx.install_dir,
            sha256: &seed.sha256,
            cancel: ctx.cancel.clone(),
            progress: ctx.progress.clone(),
        })
        .await?;
    // Retain authenticated input on failure/uncertainty for explicit recovery.
    let _ = std::fs::remove_file(archive);
    Ok(())
}

/// Leaves the seed in `archive`: the complete cache when there is one, else a
/// download. If the player switches to a local archive mid-download, the
/// partial file is removed and the local copy is used instead.
async fn fetch_seed<'s>(
    ctx: &InstallContext<'_>,
    url: &str,
    seed: &'s SeedEntry,
    archive: &Path,
) -> Result<ArchiveCheck<'s>, InstallError> {
    // A complete, authenticated cache can survive an interrupted attempt.
    // Reuse it even when the archive host is unavailable; extraction rechecks it.
    let cached = archive.to_path_buf();
    let expected = seed.sha256.clone();
    let size = seed.size;
    let cancel = ctx.cancel.clone();
    let complete = tokio::task::spawn_blocking(move || -> Result<bool, InstallError> {
        if cancel.is_cancelled() {
            return Err(InstallError::Cancelled);
        }
        let meta = match std::fs::symlink_metadata(&cached) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if !meta.is_file() {
            return Err(std::io::Error::other("seed cache is not a regular file").into());
        }
        if meta.len() != size {
            return Ok(false);
        }
        match verify_sha256(&cached, &expected, "seed") {
            Ok(()) => Ok(true),
            Err(InstallError::HashMismatch { .. }) => {
                if cancel.is_cancelled() {
                    return Err(InstallError::Cancelled);
                }
                // A full-size corrupt file cannot resume: Range at EOF would
                // receive 416 and keep failing. Fetch clean bytes from zero.
                std::fs::remove_file(cached)?;
                Ok(false)
            }
            Err(error) => Err(error),
        }
    })
    .await
    .map_err(|_| InstallError::Io(std::io::Error::other("seed cache verification failed")))??;
    if ctx.cancel.is_cancelled() {
        return Err(InstallError::Cancelled);
    }
    if complete {
        return Ok(ArchiveCheck::Sha256(&seed.sha256));
    }
    loop {
        let switch = ctx.seed_source.switched();
        match download_to_file(
            ctx.http,
            url,
            archive,
            ctx.cancel.clone(),
            switch,
            seed.size,
            "seed",
            &ctx.progress,
        )
        .await
        {
            Err(InstallError::SeedSwitched) => {
                let _ = std::fs::remove_file(archive);
                match ctx.seed_source.resume_for(&seed.sha256) {
                    Some(local) => return copy_local(ctx, &local, archive).await,
                    // An archive for another seed does not stop this download.
                    None => continue,
                }
            }
            other => return other.map(|()| ArchiveCheck::Sha256(&seed.sha256)),
        }
    }
}

/// Copies the player's verified archive to `archive`. The copy is checked
/// again on the way (identity, size, hash), so a file changed since the pick
/// is refused here.
async fn copy_local(
    ctx: &InstallContext<'_>,
    local: &VerifiedLocalSeed,
    archive: &Path,
) -> Result<ArchiveCheck<'static>, InstallError> {
    let local = local.clone();
    let dest = archive.to_path_buf();
    let cancel = ctx.cancel.clone();
    let progress = ctx.progress.clone();
    let copied = tokio::task::spawn_blocking(move || local.copy_into(&dest, &cancel, &progress))
        .await
        .map_err(|_| InstallError::Io(std::io::Error::other("local seed copy task failed")))?;
    copied
        .map(|()| ArchiveCheck::HashedWhileCopied)
        .map_err(|error| {
        if !matches!(error, LocalSeedError::Cancelled) {
            // The player moved, changed or deleted the file after choosing it.
            warn!(target: "launcher.install", event = "seed_copy_failed", reason = error.code(), "the chosen seed archive could not be copied; the next attempt downloads it");
            ctx.seed_source.note_local_failure();
        }
        InstallError::from(error)
    })
}

#[cfg(test)]
#[path = "install_seed_tests.rs"]
mod tests;
