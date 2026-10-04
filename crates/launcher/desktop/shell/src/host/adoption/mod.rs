//! Verified-copy adoption IPC. Destinations, releases and artifacts stay native.
//!
//! Lock order: the adoption mutex, then the store. Retained engine workers take
//! only the store, so nothing takes the two in the other order. A native preview
//! is released outside the adoption mutex, because releasing waits for the store.
use super::*;
use cimmeria_launcher_engine::{
    adoption::{self, Choices},
    catalog::VerifiedRelease,
};
#[cfg(test)]
use cimmeria_launcher_engine::{OperationKind, OperationState};
use uuid::Uuid;
mod contract;
mod review;
mod status;
use contract::{Activity, Backend};
pub use contract::{AdoptionCommand, AdoptionError, AdoptionStatus};
#[cfg(test)]
use contract::{Completed, Imported, Phase, Reconciliation};
use review::Review;
#[cfg(all(test, target_os = "macos"))]
mod fixture;
#[cfg(all(test, target_os = "macos"))]
mod lifecycle_tests;
#[cfg(all(test, target_os = "macos"))]
mod tests;
#[cfg(all(test, target_os = "macos"))]
mod uat_bridge;
#[cfg(all(test, target_os = "macos"))]
mod wine_tests;

/// Created inside the folder the user picks. Confirmation requires a destination
/// that does not exist yet, and a folder dialog can only return existing folders.
const COPY_FOLDER: &str = "Stargate Worlds";

