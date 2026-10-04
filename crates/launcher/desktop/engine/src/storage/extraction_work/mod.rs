//! Native-derived extraction identity. Work IDs never replace installation IDs.
use super::*;
use crate::OperationKind;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractionWork {
    pub operation_id: Uuid,
    pub intent_digest: [u8; 32],
    pub installation: InstallIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_release: Option<ReleaseIdentity>,
    pub stage: PathBuf,
    pub cache: PathBuf,
}
impl DesktopState {
    pub(crate) fn cached_extraction_release(
        &self,
        id: Uuid,
    ) -> Result<crate::catalog::VerifiedRelease, IntentError> {
        let work = self.extraction_work(id)?;
        self.verify_release_identity(
            work.current_release
                .unwrap_or_else(|| work.installation.release_identity()),
        )
        .map_err(|error| match error {
            EvidenceError::Storage(error) => IntentError::Storage(error),
            _ => IntentError::Storage(StorageError::Corrupt),
        })
    }

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
        let mut current_release = None;
        let (installation, stage, cache) = match operation.kind {
            OperationKind::Install => {
                let intent = self.install_intent()?.ok_or(StorageError::Corrupt)?;
                let stage = intent.destination.join(format!(".cimmeria-stage-{id}"));
                let cache = intent.destination.join(format!(".cimmeria-cache-{id}"));
                (intent, stage, cache)
            }
            OperationKind::Repair => {
                let plan = self.repair_plan()?.ok_or(StorageError::Corrupt)?;
                current_release = plan.current_release;
                let stage = plan.stage();
                let cache = plan.work_directory().join("cache");
                (plan.installation, stage, cache)
            }
            OperationKind::Adopt => {
                let plan = super::adoption::preparation::read_record(self, id)?
                    .ok_or(StorageError::Corrupt)?;
                if plan.digest().map_err(|_| StorageError::Corrupt)? != operation.intent_digest {
                    return Err(StorageError::Corrupt.into());
                }
                let stage = plan.stage();
                let cache = plan.cache();
                (plan.descriptor, stage, cache)
            }
            _ => return Err(ContractError::InvalidTransition.into()),
        };
        Ok(ExtractionWork {
            operation_id: id,
            intent_digest: operation.intent_digest,
            installation,
            current_release,
            stage,
            cache,
        })
    }
}
#[cfg(test)]
mod tests;
