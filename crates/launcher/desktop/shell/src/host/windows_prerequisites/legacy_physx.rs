//! NVIDIA's authenticated legacy MSI for adopted copies without the retained seed installer.
use super::*;
use px::Elevate;
// NVIDIA's official Legacy System Software page, downloaded/signature-checked 2026-10-23.
const URL: &str =
    "https://us.download.nvidia.com/Windows/9.13.0604/PhysX-9.13.0604-SystemSoftware-Legacy.msi";
const BYTES: usize = 20_154_368;
const SHA256: &str = "2b78df4a9004f5c4220d62c363709e5ac5e2e58df9f2284df0254cbfa4970d91";
fn error(error: dx::Error) -> px::PhysxError {
    match error {
        dx::Error::PackageMismatch => px::PhysxError::PackageMismatch,
        dx::Error::DownloadFailed => px::PhysxError::DownloadFailed,
        _ => StorageError::Io.into(),
    }
}
impl NativeHost {
    pub(in crate::host) fn install_legacy_physx(&self) -> Result<px::PhysxStatus, px::PhysxError> {
        let state = self.store()?;
        let lease = SetupLease::begin(&state)?;
        tracing::info!(target:"launcher.install",event="physx_started",source="nvidia_legacy","PhysX Legacy setup started");
        let result = (|| {
            let bytes = tokio::runtime::Handle::current()
                .block_on(dx::download_pinned(URL, BYTES, SHA256))
                .map_err(error)?;
            let (path, guard) = dx::stage_pinned(
                &self.root.join("physx-setup"),
                &bytes,
                BYTES,
                SHA256,
                ".msi",
            )
            .map_err(error)?;
            let exit = px::native::ShellExecuteRunas.run_msi(&path);
            drop(guard);
            drop(path);
            Ok::<_, px::PhysxError>(px::outcome(exit, px::detect(&px::native::System)))
        })();
        match &result {
            Ok(result) => {
                tracing::info!(target:"launcher.install",event="physx_finished",outcome=?result,"PhysX Legacy installer finished; detection determines readiness")
            }
            Err(reason) => {
                tracing::warn!(target:"launcher.install",event="physx_failed",reason=?reason,"PhysX Legacy setup could not complete")
            }
        }
        *self.physx_last.lock().map_err(|_| StorageError::Io)? = Some(result?);
        drop(lease);
        self.physx_status(&px::native::System)
    }
}
