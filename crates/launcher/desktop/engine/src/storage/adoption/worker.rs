//! Retained blocking workers; dropping an observer never aborts confirmation.
use super::*;
use tokio::sync::{oneshot, watch};

pub struct PreviewWorker {
    cancel: CancellationToken,
    pub progress: watch::Receiver<Option<crate::install::Progress>>,
    result: oneshot::Receiver<Result<Preview, Error>>,
}
impl PreviewWorker {
    pub fn request_cancel(&self) {
        self.cancel.cancel();
    }
    /// `None` while the worker runs. A worker that vanished without an answer is
    /// reported as uncertain, never as a clean failure.
    pub fn try_result(&mut self) -> Option<Result<Preview, Error>> {
        match self.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(oneshot::error::TryRecvError::Empty) => None,
            Err(oneshot::error::TryRecvError::Closed) => {
                Some(Err(StorageError::PersistenceUncertain.into()))
            }
        }
    }
    pub async fn wait(&mut self) -> Result<Preview, Error> {
        (&mut self.result)
            .await
            .unwrap_or(Err(StorageError::PersistenceUncertain.into()))
    }
}
impl Drop for PreviewWorker {
    /// Nothing is confirmed yet, so an unobserved preview stops its download or
    /// extraction instead of running on with no owner.
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
pub struct ConfirmationWorker {
    cancel: CancellationToken,
    pub work_id: Uuid,
    pub progress: watch::Receiver<Option<crate::install::Progress>>,
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
    start_preview_backend(state, request, Backend::Native, None)
}

#[cfg(target_os = "macos")]
pub fn start_preview_wine(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    helper: crate::mac_wine::HelperResource,
) -> Result<PreviewWorker, Error> {
    start_preview_backend(state, request, Backend::Wine(helper), None)
}
pub(super) fn start_preview_backend(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    backend: Backend,
    transport: Option<artifacts::Transport>,
) -> Result<PreviewWorker, Error> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let cancel = CancellationToken::new();
    let owned_cancel = cancel.clone();
    let (progress, observed) = crate::install_progress::ProgressSink::latest();
    let (send, result) = oneshot::channel();
    runtime.spawn_blocking(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            preview_using(state, request, owned_cancel, progress, backend, transport)
        }))
        .unwrap_or(Err(StorageError::PersistenceUncertain.into()));
        // An unobserved preview releases its source lock/private temp reference.
        if let Err(Ok(preview)) = send.send(outcome) {
            preview.discard();
        }
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
    start_confirmation_with(preview, work_id, preview_handle, choices, |_| Ok(()))
}
pub(super) fn start_confirmation_with(
    preview: Preview,
    work_id: Uuid,
    preview_handle: Uuid,
    choices: Choices,
    hook: impl FnMut(publication::Point) -> Result<(), Error> + Send + 'static,
) -> Result<ConfirmationWorker, Error> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let cancel = CancellationToken::new();
    let owned_cancel = cancel.clone();
    let (progress, observed) = crate::install_progress::ProgressSink::latest();
    let (send, result) = watch::channel(None);
    runtime.spawn_blocking(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            publication::confirm(
                preview,
                work_id,
                preview_handle,
                choices,
                owned_cancel,
                &progress,
                hook,
            )
        }))
        .unwrap_or(Err(StorageError::PersistenceUncertain.into()));
        send.send_replace(Some(outcome));
    });
    Ok(ConfirmationWorker {
        cancel,
        work_id,
        progress: observed,
        result,
    })
}
