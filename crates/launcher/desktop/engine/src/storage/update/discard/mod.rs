//! Explicit resumable deletion of a cancelled/failed preparation, never game content.
use super::*;
use crate::OperationState;
use std::sync::Mutex;
use tokio::sync::oneshot;
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Deleting,
    Empty,
    Removed,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    plan: Plan,
    phase: Phase,
}

/// Explicit cleanup after terminal precommit cancellation/failure. Interrupted
/// nonterminal work must first be abandoned; a checkpointed commit cannot discard.
pub fn discard_native(
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
pub fn discard_wine(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
    confirmed: bool,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    dispatch(state, id, revision, confirmed)
}
pub(super) fn dispatch(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
    confirmed: bool,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (send, result) = oneshot::channel();
    runtime.spawn_blocking(move || {
        let result = state
            .lock()
            .map_err(|_| IntentError::from(StorageError::Io))
            .and_then(|mut state| discard(&mut state, id, revision, confirmed));
        let _ = send.send(result);
    });
    Ok(result)
}
fn discard(
    state: &mut DesktopState,
    id: Uuid,
    revision: u64,
    confirmed: bool,
) -> Result<(), IntentError> {
    state.ensure_updater_idle()?;
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
    let op = snapshot
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.id != id
        || op.kind != OperationKind::Update
        || !matches!(op.state, OperationState::Cancelled | OperationState::Failed)
    {
        return Err(ContractError::InvalidTransition.into());
    }
    let plan = state.update_plan()?.ok_or(StorageError::Corrupt)?;
    let _owner = lock_owner(&plan.owner)?;
    #[cfg(target_os = "macos")]
    let _prefix = if plan.owner.backend.is_native() {
        None
    } else {
        Some(crate::mac_wine::repair_recovery::stop_update(
            state, id, false,
        )?)
    };
    #[cfg(not(target_os = "macos"))]
    if !plan.owner.backend.is_native() {
        return Err(ContractError::InvalidTransition.into());
    }
    absent(
        &state
            .directory
            .root
            .join(format!("update-commit-{id}.json")),
    )?;
    absent(&plan.backup())?;
    if plan
        .owner
        .destination
        .canonicalize()
        .map_err(|_| StorageError::Io)?
        != plan.owner.destination
        || !directory_or_absent(&plan.owner.destination.join("game"))?
        || installed_content::read_ready(&plan.owner.destination, &plan.owner)?
            != Some(plan.previous)
    {
        return Err(StorageError::Corrupt.into());
    }
    let work = plan.work_directory();
    let saved: Option<Record> = read(
        &state
            .directory
            .root
            .join(format!("update-discard-{id}.json")),
    )?;
    let phase = match saved {
        Some(record) if record.schema_version == 1 && record.plan == plan => Some(record.phase),
        Some(_) => return Err(StorageError::Corrupt.into()),
        None => None,
    };
    if phase == Some(Phase::Removed) {
        absent(&work)?;
        return Ok(());
    }
    if !directory_or_absent(&work)? {
        if phase.is_some() && phase != Some(Phase::Empty) {
            return Err(StorageError::Corrupt.into());
        }
        return record(state, &plan, Phase::Removed).map_err(Into::into);
    }
    failed_cleanup::validate_tree(&work)?;
    let marker = work.join("owner.json");
    if phase != Some(Phase::Empty) {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&marker)
            .map_err(|_| StorageError::Corrupt)?;
        let owner = OwnerLock::acquire(file).map_err(|_| StorageError::InUse)?;
        if read_open::<Plan>(&owner)? != plan {
            return Err(StorageError::Corrupt.into());
        }
        record(state, &plan, Phase::Deleting)?;
        for entry in std::fs::read_dir(&work).map_err(|_| StorageError::Io)? {
            let entry = entry.map_err(|_| StorageError::Io)?;
            if entry.file_name() == "owner.json" {
                continue;
            }
            if entry.file_type().map_err(|_| StorageError::Io)?.is_dir() {
                std::fs::remove_dir_all(entry.path())
            } else {
                std::fs::remove_file(entry.path())
            }
            .map_err(|_| StorageError::Io)?;
        }
        commit::sync(&work)?;
        record(state, &plan, Phase::Empty)?;
        drop(owner);
    }
    let entries = std::fs::read_dir(&work)
        .map_err(|_| StorageError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::Io)?;
    if entries.len() == 1 && entries[0].file_name() == "owner.json" {
        if read::<Plan>(&marker)?.as_ref() != Some(&plan) {
            return Err(StorageError::Corrupt.into());
        }
        std::fs::remove_file(marker).map_err(|_| StorageError::Io)?;
    } else if !entries.is_empty() {
        return Err(StorageError::UnsafeFile.into());
    }
    commit::sync(&work)?;
    std::fs::remove_dir(&work).map_err(|_| StorageError::Io)?;
    commit::sync(&plan.owner.destination)?;
    record(state, &plan, Phase::Removed)?;
    Ok(())
}
fn absent(path: &Path) -> Result<(), StorageError> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StorageError::Io),
        Ok(_) => Err(StorageError::UnsafeFile),
    }
}
fn record(state: &mut DesktopState, plan: &Plan, phase: Phase) -> Result<(), StorageError> {
    let result = atomic::write(
        &state.directory.root,
        &format!("update-discard-{}.json", plan.id),
        &Record {
            schema_version: 1,
            plan: plan.clone(),
            phase,
        },
    );
    state.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
    result
}
