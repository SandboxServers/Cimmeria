//! Reconcile interrupted native content work without replaying a mutation.
use super::*;
use crate::{catalog::VerifiedRelease, OperationState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    /// The destination is missing or empty; no owned output can be resumed there.
    NoOutput,
    /// A matching durable completion receipt and current content checks passed.
    ContentPrepared,
    /// Owned output exists, but completion is not established. Keep the gate.
    Partial,
}

/// Native-only and serialized with command admission. Applies only to this
/// launcher's in-process content worker, not an unobserved Wine guest process.
/// Never downloads, extracts, deletes or resumes anything.
pub fn reconcile(
    state: &mut DesktopState,
    release: &VerifiedRelease,
) -> Result<Recovery, IntentError> {
    if state.requires_reopen() {
        return Err(StorageError::PersistenceUncertain.into());
    }
    let operation = state
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if operation.state != OperationState::ReconciliationRequired {
        return Err(ContractError::InvalidTransition.into());
    }
    let intent = state.install_intent()?.ok_or(StorageError::Corrupt)?;
    if release.digest() != intent.manifest_digest {
        return Err(ContractError::IdentityConflict.into());
    }
    if !intent.backend.is_native() {
        return Err(ContractError::InvalidTransition.into());
    }
    // Refuse a redirected parent, even when the resulting tree looks plausible.
    let parent = intent
        .destination
        .parent()
        .ok_or(StorageError::InvalidDirectory)?;
    let name = intent
        .destination
        .file_name()
        .ok_or(StorageError::InvalidDirectory)?;
    if parent
        .canonicalize()
        .map_err(|_| StorageError::InvalidDirectory)?
        .join(name)
        != intent.destination
    {
        return Err(StorageError::InvalidDirectory.into());
    }
    match std::fs::symlink_metadata(&intent.destination) {
        Err(error) if error.kind() == ErrorKind::NotFound => {
            state
                .operations_mut()?
                .reconcile(intent.operation_id, OperationState::Failed)?;
            return Ok(Recovery::NoOutput);
        }
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => (),
        _ => return Err(StorageError::UnsafeFile.into()),
    }
    // A crash can occur after creating an empty destination, before its marker.
    if std::fs::read_dir(&intent.destination)
        .map_err(|_| StorageError::Io)?
        .next()
        .is_none()
    {
        state
            .operations_mut()?
            .reconcile(intent.operation_id, OperationState::Failed)?;
        return Ok(Recovery::NoOutput);
    }
    let marker_path = intent.destination.join(".cimmeria-install.json");
    ensure_regular_or_absent(&marker_path)?;
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&marker_path)
        .map_err(|_| StorageError::Corrupt)?;
    match marker.try_lock() {
        Ok(()) => (),
        Err(std::fs::TryLockError::WouldBlock) => return Err(StorageError::InUse.into()),
        Err(_) => return Err(StorageError::Io.into()),
    }
    // Keep the marker handle/lock through the final journal commit.
    let owner: InstallIntent = read_open(&marker)?;
    if owner != intent {
        return Err(StorageError::Corrupt.into());
    }
    let receipt: Option<InstallIntent> = read(&intent.destination.join("content-ready.json"))?;
    let content = intent.destination.join("game");
    let prepared = receipt.as_ref() == Some(&intent)
        && std::fs::symlink_metadata(&content)
            .is_ok_and(|meta| meta.is_dir() && !meta.file_type().is_symlink())
        && super::install_worker::content_valid(&content, release);
    if !prepared {
        return Ok(Recovery::Partial);
    }
    state
        .operations_mut()?
        .reconcile(intent.operation_id, OperationState::Succeeded)?;
    Ok(Recovery::ContentPrepared)
}

#[cfg(test)]
mod tests;
