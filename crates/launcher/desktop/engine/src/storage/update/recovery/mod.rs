//! Explicit recovery of a checkpointed native replacement; never rerun extraction.
use super::*;
use crate::OperationState;
use commit::{Phase, Point, Record};
use std::sync::Mutex;
use tokio::sync::oneshot;
use tree_identity::{read_role, Role};

/// Requires the observed recovery revision and operation ID. The retained
/// blocking worker completes independently of the caller's response channel.
pub fn recover_native(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    if !cfg!(windows) {
        return Err(ContractError::InvalidTransition.into());
    }
    dispatch(state, id, revision)
}
#[cfg(target_os = "macos")]
pub fn recover_wine(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    dispatch(state, id, revision)
}
pub(super) fn dispatch(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (send, result) = oneshot::channel();
    runtime.spawn_blocking(move || {
        let outcome = state
            .lock()
            .map_err(|_| IntentError::from(StorageError::Io))
            .and_then(|mut state| reconcile(&mut state, id, revision));
        let _ = send.send(outcome);
    });
    Ok(result)
}
pub(super) fn reconcile(
    state: &mut DesktopState,
    id: Uuid,
    revision: u64,
) -> Result<(), IntentError> {
    reconcile_with(state, id, revision, |_| Ok(()))
}
fn reconcile_with(
    state: &mut DesktopState,
    id: Uuid,
    revision: u64,
    mut hook: impl FnMut(Point) -> Result<(), StorageError>,
) -> Result<(), IntentError> {
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
        || operation.kind != OperationKind::Update
        || operation.state != OperationState::ReconciliationRequired
    {
        return Err(ContractError::InvalidTransition.into());
    }
    let plan = state.update_plan()?.ok_or(StorageError::Corrupt)?;
    #[cfg(not(target_os = "macos"))]
    if !plan.owner.backend.is_native() {
        return Err(ContractError::InvalidTransition.into());
    }
    let root = &plan.owner.destination;
    if root.canonicalize().map_err(|_| StorageError::Io)? != *root {
        return Err(StorageError::UnsafeFile.into());
    }
    let _root_owner = lock_owner(&plan.owner)?;
    #[cfg(target_os = "macos")]
    let _stopped_prefix = if plan.owner.backend.is_native() {
        None
    } else {
        Some(crate::mac_wine::repair_recovery::stop_update(
            state, id, true,
        )?)
    };
    if !directory_or_absent(&plan.work_directory())? {
        return Err(StorageError::Corrupt.into());
    }
    let path = plan.work_directory().join("owner.json");
    ordinary(&std::fs::symlink_metadata(&path).map_err(|_| StorageError::Corrupt)?)?;
    ensure_regular_or_absent(&path)?;
    let work_owner = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| StorageError::Io)?;
    work_owner.try_lock().map_err(|_| StorageError::InUse)?;
    if read_open::<Plan>(&work_owner)? != plan {
        return Err(StorageError::Corrupt.into());
    }
    let prepared: staged_content::Ready = read(
        &state
            .directory
            .root
            .join(format!("update-prepared-{id}.json")),
    )?
    .ok_or(StorageError::Corrupt)?;
    let record: Record = read(
        &state
            .directory
            .root
            .join(format!("update-commit-{id}.json")),
    )?
    .ok_or(StorageError::Corrupt)?;
    if prepared.plan != plan || record.schema_version != 2 || record.plan != plan {
        return Err(StorageError::Corrupt.into());
    }
    let release = state
        .verify_release_identity(plan.target)
        .map_err(|_| StorageError::Corrupt)?;
    failed_cleanup::validate_tree(&plan.work_directory())?;
    let game = root.join("game");
    let shape = (
        read_role(&game, &plan)?,
        read_role(&plan.stage(), &plan)?,
        read_role(&plan.backup(), &plan)?,
    );
    let pending = match (shape, record.phase) {
        ((Some(Role::Original), Some(Role::Replacement), None), Phase::Planned) => true,
        (
            (None, Some(Role::Replacement), Some(Role::Original)),
            Phase::Planned | Phase::OriginalMoved,
        ) => false,
        (
            (Some(Role::Replacement), None, Some(Role::Original)),
            Phase::OriginalMoved | Phase::Promoted | Phase::Published,
        ) => {
            return publish(state, &plan, &release, &mut hook);
        }
        _ => return Err(StorageError::Corrupt.into()),
    };
    staged_content::verify(state, &plan, &plan.stage())?;
    if !install_worker::content_valid(&plan.stage(), &release) {
        return Err(StorageError::Corrupt.into());
    }
    if pending {
        hook(Point::BeforeOriginalRename)?;
        std::fs::rename(&game, plan.backup()).map_err(|_| StorageError::Io)?;
        hook(Point::AfterOriginalRename)?;
    }
    // Also sync a first rename that happened immediately before the interruption.
    commit::sync(root)?;
    commit::record(state, &plan, Phase::OriginalMoved)?;
    hook(Point::OriginalMoved)?;
    hook(Point::BeforePromotion)?;
    std::fs::rename(plan.stage(), &game).map_err(|_| StorageError::Io)?;
    hook(Point::AfterPromotion)?;
    commit::sync(&plan.work_directory())?;
    commit::sync(root)?;
    commit::record(state, &plan, Phase::Promoted)?;
    hook(Point::Promoted)?;
    publish(state, &plan, &release, &mut hook)
}
fn publish(
    state: &mut DesktopState,
    plan: &Plan,
    release: &crate::catalog::VerifiedRelease,
    hook: &mut impl FnMut(Point) -> Result<(), StorageError>,
) -> Result<(), IntentError> {
    let root = &plan.owner.destination;
    if !install_worker::content_valid(&root.join("game"), release) {
        return Err(StorageError::Corrupt.into());
    }
    staged_content::verify(state, plan, &root.join("game"))?;
    commit::sync(&plan.work_directory())?;
    commit::sync(root)?;
    let receipt = installed_content::read_ready(root, &plan.owner)?;
    if receipt != Some(plan.previous) && receipt != Some(plan.target) {
        return Err(StorageError::Corrupt.into());
    }
    hook(Point::BeforeReceipt)?;
    let saved = installed_content::write_ready(root, &plan.owner, plan.target);
    state.preferences_uncertain |= saved == Err(StorageError::PersistenceUncertain);
    saved?;
    hook(Point::AfterReceipt)?;
    hook(Point::BeforeIndex)?;
    state.publish_update_index(&plan.owner, plan.previous, plan.target)?;
    hook(Point::AfterIndex)?;
    commit::record(state, plan, Phase::Published)?;
    hook(Point::Published)?;
    state
        .operations_mut()?
        .reconcile(plan.id, OperationState::Succeeded)?;
    Ok(())
}
