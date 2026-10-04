//! Status projection: retained workers are collected, and store-derived facts are
//! read without ever waiting on a running copy.
use super::contract::{Completed, Imported, Phase, Progress, Reconciliation};
use super::*;
use cimmeria_launcher_engine::{install::Progress as Observed, OperationKind, OperationState};
use std::sync::TryLockError;

#[derive(Clone)]
pub(super) struct Facts {
    native: NativeSnapshot,
    imported: Option<Imported>,
    reconciliation: Option<Reconciliation>,
    preparations: Vec<Uuid>,
    owned: bool,
    completed: Option<Completed>,
}
impl NativeHost {
    /// Never blocks on a retained copy: while publication holds the store, the
    /// last facts are served with the worker's live progress.
    pub fn adoption_status(&self) -> Result<AdoptionStatus, AdoptionError> {
        let mut adoption = self.adoption.lock().map_err(|_| AdoptionError::Io)?;
        self.settle_adoption(&mut adoption)?;
        let store = self.store()?;
        let fresh = match store.try_lock() {
            Ok(mut state) => Some(adoption_facts(&mut state, &adoption)?),
            Err(TryLockError::WouldBlock)
                if adoption.confirm.is_some() && adoption.facts.is_some() =>
            {
                None
            }
            Err(TryLockError::WouldBlock) => {
                let mut state = store.lock().map_err(|_| AdoptionError::Io)?;
                Some(adoption_facts(&mut state, &adoption)?)
            }
            Err(TryLockError::Poisoned(_)) => return Err(AdoptionError::Io),
        };
        if let Some(fresh) = fresh {
            adoption.facts = Some(fresh);
        }
        let facts = adoption.facts.clone().ok_or(AdoptionError::Io)?;
        let (activity, progress) = if let Some(job) = &adoption.confirm {
            (
                Activity::Copying,
                job.worker.progress.borrow().as_ref().map(|observed| {
                    let (current, total) = counts(observed);
                    Progress {
                        phase: Phase::Copy,
                        current,
                        total,
                    }
                }),
            )
        } else if let Some(worker) = &adoption.preview {
            (
                Activity::Preparing,
                worker.progress.borrow().as_ref().map(|observed| {
                    let (current, total) = counts(observed);
                    Progress {
                        phase: match observed {
                            Observed::Downloading { .. } => Phase::Download,
                            Observed::Extracting { .. } => Phase::Extraction,
                        },
                        current,
                        total,
                    }
                }),
            )
        } else if adoption.session.is_some() {
            (Activity::Review, None)
        } else {
            (Activity::Idle, None)
        };
        Ok(AdoptionStatus {
            schema_version: 1,
            native: facts.native,
            backend: self.adoption_backend(),
            imported: facts.imported,
            activity,
            review: adoption
                .session
                .as_ref()
                .map(|session| session.review.clone()),
            progress,
            // Once every file is copied, publication can no longer be stopped.
            cancellable: adoption.preview.is_some()
                || (adoption.confirm.is_some()
                    && progress.is_none_or(|copied| copied.current < copied.total)),
            reconciliation: facts.reconciliation,
            preparations: facts.preparations,
            owned: facts.owned,
            completed: facts.completed,
            last_error: adoption.last_error,
        })
    }
    /// Collect finished workers. The preview stays native-held as the session.
    pub(super) fn settle_adoption(&self, adoption: &mut Adoption) -> Result<(), AdoptionError> {
        if let Some(outcome) = adoption.preview.as_mut().and_then(|w| w.try_result()) {
            adoption.preview = None;
            match outcome {
                Ok(preview) => {
                    // Admitting the preparation advanced the journal. Confirmation
                    // is bound to the revisions the user actually reviews.
                    let store = self.store()?;
                    let state = store.lock().map_err(|_| AdoptionError::Io)?;
                    let review = Review::of(
                        &preview,
                        state.operations().snapshot().revision,
                        state.preferences().revision,
                    );
                    drop(state);
                    adoption.session = Some(Session { preview, review });
                    adoption.last_error = None;
                }
                Err(error) => adoption.last_error = Some(error.into()),
            }
        }
        let finished = adoption
            .confirm
            .as_ref()
            .and_then(|job| job.worker.result.borrow().clone());
        if let Some(outcome) = finished {
            adoption.confirm = None;
            adoption.last_error = outcome.err().map(Into::into);
        }
        Ok(())
    }
}
fn counts(observed: &Observed) -> (u64, u64) {
    const MAX: u64 = 9_007_199_254_740_991;
    match observed {
        Observed::Downloading {
            downloaded, total, ..
        } => ((*downloaded).min(MAX), (*total).min(MAX)),
        Observed::Extracting { current, total, .. } => {
            ((*current as u64).min(MAX), (*total as u64).min(MAX))
        }
    }
}
fn adoption_facts(state: &mut DesktopState, adoption: &Adoption) -> Result<Facts, AdoptionError> {
    let native = state.inspect();
    if native.requires_reopen {
        return Ok(Facts {
            native,
            imported: None,
            reconciliation: None,
            preparations: Vec::new(),
            owned: false,
            completed: None,
        });
    }
    // Adoption applies only before any desktop-owned installation exists. An
    // unreadable owner is treated as present rather than adopted over.
    let owned = !matches!(state.installed_content(), Ok(None));
    let imported = state
        .legacy_import()
        .map_err(|_| AdoptionError::CorruptState)?
        .map(|imported| Imported {
            blocker: adoption::import_blocker(&imported).map(Into::into),
            launcher_directory: imported.source.launcher_directory,
            game_directory: imported.source.game_directory,
        });
    let operation = native
        .operation
        .operation
        .as_ref()
        .filter(|op| op.kind == OperationKind::Adopt);
    let mut reconciliation = None;
    let mut completed = None;
    match operation.map(|op| (op.id, op.state)) {
        Some((id, OperationState::ReconciliationRequired)) => {
            reconciliation = if adoption::inspect_preparation(state, id).is_ok() {
                Some(Reconciliation::Preparation { preparation_id: id })
            } else {
                adoption::interrupted(state, id)
                    .ok()
                    .map(|copy| Reconciliation::Copy {
                        operation_id: id,
                        directory: copy.directory,
                        can_recover: copy.recoverable,
                        can_abandon: copy.abandonable,
                    })
            };
        }
        Some((id, OperationState::Succeeded)) => {
            completed = adoption::inspect(state, id)
                .ok()
                .filter(|record| record.phase == adoption::Phase::Published)
                .map(|record| Completed {
                    directory: record.plan.installation.destination,
                });
        }
        _ => (),
    }
    // A reference still owned by a journal operation, the held review or the
    // running copy is not a leftover.
    let held = [
        operation.filter(|op| !op.state.terminal()).map(|op| op.id),
        adoption
            .session
            .as_ref()
            .map(|session| session.review.preview_handle),
        adoption.confirm.as_ref().map(|job| job.preview_handle),
    ];
    let preparations = adoption::list_preparations(state)?
        .into_iter()
        .map(|record| record.id)
        .filter(|id| !held.contains(&Some(*id)))
        .collect();
    Ok(Facts {
        native,
        imported,
        reconciliation,
        preparations,
        owned,
        completed,
    })
}
