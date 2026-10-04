//! Recoverable removal of the old backup after durable repair success.
use super::*;
use crate::OperationState;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tree_identity::{read_role, Role};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Point {
    Planned,
    EntryRemoved,
    EmptyRecorded,
    MarkerRemoved,
    DirectoryRemoved,
    RemovedRecorded,
}

/// Current backup evidence, independent of the successful Repair journal result.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupStatus {
    Unavailable,
    NotRetained,
    Retained,
    CleanupPending,
    Removed,
}
/// Observe only the current repair's descriptor, checkpoint, role and cleanup record.
/// A partial deletion remains resumable even after its directory was removed.
pub fn status(state: &mut DesktopState) -> Result<BackupStatus, IntentError> {
    let snapshot = state.operations().snapshot();
    let Some(op) = snapshot
        .operation
        .as_ref()
        .filter(|op| op.kind == OperationKind::Repair && op.state == OperationState::Succeeded)
    else {
        return Ok(BackupStatus::Unavailable);
    };
    let id = op.id;
    let plan = state.repair_plan()?.ok_or(StorageError::Corrupt)?;
    let _owner = lock_owner(&plan.installation)?;
    let committed: commit::Record = read(
        &state
            .directory
            .root
            .join(format!("repair-commit-{id}.json")),
    )?
    .ok_or(StorageError::Corrupt)?;
    if committed.schema_version != 2
        || committed.plan != plan
        || committed.phase != commit::Phase::Published
    {
        return Err(StorageError::Corrupt.into());
    }
    let saved: Option<Record> = read(&state.directory.root.join(name(id)))?;
    let phase = match saved {
        Some(saved) if saved.schema_version == 1 && saved.plan == plan => Some(saved.phase),
        Some(_) => return Err(StorageError::Corrupt.into()),
        None => None,
    };
    let exists = directory_or_absent(&plan.backup())?;
    match phase {
        Some(Phase::Removed) if !exists => Ok(BackupStatus::Removed),
        Some(Phase::Empty) => Ok(BackupStatus::CleanupPending),
        Some(Phase::Removed) => Err(StorageError::UnsafeFile.into()),
        _ if !plan.original_present && !exists => Ok(BackupStatus::NotRetained),
        _ if exists && read_role(&plan.backup(), &plan)? == Some(Role::Original) => {
            Ok(if phase.is_some() {
                BackupStatus::CleanupPending
            } else {
                BackupStatus::Retained
            })
        }
        _ => Err(StorageError::Corrupt.into()),
    }
}

