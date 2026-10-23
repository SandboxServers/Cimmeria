//! Explicit download/install of NVIDIA's legacy MSI when the required core is
//! absent. This policy is native-owned; IPC cannot select an endpoint or hash.
use super::{Elevate, InstallResult, PhysxError, Probe};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};

const URL: &str =
    "https://us.download.nvidia.com/Windows/9.13.0604/PhysX-9.13.0604-SystemSoftware-Legacy.msi";
const BYTES: usize = 20_154_368;
const SHA256: &str = "2b78df4a9004f5c4220d62c363709e5ac5e2e58df9f2284df0254cbfa4970d91";

/// Called on the shell's blocking worker, after mutation admission and only
/// after a player press. The interactive MSI owns license acceptance and UAC.
pub fn install(
    scratch: &Path,
    elevate: &dyn Elevate,
    probe: &dyn Probe,
) -> Result<InstallResult, PhysxError> {
    let started = std::time::Instant::now();
    tracing::info!(target: "launcher.install", event = "physx_started", action = "install_legacy", "Legacy PhysX download and install started");
    let result = (|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| PhysxError::DownloadFailed)?;
        let bytes = runtime.block_on(download(URL))?;
        install_bytes(scratch, &bytes, elevate, probe)
    })();
    let duration_ms = started.elapsed().as_millis() as u64;
    match &result {
        Ok(InstallResult::Installed { .. }) => {
            tracing::info!(target: "launcher.install", event = "physx_finished", action = "install_legacy", outcome = "installed", duration_ms, "Legacy PhysX installed")
        }
        Ok(other) => {
            tracing::warn!(target: "launcher.install", event = "physx_failed", action = "install_legacy", reason = super::result_name(other), duration_ms, "Legacy PhysX installation did not complete")
        }
        Err(error) => {
            let reason = match error {
                PhysxError::DownloadFailed => "download_failed",
                PhysxError::PackageMismatch => "package_mismatch",
                _ => "preparation_failed",
            };
            tracing::warn!(target: "launcher.install", event = "physx_failed", action = "install_legacy", reason, duration_ms, "Legacy PhysX installation refused");
        }
    }
    result
}

fn verify(bytes: &[u8]) -> Result<(), PhysxError> {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if bytes.len() != BYTES || digest != SHA256 {
        return Err(PhysxError::PackageMismatch);
    }
    Ok(())
}

fn install_bytes(
    scratch: &Path,
    bytes: &[u8],
    elevate: &dyn Elevate,
    probe: &dyn Probe,
) -> Result<InstallResult, PhysxError> {
    verify(bytes)?;
    super::install_msi(scratch, bytes, elevate, probe)
}

async fn download(url: &str) -> Result<Vec<u8>, PhysxError> {
    let failed = |_| PhysxError::DownloadFailed;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(failed)?;
    let response = client.get(url).send().await.map_err(failed)?;
    if response.status() != reqwest::StatusCode::OK
        || response
            .content_length()
            .is_some_and(|len| len != BYTES as u64)
    {
        return Err(PhysxError::DownloadFailed);
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(failed)?;
        if chunk.len() > BYTES.saturating_sub(bytes.len()) {
            return Err(PhysxError::DownloadFailed);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
