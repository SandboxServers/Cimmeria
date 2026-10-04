//! Explicitly abandon interrupted native preparation without deleting any files.
use super::*;
use crate::OperationState;
use std::sync::Mutex;
use tokio::sync::oneshot;

/// Cancellation before a commit checkpoint leaves both trees/evidence in place.
/// This does not repair the game; a new confirmed Repair can reconstruct again.
pub fn abandon_native(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
    confirmed: bool,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    if !cfg!(windows) {
        return Err(ContractError::InvalidTransition.into());
    }
    dispatch(state, id, revision, confirmed)
}
#[cfg(target_os = "macos")]
pub fn abandon_wine(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
    confirmed: bool,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    dispatch(state, id, revision, confirmed)
}
fn dispatch(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
    confirmed: bool,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (send, result) = oneshot::channel();
    runtime.spawn_blocking(move || {
        let outcome = state
            .lock()
            .map_err(|_| IntentError::from(StorageError::Io))
            .and_then(|mut state| abandon(&mut state, id, revision, confirmed));
        let _ = send.send(outcome);
    });
    Ok(result)
}
fn abandon(
    state: &mut DesktopState,
    id: Uuid,
    revision: u64,
    confirmed: bool,
) -> Result<(), IntentError> {
    if !confirmed {
        return Err(ContractError::InvalidTransition.into());
    }
    if state.requires_reopen() {
        return Err(StorageError::PersistenceUncertain.into());
    }
    let snapshot = state.operations().snapshot();
    if snapshot.revision != revision {
        return Err(ContractError::StaleRevision.into());
    }
    let operation = snapshot
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if operation.id != id
        || operation.kind != OperationKind::Repair
        || operation.state != OperationState::ReconciliationRequired
    {
        return Err(ContractError::InvalidTransition.into());
    }
    let plan = state.repair_plan()?.ok_or(StorageError::Corrupt)?;
    #[cfg(not(target_os = "macos"))]
    if !plan.installation.backend.is_native() {
        return Err(ContractError::InvalidTransition.into());
    }
    let root = &plan.installation.destination;
    let _root_owner = lock_owner(&plan.installation)?;
    #[cfg(target_os = "macos")]
    let _stopped_prefix = if plan.installation.backend.is_native() {
        None
    } else {
        Some(crate::mac_wine::repair_recovery::stop(state, id, false)?)
    };
    // Absence must be unambiguous. Corrupt, linked, partial and legacy commit
    // records still forbid abandonment; they may describe a visible rename.
    absent(
        &state
            .directory
            .root
            .join(format!("repair-commit-{id}.json")),
    )?;
    absent(&plan.backup())?;
    if root.canonicalize().map_err(|_| StorageError::Io)? != *root
        || directory_or_absent(&root.join("game"))? != plan.original_present
    {
        return Err(StorageError::UnsafeFile.into());
    }
    let prepared: Option<Plan> = read(
        &state
            .directory
            .root
            .join(format!("repair-prepared-{id}.json")),
    )?;
    if prepared.as_ref().is_some_and(|saved| *saved != plan) {
        return Err(StorageError::Corrupt.into());
    }
    let _work_owner = if directory_or_absent(&plan.work_directory())? {
        let path = plan.work_directory().join("owner.json");
        ordinary(&std::fs::symlink_metadata(&path).map_err(|_| StorageError::Corrupt)?)?;
        ensure_regular_or_absent(&path)?;
        let owner = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| StorageError::Io)?;
        owner.try_lock().map_err(|_| StorageError::InUse)?;
        if read_open::<Plan>(&owner)? != plan {
            return Err(StorageError::Corrupt.into());
        }
        Some(owner)
    } else {
        if prepared.is_some() {
            return Err(StorageError::Corrupt.into());
        }
        None
    };
    // Never interpret the original/stage bytes as repaired, and never delete
    // incomplete markers, user modifications, download caches or evidence.
    state
        .operations_mut()?
        .reconcile(id, OperationState::Cancelled)?;
    Ok(())
}
fn absent(path: &Path) -> Result<(), StorageError> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StorageError::Io),
        Ok(_) => Err(StorageError::UnsafeFile),
    }
}
#[cfg(test)]
mod tests;
