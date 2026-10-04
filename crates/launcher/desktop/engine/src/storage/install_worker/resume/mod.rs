//! Explicit resume of an interrupted attempt, never automatic restart replay.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeError {
    Evidence(EvidenceError),
    Intent(IntentError),
}
impl From<EvidenceError> for ResumeError {
    fn from(error: EvidenceError) -> Self {
        Self::Evidence(error)
    }
}
impl From<StorageError> for ResumeError {
    fn from(error: StorageError) -> Self {
        Self::Intent(error.into())
    }
}
impl From<ContractError> for ResumeError {
    fn from(error: ContractError) -> Self {
        Self::Intent(error.into())
    }
}

/// Requires an explicit user action carrying the currently inspected revision.
/// Only interrupted in-process workers are supported, not cancelled/failed
/// terminal attempts, promoted content, or unobserved Wine guest processes.
pub fn resume(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    expected_revision: u64,
) -> Result<Worker, ResumeError> {
    resume_with(
        state,
        id,
        expected_revision,
        catalog::URL.into(),
        download_client()?,
    )
}
fn resume_with(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    expected_revision: u64,
    manifest_url: String,
    http: reqwest::Client,
) -> Result<Worker, ResumeError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let input = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let snapshot = owner.operations().snapshot();
        if snapshot.revision != expected_revision {
            return Err(ContractError::StaleRevision.into());
        }
        let operation = snapshot
            .operation
            .as_ref()
            .ok_or(ContractError::UnknownOperation)?;
        if operation.id != id {
            return Err(ContractError::UnknownOperation.into());
        }
        if operation.state != OperationState::ReconciliationRequired {
            return Err(ContractError::InvalidTransition.into());
        }
        if owner.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let release = owner.cached_install_release()?;
        let intent = owner.install_intent()?.ok_or(StorageError::Corrupt)?;
        let ownership = lock_and_validate(&intent, &release)?;
        owner
            .operations_mut()?
            .reconcile(id, OperationState::Running)?;
        TaskInputs {
            intent,
            state_root: owner.directory.root.clone(),
            release,
            manifest_url,
            http,
            ownership: Some(ownership),
        }
    };
    Ok(spawn_worker(runtime, state, id, input))
}

fn lock_and_validate(
    intent: &InstallIntent,
    release: &VerifiedRelease,
) -> Result<File, StorageError> {
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
        return Err(StorageError::InvalidDirectory);
    }
    real_directory(&intent.destination)?;
    let marker_path = intent.destination.join(".cimmeria-install.json");
    ensure_regular_or_absent(&marker_path)?;
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(marker_path)
        .map_err(|_| StorageError::Corrupt)?;
    match marker.try_lock() {
        Ok(()) => (),
        Err(std::fs::TryLockError::WouldBlock) => return Err(StorageError::InUse),
        Err(_) => return Err(StorageError::Io),
    }
    let saved: InstallIntent = read_open(&marker)?;
    if saved != *intent {
        return Err(StorageError::Corrupt);
    }
    // Promotion/receipt uncertainty is inspected separately; never replace it.
    for name in ["game", "content-ready.json"] {
        match std::fs::symlink_metadata(intent.destination.join(name)) {
            Err(error) if error.kind() == ErrorKind::NotFound => (),
            _ => return Err(StorageError::Busy),
        }
    }
    let stage = intent
        .destination
        .join(format!(".cimmeria-stage-{}", intent.operation_id));
    match std::fs::symlink_metadata(&stage) {
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(marker),
        Err(_) => return Err(StorageError::Io),
        Ok(_) => (),
    }
    real_directory(&stage)?;
    validate_tree(&stage)?;
    if let Some(saved) =
        read::<crate::state::InstalledState>(&crate::state::InstalledState::path(&stage))?
    {
        let manifest = release.manifest();
        if saved.seed_adopted
            || saved
                .seed_sha256
                .as_ref()
                .is_some_and(|hash| hash != &manifest.seed.sha256)
            || saved.applied_patches.iter().any(|key| {
                !manifest
                    .patches
                    .iter()
                    .any(|patch| patch.state_key() == *key)
            })
        {
            return Err(StorageError::Corrupt);
        }
    }
    Ok(marker)
}
fn safe_type(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        } // Reparse point/junction.
    }
    !metadata.file_type().is_symlink() && (metadata.is_dir() || metadata.is_file())
}
fn real_directory(path: &Path) -> Result<(), StorageError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| StorageError::Io)?;
    if !metadata.is_dir() || !safe_type(&metadata) {
        return Err(StorageError::UnsafeFile);
    }
    Ok(())
}
fn validate_tree(root: &Path) -> Result<(), StorageError> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).map_err(|_| StorageError::Io)? {
            let entry = entry.map_err(|_| StorageError::Io)?;
            let metadata = std::fs::symlink_metadata(entry.path()).map_err(|_| StorageError::Io)?;
            if !safe_type(&metadata) {
                return Err(StorageError::UnsafeFile);
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
