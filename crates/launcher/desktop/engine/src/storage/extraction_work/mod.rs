//! Native-derived extraction identity. Work IDs never replace installation IDs.
use super::*;
use crate::OperationKind;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtractionWork {
    pub operation_id: Uuid,
    pub intent_digest: [u8; 32],
    pub installation: InstallIntent,
    pub stage: PathBuf,
    pub cache: PathBuf,
}
impl DesktopState {
    /// Reverify the current durable plan; never accept work paths from the UI.
    /// This describes identity only, not admission, launch permission or quiescence.
    pub(crate) fn extraction_work(&self, id: Uuid) -> Result<ExtractionWork, IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let operation = self
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .ok_or(ContractError::UnknownOperation)?;
        if operation.id != id {
            return Err(ContractError::UnknownOperation.into());
        }
        let (installation, stage, cache) = match operation.kind {
            OperationKind::Install => {
                let intent = self.install_intent()?.ok_or(StorageError::Corrupt)?;
                let stage = intent.destination.join(format!(".cimmeria-stage-{id}"));
                let cache = intent.destination.join(format!(".cimmeria-cache-{id}"));
                (intent, stage, cache)
            }
            OperationKind::Repair => {
                let plan = self.repair_plan()?.ok_or(StorageError::Corrupt)?;
                let stage = plan.stage();
                let cache = plan.work_directory().join("cache");
                (plan.installation, stage, cache)
            }
            _ => return Err(ContractError::InvalidTransition.into()),
        };
        Ok(ExtractionWork {
            operation_id: id,
            intent_digest: operation.intent_digest,
            installation,
            stage,
            cache,
        })
    }
}
#[cfg(test)]
mod tests;
