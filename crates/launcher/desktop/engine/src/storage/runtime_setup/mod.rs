//! Durable prerequisite attempt identity and dispatch evidence, separate from
//! content installation. Native coordinators own prefix/process inspection.
use super::*;
use crate::{OperationKind, OperationState};
use cimmeria_runtime_probe::prerequisite::{decode_result, PrepareResult, ResultKind};
mod prepared;
pub use prepared::PreparedRuntime;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub id: Uuid,
    pub installation: InstallIntent,
    pub prefix_generation: Uuid,
    pub runtime_sha256: [u8; 32],
    pub helper_sha256: [u8; 32],
    /// Fixed original PhysX package/recipe and probe policy version.
    pub policy_version: u32,
}
impl Plan {
    pub fn digest(&self) -> Result<[u8; 32], StorageError> {
        Ok(Sha256::digest(serde_json::to_vec(self).map_err(|_| StorageError::Corrupt)?).into())
    }
    pub fn prefix_directory(&self, state_root: &Path) -> PathBuf {
        state_root
            .join("game-prefixes")
            .join(self.installation.operation_id.to_string())
            .join(self.prefix_generation.to_string())
    }
    fn valid(&self) -> bool {
        self.schema_version == 1
            && self.policy_version == 1
            && !self.id.is_nil()
            && !self.prefix_generation.is_nil()
            && self.helper_sha256 != [0; 32]
            && self.runtime_sha256 != [0; 32]
            && matches!(self.installation.backend, ExtractionBackend::Wine { runtime_sha256, .. } if runtime_sha256 == self.runtime_sha256)
    }
}
pub struct Admission {
    pub plan: Plan,
    pub dispatch: bool,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    LaunchIntent,
    HostStarted,
    Observed,
    Quiescent,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    schema_version: u32,
    pub operation_id: Uuid,
    pub plan_digest: [u8; 32],
    pub phase: Phase,
    pub host_pid: Option<u32>,
    pub result: Option<PrepareResult>,
}
fn plan_name(id: Uuid) -> String {
    format!("runtime-plan-{id}.json")
}
fn record_name(id: Uuid) -> String {
    format!("runtime-attempt-{id}.json")
}
impl DesktopState {
    /// Native artifact identities only; no webview-controlled executable or URL.
    /// Admission writes state only and never creates a Wine prefix or game files.
    pub fn admit_runtime_setup(
        &mut self,
        id: Uuid,
        revision: u64,
        installation_id: Uuid,
        runtime_sha256: [u8; 32],
        helper_sha256: [u8; 32],
    ) -> Result<Admission, IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if let Some(op) = self.operations.snapshot().operation.as_ref() {
            if op.id == id {
                let plan = self
                    .runtime_plan()?
                    .ok_or(ContractError::IdentityConflict)?;
                if plan.installation.operation_id != installation_id
                    || plan.runtime_sha256 != runtime_sha256
                    || plan.helper_sha256 != helper_sha256
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
        if revision != self.operations.snapshot().revision {
            return Err(ContractError::StaleRevision.into());
        }
        let installed = self.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent.operation_id != installation_id {
            return Err(ContractError::IdentityConflict.into());
        }
        if !install_worker::content_valid(
            &installed.intent.destination.join("game"),
            &installed.release,
        ) {
            return Err(StorageError::Corrupt.into());
        }
        let plan = Plan {
            schema_version: 1,
            id,
            installation: installed.intent,
            prefix_generation: Uuid::new_v4(),
            runtime_sha256,
            helper_sha256,
            policy_version: 1,
        };
        if !plan.valid() {
            return Err(ContractError::IdentityConflict.into());
        }
        // Refuse reuse of a previous attempt, including an orphan from failed admission.
        if self
            .directory
            .root
            .join(plan_name(id))
            .try_exists()
            .map_err(|_| StorageError::Io)?
        {
            return Err(ContractError::IdentityConflict.into());
        }
        self.write_runtime(&plan_name(id), &plan)?;
        self.select_runtime_attempt(&plan)?;
        let (_, dispatch) = self.operations_mut()?.begin(
            id,
            OperationKind::PrepareRuntime,
            plan.digest()?,
            revision,
        )?;
        Ok(Admission { plan, dispatch })
    }
    pub fn runtime_plan(&self) -> Result<Option<Plan>, IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let Some(op) = self.operations.snapshot().operation.as_ref() else {
            return Ok(None);
        };
        if op.kind != OperationKind::PrepareRuntime {
            return Ok(None);
        }
        let plan: Plan =
            read(&self.directory.root.join(plan_name(op.id)))?.ok_or(StorageError::Corrupt)?;
        if !plan.valid() || plan.id != op.id || plan.digest()? != op.intent_digest {
            return Err(StorageError::Corrupt.into());
        }
        Ok(Some(plan))
    }
    pub fn runtime_record(&self) -> Result<Option<Record>, IntentError> {
        let plan = self
            .runtime_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        read_record(&self.directory.root, &plan)
    }

