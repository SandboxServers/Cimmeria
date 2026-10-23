//! Reading an adoption checkpoint, and explicit recovery or abandonment of an interrupted one.
use super::publication::{finish, name, plan_name, read_large};
use super::*;

pub fn inspect(state: &DesktopState, id: Uuid) -> Result<Record, Error> {
    if state.requires_reopen() {
        return Err(StorageError::PersistenceUncertain.into());
    }
    let plan: Plan = read_large(&state.directory.root.join(plan_name(id)))?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.id != id
        || op.kind != OperationKind::Adopt
        || op.intent_digest != plan.digest()?
        || plan.schema_version != 1
        || plan.provenance.work_id != id
    {
        return Err(StorageError::Corrupt.into());
    }
    state
        .release_for_intent(&plan.installation)
        .map_err(|_| StorageError::Corrupt)?;
    let record: Record = read_large(&state.directory.root.join(name(id)))?;
    if record.plan != plan {
        return Err(StorageError::Corrupt.into());
    }
    Ok(record)
}
/// What an interrupted copy still allows. Reading this never mutates or redispatches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interrupted {
    pub directory: PathBuf,
    /// A verified Staged checkpoint exists, so `recover` can finish publication.
    pub recoverable: bool,
    /// Nothing was promoted, so `abandon` can end the operation.
    pub abandonable: bool,
}
pub fn interrupted(state: &DesktopState, id: Uuid) -> Result<Interrupted, Error> {
    let plan: Plan = read_large(&state.directory.root.join(plan_name(id)))?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.id != id || op.kind != OperationKind::Adopt || op.intent_digest != plan.digest()? {
        return Err(StorageError::Corrupt.into());
    }
    Ok(Interrupted {
        recoverable: inspect(state, id).is_ok(),
        abandonable: if plan.report.in_place() {
            super::in_place::abandonable(state, &plan)
        } else {
            matches!(
                std::fs::symlink_metadata(plan.installation.destination.join("game")),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            )
        },
        directory: plan.installation.destination,
    })
}
/// Explicit reopen recovery finishes only an already verified Staged checkpoint.
/// Missing checkpoints are quarantined; they never trigger extraction or copying.
pub fn recover(state: &mut DesktopState, id: Uuid, revision: u64) -> Result<(), Error> {
    state.ensure_updater_idle()?;
    if state.operations.snapshot().revision != revision {
        return Err(ContractError::StaleRevision.into());
    }
    let record = inspect(state, id)?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.state == OperationState::Succeeded {
        return Ok(());
    }
    if op.state != OperationState::ReconciliationRequired {
        return Err(ContractError::InvalidTransition.into());
    }
    let owner = inventory::open(
        &record
            .plan
            .installation
            .destination
            .join(".cimmeria-install.json"),
    )?;
    owner.try_lock().map_err(|_| StorageError::InUse)?;
    let saved: InstallIntent = super::super::read_open(&owner)?;
    if saved != record.plan.installation {
        return Err(StorageError::Corrupt.into());
    }
    let result = if record.plan.report.in_place() {
        super::in_place::finish(state, &record.plan, &mut |_| Ok(()))
    } else {
        finish(state, &record.plan, &mut |_| Ok(()))
    };
    if result.is_err() {
        let _ = state.operations_mut().and_then(|op| {
            op.mark_uncertain(id)
                .map(|_| ())
                .map_err(|_| StorageError::PersistenceUncertain)
        });
    }
    drop(owner);
    if result.is_ok() {
        state.log_applied_content("applied");
    }
    result
}
/// Explicit abandonment retains quarantined bytes. It never removes an entire
/// destination based on a missing receipt or a renderer's path claim.
pub fn abandon(state: &mut DesktopState, id: Uuid, revision: u64) -> Result<(), Error> {
    state.ensure_updater_idle()?;
    if state.operations.snapshot().revision != revision {
        return Err(ContractError::StaleRevision.into());
    }
    let plan: Plan = read_large(&state.directory.root.join(plan_name(id)))?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.id != id || op.kind != OperationKind::Adopt || op.intent_digest != plan.digest()? {
        return Err(StorageError::Corrupt.into());
    }
    if op.state != OperationState::ReconciliationRequired {
        return Err(ContractError::InvalidTransition.into());
    }
    // In place, abandonment moves the release files back where they were.
    if plan.report.in_place() {
        return super::in_place::abandon(state, &plan);
    }
    if std::fs::symlink_metadata(plan.installation.destination.join("game")).is_ok() {
        return Err(ContractError::InvalidTransition.into());
    }
    state
        .operations_mut()?
        .reconcile(id, OperationState::Cancelled)?;
    Ok(())
}
