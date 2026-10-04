//! Game Update admission binds two authenticated releases to one permanent owner.
//! Admission persists intent only; replacement and recovery are separate phases.
use super::repair::{directory_or_absent, lock_owner, ordinary};
use super::*;
use crate::owner_lock::OwnerLock;
pub mod abandon;
pub mod cleanup;
pub mod commit;
pub mod discard;
pub mod preparation;
pub mod recovery;
mod staged_content;
use super::repair::tree_identity;
use crate::{catalog::VerifiedRelease, OperationKind};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub id: Uuid,
    pub owner: InstallIntent,
    pub previous: ReleaseIdentity,
    pub target: ReleaseIdentity,
}
impl tree_identity::TreePlan for Plan {
    fn marker_name(&self) -> String {
        format!(".cimmeria-update-tree-{}.json", self.id)
    }
}
impl Plan {
    pub fn work_directory(&self) -> PathBuf {
        self.owner
            .destination
            .join(format!(".cimmeria-update-{}", self.id))
    }
    pub fn stage(&self) -> PathBuf {
        self.work_directory().join("game")
    }
    pub fn backup(&self) -> PathBuf {
        self.owner
            .destination
            .join(format!(".cimmeria-update-backup-{}", self.id))
    }
    fn valid(&self) -> bool {
        self.schema_version == 1
            && self.owner.schema_version == 1
            && self.owner.destination.is_absolute()
            && !self.id.is_nil()
            && self.id != self.owner.operation_id
            && self.id != self.previous.evidence_id
            && !self.previous.evidence_id.is_nil()
            && self.target.evidence_id == self.id
            && self.target.manifest_digest != self.previous.manifest_digest
    }
    fn digest(&self) -> Result<[u8; 32], StorageError> {
        Ok(Sha256::digest(serde_json::to_vec(self).map_err(|_| StorageError::Corrupt)?).into())
    }
}

/// All fields come from native reviewed state/catalog, never renderer paths or bytes.
pub struct Request<'a> {
    pub id: Uuid,
    pub operation_revision: u64,
    pub installation_id: Uuid,
    pub expected_current: ReleaseIdentity,
    pub target: &'a VerifiedRelease,
    pub confirmed: bool,
}
pub struct Admission {
    pub plan: Plan,
    pub dispatch: bool,
}
fn name(id: Uuid) -> String {
    format!("update-plan-{id}.json")
}
impl DesktopState {
    /// A separate confirmed transition reconstructs the previous authenticated
    /// release. The old backup is never treated as executable rollback evidence.
    pub fn admit_update_rollback(
        &mut self,
        completed_update: Uuid,
        id: Uuid,
        revision: u64,
        expected_current: ReleaseIdentity,
        confirmed: bool,
    ) -> Result<Admission, IntentError> {
        if !confirmed {
            return Err(ContractError::InvalidTransition.into());
        }
        let record: commit::Record = read(
            &self
                .directory
                .root
                .join(format!("update-commit-{completed_update}.json")),
        )?
        .ok_or(StorageError::Corrupt)?;
        let saved: Plan = read(&self.directory.root.join(name(completed_update)))?
            .ok_or(StorageError::Corrupt)?;
        if saved != record.plan
            || record.schema_version != 2
            || record.phase != commit::Phase::Published
            || record.plan.id != completed_update
            || !record.plan.valid()
            || record.plan.target != expected_current
        {
            return Err(ContractError::IdentityConflict.into());
        }
        let release = self
            .verify_release_identity(record.plan.previous)
            .map_err(|_| StorageError::Corrupt)?;
        self.admit_update(Request {
            id,
            operation_revision: revision,
            installation_id: record.plan.owner.operation_id,
            expected_current,
            target: &release,
            confirmed,
        })
    }

