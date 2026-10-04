//! Durable external-helper dispatch checkpoints. A PID is diagnostic evidence,
//! never permission to kill a process or proof that a Wine guest has exited.
use super::*;
use crate::{helper_supervisor, OperationState};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum HelperPhase {
    LaunchIntent,
    HostStarted,
    Finished { result: HelperResult },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperResult {
    Completed,
    Cancelled,
    Failed,
    NotStarted,
    Uncertain,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelperRecord {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub attempt_id: Uuid,
    pub intent_digest: [u8; 32],
    pub phase: HelperPhase,
    pub host_pid: Option<u32>,
}
impl DesktopState {
    fn external_identity(&self, id: Uuid) -> Result<[u8; 32], IntentError> {
        let work = self.extraction_work(id)?;
        if work.installation.backend.is_native() {
            return Err(ContractError::InvalidTransition.into());
        }
        Ok(work.intent_digest)
    }
    pub fn helper_record(&self, id: Uuid) -> Result<Option<HelperRecord>, IntentError> {
        let digest = self.external_identity(id)?;
        self.read_helper_identity(id, digest)
    }
    #[cfg(target_os = "macos")]
    pub(crate) fn helper_record_for_install(
        &self,
        intent: &InstallIntent,
    ) -> Result<Option<HelperRecord>, IntentError> {
        self.read_helper_identity(intent.operation_id, intent.digest()?)
    }
    fn read_helper_identity(
        &self,
        id: Uuid,
        digest: [u8; 32],
    ) -> Result<Option<HelperRecord>, IntentError> {
        let record: Option<HelperRecord> = read(&self.directory.root.join(name(id)))?;
        if let Some(record) = &record {
            if record.schema_version != 1 {
                return Err(StorageError::UnsupportedSchema.into());
            }
            if record.operation_id != id
                || record.intent_digest != digest
                || record.host_pid == Some(0)
                || (record.phase == HelperPhase::LaunchIntent && record.host_pid.is_some())
                || (!matches!(
                    record.phase,
                    HelperPhase::LaunchIntent
                        | HelperPhase::Finished {
                            result: HelperResult::NotStarted
                        }
                ) && record.host_pid.is_none())
            {
                return Err(StorageError::Corrupt.into());
            }
        }
        Ok(record)
    }
    fn write_helper(&mut self, record: &HelperRecord) -> Result<(), IntentError> {
        if let Err(error) = atomic::write(&self.directory.root, &name(record.operation_id), record)
        {
            self.preferences_uncertain |= error == StorageError::PersistenceUncertain;
            return Err(error.into());
        }
        Ok(())
    }
    /// Commit before spawning. There is no automatic second attempt or replay.
    pub fn begin_helper(&mut self, id: Uuid) -> Result<HelperRecord, IntentError> {
        let digest = self.external_identity(id)?;
        let operation = self.operations().snapshot().operation.as_ref().unwrap();
        if operation.state != OperationState::Running {
            return Err(ContractError::InvalidTransition.into());
        }
        if self.helper_record(id)?.is_some() {
            return Err(ContractError::Busy.into());
        }
        let record = HelperRecord {
            schema_version: 1,
            operation_id: id,
            attempt_id: Uuid::new_v4(),
            intent_digest: digest,
            phase: HelperPhase::LaunchIntent,
            host_pid: None,
        };
        self.write_helper(&record)?;
        Ok(record)
    }
    /// Supervisor calls this before sending any extraction request.
    pub fn record_helper_host(
        &mut self,
        id: Uuid,
        attempt: Uuid,
        pid: u32,
    ) -> Result<(), IntentError> {
        let mut record = self.helper_record(id)?.ok_or(StorageError::Corrupt)?;
        if record.attempt_id != attempt {
            return Err(ContractError::IdentityConflict.into());
        }
        if pid == 0 || record.phase != HelperPhase::LaunchIntent {
            return Err(ContractError::InvalidTransition.into());
        }
        let operation = self.operations().snapshot().operation.as_ref().unwrap();
        if !matches!(
            operation.state,
            OperationState::Running | OperationState::CancelRequested
        ) {
            return Err(ContractError::InvalidTransition.into());
        }
        record.phase = HelperPhase::HostStarted;
        record.host_pid = Some(pid);
        self.write_helper(&record)
    }
    /// Native supervisor outcome only; this is not exposed to the webview.
    /// Failed persistence or an uncertain result must leave the operation gated.
    pub fn finish_helper(
        &mut self,
        id: Uuid,
        attempt: Uuid,
        outcome: helper_supervisor::Outcome,
    ) -> Result<(), IntentError> {
        let mut record = self.helper_record(id)?.ok_or(StorageError::Corrupt)?;
        if record.attempt_id != attempt {
            return Err(ContractError::IdentityConflict.into());
        }
        let operation = self.operations().snapshot().operation.as_ref().unwrap();
        if !matches!(
            operation.state,
            OperationState::Running | OperationState::CancelRequested
        ) {
            return Err(ContractError::InvalidTransition.into());
        }
        let result = match outcome {
            helper_supervisor::Outcome::Completed => HelperResult::Completed,
            helper_supervisor::Outcome::Cancelled => HelperResult::Cancelled,
            helper_supervisor::Outcome::Failed(_) => HelperResult::Failed,
            helper_supervisor::Outcome::NotStarted(_) => HelperResult::NotStarted,
            helper_supervisor::Outcome::ReconciliationRequired(_) => HelperResult::Uncertain,
        };
        match record.phase {
            HelperPhase::HostStarted => (),
            HelperPhase::LaunchIntent if result == HelperResult::NotStarted => (),
            _ => return Err(ContractError::InvalidTransition.into()),
        }
        record.phase = HelperPhase::Finished { result };
        self.write_helper(&record)
    }
}
fn name(id: Uuid) -> String {
    format!("helper-{id}.json")
}
#[cfg(test)]
mod tests;
