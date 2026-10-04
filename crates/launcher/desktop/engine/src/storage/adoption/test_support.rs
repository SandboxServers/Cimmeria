//! Explicit test entry points for a loopback signed-artifact origin. Production
//! entry points keep the fixed HTTPS catalog and cannot reach these.
use super::*;

/// Portable ZIP backend; the manifest URL must be a literal loopback origin.
pub fn start_preview(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    manifest_url: String,
) -> Result<PreviewWorker, Error> {
    worker::start_preview_backend(
        state,
        request,
        Backend::Native,
        Some(artifacts::Transport::loopback(manifest_url)?),
    )
}
/// The production Wine helper backend against a loopback artifact origin.
#[cfg(target_os = "macos")]
pub fn start_preview_wine(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    helper: crate::mac_wine::HelperResource,
    manifest_url: String,
) -> Result<PreviewWorker, Error> {
    worker::start_preview_backend(
        state,
        request,
        Backend::Wine(helper),
        Some(artifacts::Transport::loopback(manifest_url)?),
    )
}

/// Where a host test interrupts or pauses the retained copy.
pub enum CopyFault {
    /// Wait where the copy starts until the sender fires or is dropped.
    Hold(std::sync::mpsc::Receiver<()>),
    /// Fail before any staged checkpoint exists: only abandonment remains.
    BeforeCheckpoint,
    /// Fail after promotion: explicit recovery can finish publication.
    AfterPromotion,
}
pub fn start_confirmation(
    preview: Preview,
    work_id: Uuid,
    preview_handle: Uuid,
    choices: Choices,
    fault: CopyFault,
) -> Result<ConfirmationWorker, Error> {
    use publication::Point;
    worker::start_confirmation_with(
        preview,
        work_id,
        preview_handle,
        choices,
        move |point| match (&fault, point) {
            (CopyFault::Hold(resume), Point::Plan) => {
                let _ = resume.recv();
                Ok(())
            }
            (CopyFault::BeforeCheckpoint, Point::Plan)
            | (CopyFault::AfterPromotion, Point::AfterPromotion) => Err(StorageError::Io.into()),
            _ => Ok(()),
        },
    )
}
/// The durable result of reference work that stopped without a certain outcome,
/// as a failed helper leaves it: a retained record and directory under an
/// operation that requires reconciliation. Nothing was extracted.
pub fn interrupted_preparation(
    state: Arc<Mutex<DesktopState>>,
    request: &PreviewRequest,
) -> Result<Uuid, Error> {
    let mut owner =
        preparation::Ownership::claim(state, request, ExtractionBackend::Native, Vec::new())?;
    owner.uncertain = true;
    owner.release();
    Ok(owner.record.id)
}