#[derive(Default)]
pub(crate) struct Adoption {
    preview: Option<adoption::PreviewWorker>,
    session: Option<Session>,
    confirm: Option<ConfirmJob>,
    last_error: Option<AdoptionError>,
    // Served, with live progress, while publication holds the store.
    facts: Option<status::Facts>,
}
struct Session {
    preview: adoption::Preview,
    review: Review,
}
struct ConfirmJob {
    worker: adoption::ConfirmationWorker,
    preview_handle: Uuid,
}
/// Signed fixture origin for host tests; production always uses the fixed catalog
/// and the bundled helper.
#[cfg(test)]
pub(super) struct TestDispatch {
    pub manifest_url: String,
    #[cfg(target_os = "macos")]
    pub helper: Option<cimmeria_launcher_engine::mac_wine::HelperResource>,
    /// Applied to the next confirmed copy only.
    pub copy_fault: Mutex<Option<adoption::test_support::CopyFault>>,
}
impl NativeHost {
    fn adoption_backend(&self) -> Backend {
        #[cfg(test)]
        if self.adoption_fixture.is_some() {
            return Backend::Available;
        }
        #[cfg(target_os = "macos")]
        {
            if self
                .helper
                .as_ref()
                .is_some_and(|helper| helper.verify().is_ok())
            {
                Backend::Available
            } else {
                Backend::HelperUnavailable
            }
        }
        #[cfg(not(target_os = "macos"))]
        Backend::UnsupportedPlatform
    }
    /// Checked before a folder dialog or the network is touched, and again when
    /// the preview is admitted.
    pub fn adoption_choice_allowed(&self) -> Result<(), AdoptionError> {
        let status = self.adoption_status()?;
        if status.backend != Backend::Available {
            return Err(AdoptionError::PlatformUnavailable);
        }
        if status.native.requires_reopen {
            return Err(AdoptionError::PersistenceUncertain);
        }
        if let Some(blocker) = status
            .imported
            .ok_or(AdoptionError::ImportRequired)?
            .blocker
        {
            return Err(blocker);
        }
        if status.reconciliation.is_some() {
            return Err(AdoptionError::RecoveryRequired);
        }
        if status.owned {
            return Err(AdoptionError::IdentityConflict);
        }
        if status.activity != Activity::Idle
            || status
                .native
                .operation
                .operation
                .is_some_and(|op| !op.state.terminal())
        {
            return Err(AdoptionError::Busy);
        }
        Ok(())
    }
    /// `location` is the folder the user picked natively; the copy goes into a
    /// new folder inside it.
    pub fn begin_adoption_preview(
        &self,
        location: PathBuf,
        release: VerifiedRelease,
    ) -> Result<AdoptionStatus, AdoptionError> {
        self.adoption_choice_allowed()?;
        let destination = location
            .canonicalize()
            .map_err(|_| AdoptionError::InvalidDirectory)?
            .join(COPY_FOLDER);
        if std::fs::symlink_metadata(&destination).is_ok() {
            return Err(AdoptionError::InvalidDirectory);
        }
        {
            let mut adoption = self.adoption.lock().map_err(|_| AdoptionError::Io)?;
            if adoption.preview.is_some()
                || adoption.session.is_some()
                || adoption.confirm.is_some()
            {
                return Err(AdoptionError::Busy);
            }
            let store = self.store()?;
            let request = {
                let state = store.lock().map_err(|_| AdoptionError::Io)?;
                let imported = state
                    .legacy_import()
                    .map_err(|_| AdoptionError::CorruptState)?
                    .ok_or(AdoptionError::ImportRequired)?;
                adoption::PreviewRequest {
                    import_digest: imported.confirmation,
                    destination,
                    operation_revision: state.operations().snapshot().revision,
                    preferences_revision: state.preferences().revision,
                    release,
                    artifacts: None,
                }
            };
            adoption.preview = Some(self.start_adoption_preview(store, request)?);
            adoption.last_error = None;
        }
        self.adoption_status()
    }
    fn start_adoption_preview(
        &self,
        store: Arc<Mutex<DesktopState>>,
        request: adoption::PreviewRequest,
    ) -> Result<adoption::PreviewWorker, AdoptionError> {
        #[cfg(test)]
        if let Some(fixture) = &self.adoption_fixture {
            let url = fixture.manifest_url.clone();
            #[cfg(target_os = "macos")]
            if let Some(helper) = &fixture.helper {
                return Ok(adoption::test_support::start_preview_wine(
                    store,
                    request,
                    helper.clone(),
                    url,
                )?);
            }
            return Ok(adoption::test_support::start_preview(store, request, url)?);
        }
        #[cfg(target_os = "macos")]
        {
            // The copy must be extracted by the build-pinned helper so that the
            // adopted installation records the Wine backend Play requires.
            let helper = self
                .helper
                .as_ref()
                .ok_or(AdoptionError::PlatformUnavailable)?;
            helper
                .verify()
                .map_err(|_| AdoptionError::PlatformUnavailable)?;
            Ok(adoption::start_preview_wine(
                store,
                request,
                helper.clone(),
            )?)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (store, request);
            Err(AdoptionError::PlatformUnavailable)
        }
    }
    pub fn adoption_command(
        &self,
        request: AdoptionCommand,
    ) -> Result<AdoptionStatus, AdoptionError> {
        request.validate()?;
        match request {
            AdoptionCommand::Inspect { .. } => (),
            AdoptionCommand::Dismiss { .. } => {
                let dismissed = {
                    let mut adoption = self.adoption.lock().map_err(|_| AdoptionError::Io)?;
                    self.settle_adoption(&mut adoption)?;
                    if let Some(worker) = &adoption.preview {
                        worker.request_cancel();
                    }
                    adoption.last_error = None;
                    adoption.session.take()
                };
                if let Some(session) = dismissed {
                    session.preview.discard();
                }
            }
            AdoptionCommand::Cancel { .. } => {
                let adoption = self.adoption.lock().map_err(|_| AdoptionError::Io)?;
                if let Some(worker) = &adoption.preview {
                    worker.request_cancel();
                }
                if let Some(job) = &adoption.confirm {
                    job.worker.request_cancel();
                }
            }
            AdoptionCommand::Confirm {
                work_id,
                preview_handle,
                operation_revision,
                preferences_revision,
                normalize_managed_files,
                accept_unavailable_game_telemetry,
                old_game_closed,
                confirmed,
                ..
            } => {
                if !confirmed {
                    return Err(AdoptionError::ConsentRequired);
                }
                self.confirm_adoption(
                    work_id,
                    preview_handle,
                    (operation_revision, preferences_revision),
                    Choices {
                        normalize_managed_files,
                        accept_unavailable_game_telemetry,
                        old_game_closed,
                    },
                )?;
            }
            AdoptionCommand::Recover {
                operation_id,
                operation_revision,
                confirmed,
                ..
            } => self.maintain_adoption(confirmed, |state| {
                adoption::recover(state, operation_id, operation_revision)
            })?,
            AdoptionCommand::Abandon {
                operation_id,
                operation_revision,
                confirmed,
                ..
            } => self.maintain_adoption(confirmed, |state| {
                adoption::abandon(state, operation_id, operation_revision)
            })?,
            AdoptionCommand::AbandonPreparation {
                preparation_id,
                operation_revision,
                confirmed,
                ..
            } => self.maintain_adoption(confirmed, |state| {
                adoption::abandon_preparation(state, preparation_id, operation_revision)
            })?,
        }
        self.adoption_status()
    }
    /// Every refusal here leaves the reviewed preview in place. It is consumed
    /// only when the retained copy worker has actually been started.
    fn confirm_adoption(
        &self,
        work_id: Uuid,
        preview_handle: Uuid,
        (operation_revision, preferences_revision): (u64, u64),
        choices: Choices,
    ) -> Result<(), AdoptionError> {
        if work_id.is_nil() {
            return Err(AdoptionError::IdentityConflict);
        }
        let mut adoption = self.adoption.lock().map_err(|_| AdoptionError::Io)?;
        self.settle_adoption(&mut adoption)?;
        if adoption.confirm.is_some() {
            return Err(AdoptionError::Busy);
        }
        let session = adoption
            .session
            .as_ref()
            .filter(|session| session.review.preview_handle == preview_handle)
            .ok_or(AdoptionError::ReviewUnavailable)?;
        if session.review.operation_revision != operation_revision
            || session.review.preferences_revision != preferences_revision
        {
            return Err(AdoptionError::StaleRevision);
        }
        {
            let store = self.store()?;
            let state = store.lock().map_err(|_| AdoptionError::Io)?;
            if state.requires_reopen() {
                return Err(AdoptionError::PersistenceUncertain);
            }
            if state.operations().snapshot().revision != operation_revision
                || state.preferences().revision != preferences_revision
            {
                return Err(AdoptionError::StaleRevision);
            }
        }
        session.preview.consent(&choices)?;
        let Some(session) = adoption.session.take() else {
            return Err(AdoptionError::ReviewUnavailable);
        };
        #[cfg(test)]
        let fault = self
            .adoption_fixture
            .as_ref()
            .and_then(|fixture| fixture.copy_fault.lock().ok()?.take());
        #[cfg(test)]
        let worker = match fault {
            Some(fault) => adoption::test_support::start_confirmation(
                session.preview,
                work_id,
                preview_handle,
                choices,
                fault,
            )?,
            None => {
                adoption::start_confirmation(session.preview, work_id, preview_handle, choices)?
            }
        };
        #[cfg(not(test))]
        let worker =
            adoption::start_confirmation(session.preview, work_id, preview_handle, choices)?;
        adoption.confirm = Some(ConfirmJob {
            worker,
            preview_handle,
        });
        adoption.last_error = None;
        Ok(())
    }
    /// Explicit recovery and cleanup run on this command's blocking worker. The
    /// engine's journal checks decide; nothing here redispatches work.
    fn maintain_adoption(
        &self,
        confirmed: bool,
        run: impl FnOnce(&mut DesktopState) -> Result<(), adoption::Error>,
    ) -> Result<(), AdoptionError> {
        if !confirmed {
            return Err(AdoptionError::ConsentRequired);
        }
        {
            let mut adoption = self.adoption.lock().map_err(|_| AdoptionError::Io)?;
            self.settle_adoption(&mut adoption)?;
            if adoption.preview.is_some()
                || adoption.session.is_some()
                || adoption.confirm.is_some()
            {
                return Err(AdoptionError::Busy);
            }
            adoption.last_error = None;
        }
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| AdoptionError::Io)?;
        Ok(run(&mut state)?)
    }
}
