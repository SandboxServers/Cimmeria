//! Seed-only extraction seam for native platform adapters. Patch overlay stays native.
use super::*;
use crate::manifest::SeedEntry;
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
    download_to_file(
        ctx.http,
        &url,
        &archive,
        ctx.cancel.clone(),
        seed.size,
        "seed",
        &ctx.progress,
    )
    .await?;
    info!("Unpacking seed into {}", ctx.install_dir.display());
    let Some(backend) = backend else {
        return verify_and_unpack(ctx, &archive, ctx.install_dir, &seed.sha256, "seed").await;
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

#[cfg(test)]
#[path = "install_seed_tests.rs"]
mod tests;