    /// Persist dispatch intent before spawning. Repeated dispatch is refused even
    /// when the previous host was never observed; recovery must inspect that gap.
    pub fn begin_runtime_dispatch(&mut self, id: Uuid) -> Result<Plan, IntentError> {
        let plan = self.runtime_current(id, &[OperationState::Starting])?;
        if self.runtime_record()?.is_some() {
            return Err(ContractError::Busy.into());
        }
        let installed = self.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent != plan.installation
            || !install_worker::content_valid(
                &installed.intent.destination.join("game"),
                &installed.release,
            )
        {
            return Err(StorageError::Corrupt.into());
        }
        self.operations_mut()?
            .observe(id, OperationState::Running)?;
        self.write_runtime(
            &record_name(id),
            &Record {
                schema_version: 1,
                operation_id: id,
                plan_digest: plan.digest()?,
                phase: Phase::LaunchIntent,
                host_pid: None,
                result: None,
            },
        )?;
        Ok(plan)
    }
    pub fn record_runtime_host(&mut self, id: Uuid, pid: u32) -> Result<(), IntentError> {
        self.runtime_current(id, &[OperationState::Running])?;
        let mut record = self.runtime_record()?.ok_or(StorageError::Corrupt)?;
        if pid == 0 || record.phase != Phase::LaunchIntent {
            return Err(ContractError::InvalidTransition.into());
        }
        record.host_pid = Some(pid);
        record.phase = Phase::HostStarted;
        self.write_runtime(&record_name(id), &record)
    }
    /// Only a fully decoded result plus successful host exit reaches this method.
    /// The platform coordinator must still stop/wait the exclusive prefix.
    pub fn record_runtime_observation(
        &mut self,
        id: Uuid,
        result: PrepareResult,
    ) -> Result<(), IntentError> {
        let plan = self.runtime_current(
            id,
            &[OperationState::Running, OperationState::CancelRequested],
        )?;
        let mut record = self.runtime_record()?.ok_or(StorageError::Corrupt)?;
        if record.phase != Phase::HostStarted {
            return Err(ContractError::InvalidTransition.into());
        }
        decode_result(
            &serde_json::to_vec(&result).map_err(|_| StorageError::Corrupt)?,
            plan.id,
            plan.prefix_generation,
        )
        .map_err(|_| StorageError::Corrupt)?;
        record.phase = Phase::Observed;
        record.result = Some(result);
        self.write_runtime(&record_name(id), &record)
    }
    /// Native coordinator only, after verified exclusive-prefix stop/wait. Never
    /// expose this evidence-recording API directly to IPC. Reopen alone cannot call
    /// it or establish quiescence. Success means checked prerequisites, not Play.
    pub fn finish_runtime_after_stop(&mut self, id: Uuid) -> Result<(), IntentError> {
        self.finish_runtime_after_stop_with(id, || Ok(()))
    }
    fn finish_runtime_after_stop_with(
        &mut self,
        id: Uuid,
        before_commit: impl FnOnce() -> Result<(), IntentError>,
    ) -> Result<(), IntentError> {
        self.runtime_current(
            id,
            &[
                OperationState::Running,
                OperationState::CancelRequested,
                OperationState::ReconciliationRequired,
            ],
        )?;
        let mut record = self.runtime_record()?.ok_or(StorageError::Corrupt)?;
        if !matches!(record.phase, Phase::Observed | Phase::Quiescent) {
            return Err(ContractError::InvalidTransition.into());
        }
        let result = record.result.as_ref().ok_or(StorageError::Corrupt)?;
        let verified = verified_result(result);
        record.phase = Phase::Quiescent;
        self.write_runtime(&record_name(id), &record)?;
        before_commit()?;
        let terminal = if verified {
            OperationState::Succeeded
        } else {
            OperationState::Failed
        };
        if self.operations.snapshot().operation.as_ref().unwrap().state
            == OperationState::ReconciliationRequired
        {
            self.operations_mut()?.reconcile(id, terminal)?;
        } else {
            self.operations_mut()?.observe(id, terminal)?;
        }
        Ok(())
    }
    fn runtime_current(&self, id: Uuid, states: &[OperationState]) -> Result<Plan, IntentError> {
        let plan = self
            .runtime_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        let op = self.operations.snapshot().operation.as_ref().unwrap();
        if plan.id != id {
            return Err(ContractError::IdentityConflict.into());
        }
        if !states.contains(&op.state) {
            return Err(ContractError::InvalidTransition.into());
        }
        Ok(plan)
    }
    fn write_runtime<T: Serialize>(&mut self, name: &str, record: &T) -> Result<(), IntentError> {
        let result = atomic::write(&self.directory.root, name, record);
        self.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
        result.map_err(Into::into)
    }
}
#[cfg(test)]
pub(crate) mod tests;

fn read_record(root: &Path, plan: &Plan) -> Result<Option<Record>, IntentError> {
    let record: Option<Record> = read(&root.join(record_name(plan.id)))?;
    if let Some(record) = &record {
        if record.schema_version != 1
            || record.operation_id != plan.id
            || record.plan_digest != plan.digest()?
            || record.host_pid == Some(0)
            || (record.phase == Phase::LaunchIntent) != record.host_pid.is_none()
            || matches!(record.phase, Phase::Observed | Phase::Quiescent) != record.result.is_some()
        {
            return Err(StorageError::Corrupt.into());
        }
        if let Some(result) = &record.result {
            let bytes = serde_json::to_vec(result).map_err(|_| StorageError::Corrupt)?;
            decode_result(&bytes, plan.id, plan.prefix_generation)
                .map_err(|_| StorageError::Corrupt)?;
        }
    }
    Ok(record)
}

fn verified_result(result: &PrepareResult) -> bool {
    matches!(&result.result, ResultKind::Probed { report }
            if report.activation_context == (cimmeria_runtime_probe::LoadResult::Loaded {})
            && report.modules.iter().all(|module| module.result == (cimmeria_runtime_probe::LoadResult::Loaded {}))
            && report.physx_sdk == (cimmeria_runtime_probe::physx::SdkResult::InitializedAndReleased {}))
}
