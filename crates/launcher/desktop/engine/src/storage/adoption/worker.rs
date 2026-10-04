//! Retained blocking workers; dropping an observer never aborts confirmation.
use super::*;
use tokio::sync::{oneshot, watch};

pub struct PreviewWorker {
    cancel: CancellationToken,
    pub progress: watch::Receiver<Option<crate::install::Progress>>,
    pub result: oneshot::Receiver<Result<Preview, Error>>,
}
impl PreviewWorker {
    pub fn request_cancel(&self) {
        self.cancel.cancel();
    }
}
pub struct ConfirmationWorker {
    cancel: CancellationToken,
    pub work_id: Uuid,
    pub result: watch::Receiver<Option<Result<Provenance, Error>>>,
}
impl ConfirmationWorker {
    /// A request only; the retained worker persists cancellation when observed.
    /// Once promotion starts, publication must finish or require reconciliation.
    pub fn request_cancel(&self) {
        self.cancel.cancel();
    }
}
pub fn start_preview(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
) -> Result<PreviewWorker, Error> {
    start_preview_backend(state, request, Backend::Native)
}

#[cfg(target_os = "macos")]
pub fn start_preview_wine(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    helper: crate::mac_wine::HelperResource,
) -> Result<PreviewWorker, Error> {
    start_preview_backend(state, request, Backend::Wine(helper))
}
fn start_preview_backend(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    backend: Backend,
) -> Result<PreviewWorker, Error> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let cancel = CancellationToken::new();
    let owned_cancel = cancel.clone();
    let (progress, observed) = crate::install_progress::ProgressSink::latest();
    let (send, result) = oneshot::channel();
    runtime.spawn_blocking(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            preview_using(state, request, owned_cancel, progress, backend)
        }))
        .unwrap_or(Err(StorageError::PersistenceUncertain.into()));
        // An unobserved preview releases its source lock/private temp reference.
        let _ = send.send(outcome);
    });
    Ok(PreviewWorker {
        cancel,
        progress: observed,
        result,
    })
}
pub fn start_confirmation(
    preview: Preview,
    work_id: Uuid,
    preview_handle: Uuid,
    choices: Choices,
) -> Result<ConfirmationWorker, Error> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let cancel = CancellationToken::new();
    let owned_cancel = cancel.clone();
    let (send, result) = watch::channel(None);
    runtime.spawn_blocking(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            confirm(preview, work_id, preview_handle, choices, owned_cancel)
        }))
        .unwrap_or(Err(StorageError::PersistenceUncertain.into()));
        send.send_replace(Some(outcome));
    });
    Ok(ConfirmationWorker {
        cancel,
        work_id,
        result,
    })
}
