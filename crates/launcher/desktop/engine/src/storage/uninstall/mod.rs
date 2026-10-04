//! Confirmed removal of an owned installation. Detach by rename before deleting;
//! a persisted plan and retained owner marker make interrupted removal inspectable.
use super::*;
use crate::{OperationKind, OperationState};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema_version: u32,
    id: Uuid,
    installation: InstallIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    current_release: Option<ReleaseIdentity>,
}
impl Plan {
    fn release_identity(&self) -> ReleaseIdentity {
        self.current_release
            .unwrap_or_else(|| self.installation.release_identity())
    }
    fn digest(&self) -> Result<[u8; 32], StorageError> {
        Ok(Sha256::digest(serde_json::to_vec(self).map_err(|_| StorageError::Corrupt)?).into())
    }
    fn detached(&self) -> Result<PathBuf, StorageError> {
        Ok(self
            .installation
            .destination
            .parent()
            .ok_or(StorageError::InvalidDirectory)?
            .join(format!(".cimmeria-uninstall-{}", self.id)))
    }
}
fn name(id: Uuid) -> String {
    format!("uninstall-{id}.json")
}
fn detached_name(id: Uuid) -> String {
    format!("uninstall-detached-{id}.json")
}

#[derive(Debug, Serialize)]
pub struct Target {
    pub installation_id: Uuid,
    pub directory: PathBuf,
    pub recovery: bool,
}