    pub fn admit_update(&mut self, request: Request<'_>) -> Result<Admission, IntentError> {
        self.ensure_updater_idle()?;
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if !request.confirmed {
            return Err(ContractError::InvalidTransition.into());
        }
        if request.id.is_nil() {
            return Err(ContractError::IdentityConflict.into());
        }
        if let Some(op) = self.operations.snapshot().operation.as_ref() {
            if op.id == request.id {
                let plan = self.update_plan()?.ok_or(ContractError::IdentityConflict)?;
                if plan.owner.operation_id != request.installation_id
                    || plan.previous != request.expected_current
                    || plan.target.manifest_digest != request.target.digest()
                {
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
        if request.operation_revision != self.operations.snapshot().revision {
            return Err(ContractError::StaleRevision.into());
        }
        if self.compatibility.for_release(request.target).blocks() {
            return Err(IntentError::LauncherTooOld);
        }
        let installed = self.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent.operation_id != request.installation_id
            || installed.current_release != request.expected_current
        {
            return Err(ContractError::IdentityConflict.into());
        }
        let _owner = repair::lock_owner(&installed.intent)?;
        let game = std::fs::symlink_metadata(installed.intent.destination.join("game"))
            .map_err(|_| StorageError::Corrupt)?;
        if !failed_cleanup::plain(&game) || !game.is_dir() {
            return Err(StorageError::UnsafeFile.into());
        }
        let plan = Plan {
            schema_version: 1,
            id: request.id,
            owner: installed.intent,
            previous: installed.current_release,
            target: ReleaseIdentity {
                evidence_id: request.id,
                manifest_digest: request.target.digest(),
            },
        };
        if !plan.valid() {
            return Err(ContractError::IdentityConflict.into());
        }
        for path in [
            self.directory.root.join(name(plan.id)),
            self.directory
                .root
                .join(format!("release-evidence-{}.bin", plan.id)),
            plan.work_directory(),
            plan.backup(),
        ] {
            match std::fs::symlink_metadata(path) {
                Err(error) if error.kind() == ErrorKind::NotFound => (),
                Err(_) => return Err(StorageError::Io.into()),
                Ok(_) => return Err(ContractError::IdentityConflict.into()),
            }
        }
        self.save_release_evidence(plan.id, request.target)?;
        let saved = atomic::write(&self.directory.root, &name(plan.id), &plan);
        self.preferences_uncertain |= saved == Err(StorageError::PersistenceUncertain);
        saved?;
        let (_, dispatch) = self.operations_mut()?.begin(
            plan.id,
            OperationKind::Update,
            plan.digest()?,
            request.operation_revision,
        )?;
        Ok(Admission { plan, dispatch })
    }

    pub fn update_plan(&self) -> Result<Option<Plan>, IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let Some(op) = self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .filter(|op| op.kind == OperationKind::Update)
        else {
            return Ok(None);
        };
        let plan: Plan =
            read(&self.directory.root.join(name(op.id)))?.ok_or(StorageError::Corrupt)?;
        if !plan.valid() || plan.id != op.id || plan.digest()? != op.intent_digest {
            return Err(StorageError::Corrupt.into());
        }
        for identity in [plan.previous, plan.target] {
            self.verify_release_identity(identity)
                .map_err(|_| StorageError::Corrupt)?;
        }
        Ok(Some(plan))
    }
}
#[cfg(test)]
mod tests;

#[cfg(feature = "test-support")]
pub mod test_support;

/// Uninstall validates auxiliary content against the durable completed plan.
pub(super) fn uninstall_artifact(
    state_root: &Path,
    path: &Path,
    owner: &InstallIntent,
    partial: bool,
) -> Result<bool, StorageError> {
    let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(false);
    };
    let work_prefix = ".cimmeria-update-";
    let backup_prefix = ".cimmeria-update-backup-";
    let (id, backup) = if let Some(id) = filename.strip_prefix(backup_prefix) {
        (id, true)
    } else if let Some(id) = filename.strip_prefix(work_prefix) {
        (id, false)
    } else {
        return Ok(false);
    };
    let id = Uuid::parse_str(id).map_err(|_| StorageError::Corrupt)?;
    let plan: Plan = read(&state_root.join(name(id)))?.ok_or(StorageError::Corrupt)?;
    let checkpoint: commit::Record =
        read(&state_root.join(format!("update-commit-{id}.json")))?.ok_or(StorageError::Corrupt)?;
    if !plan.valid()
        || plan.id != id
        || plan.owner != *owner
        || checkpoint.schema_version != 2
        || checkpoint.plan != plan
        || checkpoint.phase != commit::Phase::Published
    {
        return Err(StorageError::Corrupt);
    }
    if backup {
        let marker = path.join(tree_identity::name(&plan));
        if (!partial || marker.try_exists().map_err(|_| StorageError::Io)?)
            && tree_identity::read_role(path, &plan)? != Some(tree_identity::Role::Original)
        {
            return Err(StorageError::Corrupt);
        }
    } else {
        let marker = path.join("owner.json");
        if (!partial || marker.try_exists().map_err(|_| StorageError::Io)?)
            && read::<Plan>(&marker)?.as_ref() != Some(&plan)
        {
            return Err(StorageError::Corrupt);
        }
    }
    failed_cleanup::validate_tree(path)?;
    Ok(true)
}

#[cfg(test)]
mod journey_tests;
