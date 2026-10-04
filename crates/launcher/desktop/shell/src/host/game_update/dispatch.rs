//! A retained preparation-to-commit handoff outlives the IPC observer.
use super::*;
use cimmeria_launcher_engine::{update, IntentError};

pub(crate) struct Worker {
    pub(super) id: Uuid,
    pub(super) preparation: update::preparation::Preparation,
}
impl NativeHost {
    pub fn apply_game_update(
        &self,
        request: GameUpdateCommand,
    ) -> Result<GameUpdateStatus, JobError> {
        request.validate()?;
        if let GameUpdateCommand::Cancel { operation_id, .. } = request {
            {
                let worker = self.game_update_worker.lock().map_err(|_| JobError::Io)?;
                let worker = worker
                    .as_ref()
                    .filter(|worker| worker.id == operation_id)
                    .ok_or(JobError::UnknownOperation)?;
                worker.preparation.request_cancel()?;
            }
            return self.game_update_status();
        }
        let GameUpdateCommand::Apply {
            offer_id,
            operation_id,
            operation_revision,
            confirmed,
            ..
        } = request
        else {
            return Err(JobError::UnsupportedSchema);
        };
        if !confirmed {
            return Err(JobError::RecoveryRequired);
        }
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| JobError::Io)?;
        let offers = self.game_update_offer.lock().map_err(|_| JobError::Io)?;
        let check = offers
            .check
            .as_ref()
            .filter(|check| check.id == offer_id)
            .ok_or(JobError::IdentityConflict)?;
        let release = offers.release.as_ref().ok_or(JobError::IdentityConflict)?;
        if check.revision != operation_revision {
            return Err(JobError::StaleRevision);
        }
        let native = check.native_backend;
        if !self.game_update_backend_supported(native) {
            return Err(JobError::PlatformUnavailable);
        }
        let mut worker = self.game_update_worker.lock().map_err(|_| JobError::Io)?;
        let state = self.store()?;
        let admission = state
            .lock()
            .map_err(|_| JobError::Io)?
            .admit_update(update::Request {
                id: operation_id,
                operation_revision,
                installation_id: check.installation,
                expected_current: check.current,
                target: release,
                confirmed,
            })?;
        if admission.dispatch {
            self.retain_game_update(state, operation_id, native, runtime, &mut worker)?;
        }
        drop(worker);
        drop(offers);
        self.game_update_status()
    }
    pub(super) fn retain_game_update(
        &self,
        state: Arc<Mutex<DesktopState>>,
        operation_id: Uuid,
        native: bool,
        runtime: tokio::runtime::Handle,
        worker: &mut Option<Worker>,
    ) -> Result<(), JobError> {
        let preparation = self.prepare_game_update(state.clone(), operation_id, native);
        let mut preparation = match preparation {
            Ok(preparation) => preparation,
            Err(error) => {
                mark_uncertain(&state, operation_id);
                return Err(error.into());
            }
        };
        #[cfg(test)]
        let fixture_commit = self
            .game_update_fixture
            .as_ref()
            .map(|fixture| fixture.interrupt);
        let (_, empty) = tokio::sync::oneshot::channel();
        let result = std::mem::replace(&mut preparation.result, empty);
        *worker = Some(Worker {
            id: operation_id,
            preparation,
        });
        runtime.spawn(async move {
            match result.await {
                Ok(Ok(prepared)) => {
                    #[cfg(test)]
                    let committed = if let Some(interrupt) = fixture_commit {
                        update::test_support::commit(state.clone(), prepared, interrupt)
                    } else {
                        commit(state.clone(), prepared, native)
                    };
                    #[cfg(not(test))]
                    let committed = commit(state.clone(), prepared, native);
                    match committed {
                        Ok(result) => {
                            if result.await.is_err() {
                                mark_uncertain(&state, operation_id);
                            }
                        }
                        Err(_) => mark_uncertain(&state, operation_id),
                    }
                }
                Ok(Err(_)) => (),
                Err(_) => mark_uncertain(&state, operation_id),
            }
        });
        Ok(())
    }
    fn prepare_game_update(
        &self,
        state: Arc<Mutex<DesktopState>>,
        operation_id: Uuid,
        native: bool,
    ) -> Result<update::preparation::Preparation, IntentError> {
        #[cfg(test)]
        if let Some(fixture) = &self.game_update_fixture {
            return update::test_support::prepare(state, operation_id, fixture.url.clone());
        }
        if native {
            update::preparation::prepare_native(state.clone(), operation_id)
        } else {
            #[cfg(target_os = "macos")]
            {
                update::preparation::prepare_wine(
                    state.clone(),
                    operation_id,
                    self.helper.clone().ok_or(StorageError::Corrupt)?,
                )
            }
            #[cfg(not(target_os = "macos"))]
            {
                Err(StorageError::Corrupt.into())
            }
        }
    }
    pub(super) fn game_update_backend_supported(&self, native: bool) -> bool {
        #[cfg(test)]
        if self.game_update_fixture.is_some() {
            return true;
        }
        if native {
            return cfg!(windows);
        }
        #[cfg(target_os = "macos")]
        {
            self.helper
                .as_ref()
                .is_some_and(|helper| helper.verify().is_ok())
        }
        #[cfg(not(target_os = "macos"))]
        false
    }
}
fn mark_uncertain(state: &Mutex<DesktopState>, id: Uuid) {
    if let Ok(mut state) = state.lock() {
        if let Ok(operations) = state.operations_mut() {
            let _ = operations.mark_uncertain(id);
        }
    }
}
fn commit(
    state: Arc<Mutex<DesktopState>>,
    prepared: update::preparation::Prepared,
    native: bool,
) -> Result<tokio::sync::oneshot::Receiver<Result<(), update::preparation::Failure>>, IntentError> {
    if native {
        return update::commit::commit_native(state, prepared);
    }
    #[cfg(target_os = "macos")]
    {
        update::commit::commit_wine(state, prepared)
    }
    #[cfg(not(target_os = "macos"))]
    {
        drop(prepared);
        Err(StorageError::Corrupt.into())
    }
}
