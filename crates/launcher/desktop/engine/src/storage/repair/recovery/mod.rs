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
fn reconcile(state: &mut DesktopState, id: Uuid, revision: u64) -> Result<(), IntentError> {
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
        || operation.kind != OperationKind::Repair
        || operation.state != OperationState::ReconciliationRequired
    {
        return Err(ContractError::InvalidTransition.into());
    }
    let plan = state.repair_plan()?.ok_or(StorageError::Corrupt)?;
    if !plan.installation.backend.is_native() {
        return Err(ContractError::InvalidTransition.into());
    }
    let root = &plan.installation.destination;
    if root.canonicalize().map_err(|_| StorageError::Io)? != *root {
        return Err(StorageError::UnsafeFile.into());
    }
    let _root_owner = lock_owner(&plan.installation)?;
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
    let prepared: Plan = read(
        &state
            .directory
            .root
            .join(format!("repair-prepared-{id}.json")),
    )?
    .ok_or(StorageError::Corrupt)?;
    let record: Record = read(
        &state
            .directory
            .root
            .join(format!("repair-commit-{id}.json")),
    )?
    .ok_or(StorageError::Corrupt)?;
    if prepared != plan || record.schema_version != 2 || record.plan != plan {
        return Err(StorageError::Corrupt.into());
    }
    let release = state
        .release_for_intent(&plan.installation)
        .map_err(|_| StorageError::Corrupt)?;
    failed_cleanup::validate_tree(&plan.work_directory())?;
    let game = root.join("game");
    let shape = (
        read_role(&game, &plan)?,
        read_role(&plan.stage(), &plan)?,
        read_role(&plan.backup(), &plan)?,
    );
    let pending = match (plan.original_present, shape, record.phase) {
        (true, (Some(Role::Original), Some(Role::Replacement), None), Phase::Planned) => true,
        (false, (None, Some(Role::Replacement), None), Phase::Planned | Phase::OriginalMoved) => {
            true
        }
        (
            true,
            (None, Some(Role::Replacement), Some(Role::Original)),
            Phase::Planned | Phase::OriginalMoved,
        ) => false,
        (
            _,
            (Some(Role::Replacement), None, backup),
            Phase::OriginalMoved | Phase::Promoted | Phase::Published,
        ) if backup == plan.original_present.then_some(Role::Original) => {
            return publish(state, &plan, &release, &mut hook);
        }
        _ => return Err(StorageError::Corrupt.into()),
    };
    if !install_worker::content_valid(&plan.stage(), &release) {
        return Err(StorageError::Corrupt.into());
    }
    if pending && plan.original_present {
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
    let root = &plan.installation.destination;
    if !install_worker::content_valid(&root.join("game"), release) {
        return Err(StorageError::Corrupt.into());
    }
    commit::sync(&plan.work_directory())?;
    commit::sync(root)?;
    atomic::write(root, "content-ready.json", &plan.installation)?;
    commit::record(state, plan, Phase::Published)?;
    hook(Point::Published)?;
    state
        .operations_mut()?
        .reconcile(plan.id, OperationState::Succeeded)?;
    Ok(())
}
#[cfg(test)]
pub(super) mod tests;
