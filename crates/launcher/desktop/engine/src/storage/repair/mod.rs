//! Same-release reconstruction identity. Admission never mutates game content.
use super::*;
use crate::OperationKind;
use sha2::{Digest, Sha256};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub id: Uuid,
    pub installation: InstallIntent,
    pub original_present: bool,
}
impl Plan {
    pub fn stage(&self) -> PathBuf {
        self.installation
            .destination
            .join(format!(".cimmeria-repair-{}", self.id))
    }
    pub fn backup(&self) -> PathBuf {
        self.installation
            .destination
            .join(format!(".cimmeria-backup-{}", self.id))
    }
    fn valid(&self) -> bool {
        self.schema_version == 1
            && !self.id.is_nil()
            && self.id != self.installation.operation_id
            && self.installation.schema_version == 1
            && self.installation.destination.is_absolute()
    }
    fn digest(&self) -> Result<[u8; 32], StorageError> {
        Ok(Sha256::digest(serde_json::to_vec(self).map_err(|_| StorageError::Corrupt)?).into())
    }
}
pub struct Admission {
    pub plan: Plan,
    pub dispatch: bool,
}
fn name(id: Uuid) -> String {
    format!("repair-plan-{id}.json")
}
impl DesktopState {
    pub fn admit_repair(
        &mut self,
        id: Uuid,
        revision: u64,
        installation_id: Uuid,
        confirmed: bool,
    ) -> Result<Admission, IntentError> {
        if !confirmed {
            return Err(ContractError::InvalidTransition.into());
        }
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if let Some(op) = self.operations.snapshot().operation.as_ref() {
            if op.id == id {
                let plan = self.repair_plan()?.ok_or(ContractError::IdentityConflict)?;
                if plan.installation.operation_id != installation_id {
                    return Err(ContractError::IdentityConflict.into());
                }
                return Ok(Admission {
                    plan,
                    dispatch: false,
                });
            }
            if !op.state.terminal() {
                return Err(ContractError::Busy.into());
            }
        }
        if self.operations.snapshot().revision != revision {
            return Err(ContractError::StaleRevision.into());
        }
        // Reverify the original signed release and permanent owner, independent of preferences.
        let installed = self.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent.operation_id != installation_id {
            return Err(ContractError::IdentityConflict.into());
        }
        let _owner = lock_owner(&installed.intent)?;
        let original_present = directory_or_absent(&installed.intent.destination.join("game"))?;
        let plan = Plan {
            schema_version: 1,
            id,
            installation: installed.intent,
            original_present,
        };
        if !plan.valid() {
            return Err(ContractError::IdentityConflict.into());
        }
        for path in [
            self.directory.root.join(name(id)),
            plan.stage(),
            plan.backup(),
        ] {
            match std::fs::symlink_metadata(path) {
                Err(e) if e.kind() == ErrorKind::NotFound => (),
                Err(_) => return Err(StorageError::Io.into()),
                Ok(_) => return Err(ContractError::IdentityConflict.into()),
            }
        }
        let result = atomic::write(&self.directory.root, &name(id), &plan);
        self.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
        result?;
        let (_, dispatch) =
            self.operations_mut()?
                .begin(id, OperationKind::Repair, plan.digest()?, revision)?;
        Ok(Admission { plan, dispatch })
    }
    pub fn repair_plan(&self) -> Result<Option<Plan>, IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let Some(op) = self.operations.snapshot().operation.as_ref() else {
            return Ok(None);
        };
        if op.kind != OperationKind::Repair {
            return Ok(None);
        }
        let plan: Plan =
            read(&self.directory.root.join(name(op.id)))?.ok_or(StorageError::Corrupt)?;
        if !plan.valid() || plan.id != op.id || plan.digest()? != op.intent_digest {
            return Err(StorageError::Corrupt.into());
        }
        Ok(Some(plan))
    }
}
fn directory_or_absent(path: &Path) -> Result<bool, StorageError> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(_) => Err(StorageError::Io),
        Ok(metadata) => {
            ordinary(&metadata)?;
            if !metadata.is_dir() {
                return Err(StorageError::UnsafeFile);
            }
            Ok(true)
        }
    }
}
fn ordinary(metadata: &std::fs::Metadata) -> Result<(), StorageError> {
    if metadata.file_type().is_symlink() {
        return Err(StorageError::UnsafeFile);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(StorageError::UnsafeFile);
        }
    }
    Ok(())
}
fn lock_owner(intent: &InstallIntent) -> Result<File, StorageError> {
    let path = intent.destination.join(".cimmeria-install.json");
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| StorageError::Corrupt)?;
    ordinary(&metadata)?;
    if !metadata.is_file() {
        return Err(StorageError::UnsafeFile);
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| StorageError::Io)?;
    file.try_lock().map_err(|_| StorageError::InUse)?;
    let saved: InstallIntent = read_open(&file)?;
    if saved != *intent {
        return Err(StorageError::Corrupt);
    }
    Ok(file)
}
#[cfg(test)]
mod tests;