impl DesktopState {
    /// Display/confirmation target only. Mutation independently checks identity.
    pub fn uninstall_target(&mut self) -> Result<Option<Target>, StorageError> {
        if self.requires_reopen() {
            return Ok(None);
        }
        if let Some(op) = self.operations.snapshot().operation.as_ref() {
            if op.kind == OperationKind::Uninstall {
                if op.state == OperationState::Succeeded {
                    return Ok(None);
                }
                let plan: Plan =
                    read(&self.directory.root.join(name(op.id)))?.ok_or(StorageError::Corrupt)?;
                if plan.schema_version != 1
                    || plan.id != op.id
                    || plan.digest()? != op.intent_digest
                {
                    return Err(StorageError::Corrupt);
                }
                return Ok(Some(Target {
                    installation_id: plan.installation.operation_id,
                    directory: plan.installation.destination,
                    recovery: op.state == OperationState::ReconciliationRequired,
                }));
            }
            if !op.state.terminal() {
                return Ok(None);
            }
        }
        match self.installed_content() {
            Ok(installed) => Ok(installed.map(|installed| Target {
                installation_id: installed.intent.operation_id,
                directory: installed.intent.destination,
                recovery: false,
            })),
            Err(StorageError::Busy) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Explicit user confirmation is required for each call, including recovery.
    /// Native command thread only: holds state ownership through stop and deletion.
    /// No cancellation after admission; a lost response requires inspection.
    pub fn uninstall(
        &mut self,
        id: Uuid,
        revision: u64,
        installation_id: Uuid,
        confirmed: bool,
    ) -> Result<(), IntentError> {
        self.ensure_updater_idle()?;
        if !confirmed {
            return Err(ContractError::InvalidTransition.into());
        }
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if self.operations.snapshot().revision != revision {
            return Err(ContractError::StaleRevision.into());
        }
        let current = self.operations.snapshot().operation.as_ref();
        if current.is_some_and(|op| op.id == id) {
            let op = current.unwrap();
            if op.kind != OperationKind::Uninstall {
                return Err(ContractError::IdentityConflict.into());
            }
            let plan: Plan =
                read(&self.directory.root.join(name(id)))?.ok_or(StorageError::Corrupt)?;
            if plan.schema_version != 1
                || plan.id != id
                || plan.installation.operation_id != installation_id
                || plan.digest()? != op.intent_digest
            {
                return Err(StorageError::Corrupt.into());
            }
            if op.state == OperationState::Succeeded {
                return Ok(());
            }
            if op.state != OperationState::ReconciliationRequired {
                return Err(ContractError::Busy.into());
            }
            return self.remove_installation(&plan);
        }
        if current.is_some_and(|op| !op.state.terminal()) {
            return Err(ContractError::Busy.into());
        }
        let installed = self.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent.operation_id != installation_id {
            return Err(ContractError::IdentityConflict.into());
        }
        let plan = Plan {
            schema_version: 1,
            id,
            current_release: (installed.current_release != installed.intent.release_identity())
                .then_some(installed.current_release),
            installation: installed.intent,
        };
        // Refuse any foreign top-level content before admitting destructive work.
        inspect_tree(
            &plan.installation.destination,
            &plan.installation,
            plan.release_identity(),
            false,
        )?;
        if std::fs::symlink_metadata(plan.detached()?).is_ok() {
            return Err(StorageError::UnsafeFile.into());
        }
        self.write_uninstall_record(&name(id), &plan)?;
        self.operations_mut()?
            .begin(id, OperationKind::Uninstall, plan.digest()?, revision)?;
        self.operations_mut()?
            .observe(id, OperationState::Running)?;
        self.remove_installation(&plan)
    }

    fn write_uninstall_record(&mut self, name: &str, plan: &Plan) -> Result<(), StorageError> {
        let result = atomic::write(&self.directory.root, name, plan);
        self.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
        result
    }

    fn remove_installation(&mut self, plan: &Plan) -> Result<(), IntentError> {
        let result = self.remove_installation_files(plan);
        if let Err(error) = result {
            if let Ok(operations) = self.operations_mut() {
                let _ = operations.mark_uncertain(plan.id);
            }
            return Err(error);
        }
        let operation = self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .ok_or(StorageError::Corrupt)?;
        let recovering = operation.state == OperationState::ReconciliationRequired;
        let result = if recovering {
            self.operations_mut()?
                .reconcile(plan.id, OperationState::Succeeded)
        } else {
            self.operations_mut()?
                .observe(plan.id, OperationState::Succeeded)
        };
        if let Err(error) = result {
            if let Ok(operations) = self.operations_mut() {
                let _ = operations.mark_uncertain(plan.id);
            }
            return Err(error.into());
        }
        Ok(())
    }

    fn remove_installation_files(&mut self, plan: &Plan) -> Result<(), IntentError> {
        #[cfg(target_os = "macos")]
        let _stopped = if !plan.installation.backend.is_native() {
            Some(crate::mac_wine::recovery::stop_for_install(
                self,
                &plan.installation,
            )?)
        } else {
            None
        };
        #[cfg(not(target_os = "macos"))]
        if !plan.installation.backend.is_native() {
            return Err(ContractError::InvalidTransition.into());
        }
        let root = &plan.installation.destination;
        let detached = plan.detached()?;
        let parent = root.parent().ok_or(StorageError::InvalidDirectory)?;
        if parent
            .canonicalize()
            .map_err(|_| StorageError::InvalidDirectory)?
            != parent
        {
            return Err(StorageError::InvalidDirectory.into());
        }
        let recorded: Option<Plan> = read(&self.directory.root.join(detached_name(plan.id)))?;
        let committed_detachment = if let Some(recorded) = recorded {
            if recorded.digest()? != plan.digest()? {
                return Err(StorageError::Corrupt.into());
            }
            true
        } else {
            false
        };
        match std::fs::symlink_metadata(&detached) {
            Err(error) if error.kind() == ErrorKind::NotFound && committed_detachment => {
                // Only a durable detach checkpoint can justify a missing tombstone.
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let owner = inspect_tree(root, &plan.installation, plan.release_identity(), false)?;
                // Windows will not rename a directory containing a locked file.
                // Admission is still serialized by DesktopState; no worker is live.
                drop(owner);
                std::fs::rename(root, &detached).map_err(|_| StorageError::Io)?;
                sync(parent)?;
                self.write_uninstall_record(&detached_name(plan.id), plan)?;
                remove_detached(&detached, &plan.installation, plan.release_identity())?;
            }
            Ok(_) => {
                // Rename may have completed just before its checkpoint write.
                if !committed_detachment {
                    drop(inspect_tree(
                        &detached,
                        &plan.installation,
                        plan.release_identity(),
                        false,
                    )?);
                    self.write_uninstall_record(&detached_name(plan.id), plan)?;
                }
                remove_detached(&detached, &plan.installation, plan.release_identity())?;
            }
            Err(_) => return Err(StorageError::Io.into()),
        }
        sync(parent)?;
        // Preserve preferences/consent, release evidence, logs and runtime caches.
        // Refuse to forget an installation other than the one being removed.
        self.forget_installed_content(&plan.installation)?;
        Ok(())
    }
}

fn sync(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| StorageError::Io)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Validate the complete tree before deleting anything. No symlink/reparse entry
/// is followed. Partial deletion may omit children but must retain its owner.
fn inspect_tree(
    root: &Path,
    intent: &InstallIntent,
    release: ReleaseIdentity,
    partial: bool,
) -> Result<Option<File>, StorageError> {
    let meta = std::fs::symlink_metadata(root).map_err(|_| StorageError::Io)?;
    if !failed_cleanup::plain(&meta) || !meta.is_dir() {
        return Err(StorageError::UnsafeFile);
    }
    let entries = std::fs::read_dir(root)
        .map_err(|_| StorageError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::Io)?;
    if partial && entries.is_empty() {
        return Ok(None);
    }
    let stage = format!(".cimmeria-stage-{}", intent.operation_id);
    let cache = format!(".cimmeria-cache-{}", intent.operation_id);
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_str().ok_or(StorageError::UnsafeFile)?;
        let meta = std::fs::symlink_metadata(entry.path()).map_err(|_| StorageError::Io)?;
        if !failed_cleanup::plain(&meta)
            || !((name == "game" || name == stage || name == cache) && meta.is_dir()
                || (name == ".cimmeria-install.json" || name == "content-ready.json")
                    && meta.is_file())
        {
            return Err(StorageError::UnsafeFile);
        }
    }
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(".cimmeria-install.json"))
        .map_err(|_| StorageError::Corrupt)?;
    marker.try_lock().map_err(|_| StorageError::InUse)?;
    let owner: InstallIntent = read_open(&marker)?;
    if owner != *intent {
        return Err(StorageError::Corrupt);
    }
    let receipt = installed_content::read_ready(root, intent)?;
    if receipt.is_some_and(|receipt| receipt != release) || (!partial && receipt.is_none()) {
        return Err(StorageError::Corrupt);
    }
    failed_cleanup::validate_tree(root)?;
    Ok(Some(marker))
}

fn remove_detached(
    root: &Path,
    intent: &InstallIntent,
    release: ReleaseIdentity,
) -> Result<(), StorageError> {
    let owner = inspect_tree(root, intent, release, true)?;
    for entry in std::fs::read_dir(root).map_err(|_| StorageError::Io)? {
        let entry = entry.map_err(|_| StorageError::Io)?;
        if entry.file_name() == ".cimmeria-install.json" {
            continue;
        }
        let meta = std::fs::symlink_metadata(entry.path()).map_err(|_| StorageError::Io)?;
        let result = if meta.is_dir() {
            std::fs::remove_dir_all(entry.path())
        } else {
            std::fs::remove_file(entry.path())
        };
        result.map_err(|_| StorageError::Io)?;
    }
    // Keep the owner through all content deletion; remove it last on Windows too.
    drop(owner);
    match std::fs::remove_file(root.join(".cimmeria-install.json")) {
        Ok(()) => (),
        Err(error) if error.kind() == ErrorKind::NotFound => (),
        Err(_) => return Err(StorageError::Io),
    }
    std::fs::remove_dir(root).map_err(|_| StorageError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests;