/// Retained native cleanup. The operation must still be the observed successful
/// Repair; cleanup does not change its result or infer success from filesystem state.
pub fn cleanup_native(
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
pub fn cleanup_wine(
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
        let result = state
            .lock()
            .map_err(|_| IntentError::from(StorageError::Io))
            .and_then(|mut state| cleanup(&mut state, id, revision, |_| Ok(())));
        let _ = send.send(result);
    });
    Ok(result)
}
fn cleanup(
    state: &mut DesktopState,
    id: Uuid,
    revision: u64,
    mut hook: impl FnMut(Point) -> Result<(), StorageError>,
) -> Result<(), IntentError> {
    state.ensure_updater_idle()?;
    if state.requires_reopen() {
        return Err(StorageError::PersistenceUncertain.into());
    }
    let snapshot = state.operations().snapshot();
    if revision != snapshot.revision {
        return Err(ContractError::StaleRevision.into());
    }
    let operation = snapshot
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if operation.id != id
        || operation.kind != OperationKind::Repair
        || operation.state != OperationState::Succeeded
    {
        return Err(ContractError::InvalidTransition.into());
    }
    let plan = state.repair_plan()?.ok_or(StorageError::Corrupt)?;
    #[cfg(not(target_os = "macos"))]
    if !plan.installation.backend.is_native() {
        return Err(ContractError::InvalidTransition.into());
    }
    let root = &plan.installation.destination;
    let _owner = lock_owner(&plan.installation)?;
    #[cfg(target_os = "macos")]
    let _stopped_prefix = if plan.installation.backend.is_native() {
        None
    } else {
        Some(crate::mac_wine::repair_recovery::stop(state, id, true)?)
    };
    if root.canonicalize().map_err(|_| StorageError::Io)? != *root
        || directory_or_absent(&plan.stage())?
    {
        return Err(StorageError::UnsafeFile.into());
    }
    let committed: commit::Record = read(
        &state
            .directory
            .root
            .join(format!("repair-commit-{id}.json")),
    )?
    .ok_or(StorageError::Corrupt)?;
    if committed.schema_version != 2
        || committed.plan != plan
        || committed.phase != commit::Phase::Published
    {
        return Err(StorageError::Corrupt.into());
    }
    let game = root.join("game");
    if read_role(&game, &plan)? != Some(Role::Replacement) {
        return Err(StorageError::Corrupt.into());
    }
    let release = state
        .verify_release_identity(plan.release_identity())
        .map_err(|_| StorageError::Corrupt)?;
    let receipt =
        installed_content::read_ready(root, &plan.installation)?.ok_or(StorageError::Corrupt)?;
    if receipt != plan.release_identity() || !install_worker::content_valid(&game, &release) {
        return Err(StorageError::Corrupt.into());
    }
    let saved: Option<Record> = read(&state.directory.root.join(name(id)))?;
    let phase = match saved {
        Some(saved) if saved.schema_version == 1 && saved.plan == plan => Some(saved.phase),
        Some(_) => return Err(StorageError::Corrupt.into()),
        None => None,
    };
    let backup = plan.backup();
    if phase == Some(Phase::Removed) {
        if directory_or_absent(&backup)? {
            return Err(StorageError::UnsafeFile.into());
        }
        return Ok(());
    }
    if !plan.original_present {
        if directory_or_absent(&backup)? {
            return Err(StorageError::UnsafeFile.into());
        }
        return record(state, &plan, Phase::Removed).map_err(Into::into);
    }
    if phase != Some(Phase::Empty) {
        // Validate the complete remaining tree before the first deletion in every
        // attempt. Keep its operation/role marker until all other entries are gone.
        if read_role(&backup, &plan)? != Some(Role::Original) {
            return Err(StorageError::Corrupt.into());
        }
        if phase.is_none() {
            record(state, &plan, Phase::Deleting)?;
        }
        hook(Point::Planned)?;
        for entry in std::fs::read_dir(&backup).map_err(|_| StorageError::Io)? {
            let entry = entry.map_err(|_| StorageError::Io)?;
            if entry.file_name() == tree_identity::name(&plan).as_str() {
                continue;
            }
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(|_| StorageError::Io)?;
            ordinary(&metadata)?;
            if metadata.is_dir() {
                std::fs::remove_dir_all(path)
            } else if metadata.is_file() {
                std::fs::remove_file(path)
            } else {
                return Err(StorageError::UnsafeFile.into());
            }
            .map_err(|_| StorageError::Io)?;
            hook(Point::EntryRemoved)?;
        }
        commit::sync(&backup)?;
        record(state, &plan, Phase::Empty)?;
        hook(Point::EmptyRecorded)?;
    }
    remove_empty(&backup, &plan, &mut hook)?;
    commit::sync(root)?;
    record(state, &plan, Phase::Removed)?;
    hook(Point::RemovedRecorded)?;
    Ok(())
}
fn remove_empty(
    backup: &Path,
    plan: &Plan,
    hook: &mut impl FnMut(Point) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    if !directory_or_absent(backup)? {
        return Ok(());
    }
    let marker_name = tree_identity::name(plan);
    let entries = std::fs::read_dir(backup)
        .map_err(|_| StorageError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::Io)?;
    if entries.len() == 1 && entries[0].file_name() == marker_name.as_str() {
        if read_role(backup, plan)? != Some(Role::Original) {
            return Err(StorageError::Corrupt);
        }
        std::fs::remove_file(backup.join(marker_name)).map_err(|_| StorageError::Io)?;
        hook(Point::MarkerRemoved)?;
        commit::sync(backup)?;
    } else if !entries.is_empty() {
        return Err(StorageError::UnsafeFile);
    }
    std::fs::remove_dir(backup).map_err(|_| StorageError::Io)?;
    hook(Point::DirectoryRemoved)?;
    Ok(())
}
fn name(id: Uuid) -> String {
    format!("repair-cleanup-{id}.json")
}
fn record(state: &mut DesktopState, plan: &Plan, phase: Phase) -> Result<(), StorageError> {
    let result = atomic::write(
        &state.directory.root,
        &name(plan.id),
        &Record {
            schema_version: 1,
            plan: plan.clone(),
            phase,
        },
    );
    state.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
    result
}
#[cfg(test)]
mod tests;
