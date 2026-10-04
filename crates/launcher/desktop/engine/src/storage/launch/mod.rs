//! Launch uses the existing operation journal. A terminal success means the
//! observed game exited normally, never authenticated or entered the world.
use super::*;
use crate::{OperationKind, OperationState};
use sha2::{Digest, Sha256};
use uuid::Uuid;
mod preparation;
mod resource;
pub use resource::{Artifact, Graphics, Resources};
mod worker;
pub use worker::{dispatch, Worker};
mod supervisor;
#[cfg(target_os = "macos")]
mod wine;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub id: Uuid,
    pub installation: InstallIntent,
    pub runtime: Option<runtime_setup::Plan>,
    pub resources: Resources,
}
impl Plan {
    fn digest(&self) -> Result<[u8; 32], IntentError> {
        Ok(Sha256::digest(serde_json::to_vec(self).map_err(|_| StorageError::Corrupt)?).into())
    }
}
#[derive(Debug)]
pub struct Admission {
    pub plan: Plan,
    pub dispatch: bool,
}
/// Path-free native observations. Host PID and guest PID have different namespaces.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    Preparing,
    HostStarted {
        host_pid: u32,
    },
    ProcessStarted {
        host_pid: u32,
        guest_pid: u32,
    },
    ProcessExited {
        host_pid: u32,
        guest_pid: u32,
        code: i32,
        early: bool,
    },
    NotStarted,
    Cancelled,
    Unknown,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    id: Uuid,
    digest: [u8; 32],
    observation: Observation,
}
impl DesktopState {
    /// Native resources only. Caller uses the last inspected revision and an
    /// independently generated ID. An identical retry never starts twice.
    pub fn admit_launch(
        &mut self,
        id: Uuid,
        revision: u64,
        installation_id: Uuid,
        resources: Resources,
    ) -> Result<Admission, IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if id.is_nil() {
            return Err(ContractError::IdentityConflict.into());
        }
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| op.id == id)
        {
            let plan = self.launch_plan()?.ok_or(ContractError::IdentityConflict)?;
            if plan.installation.operation_id != installation_id || plan.resources != resources {
                return Err(ContractError::IdentityConflict.into());
            }
            return Ok(Admission {
                plan,
                dispatch: false,
            });
        }
        if revision != self.operations.snapshot().revision {
            return Err(ContractError::StaleRevision.into());
        }
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| !op.state.terminal())
        {
            return Err(ContractError::Busy.into());
        }
        resources.verify()?;
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
        let runtime = match installed.intent.backend {
            ExtractionBackend::Native => {
                if !cfg!(windows) {
                    return Err(StorageError::Corrupt.into());
                }
                None
            }
            ExtractionBackend::Wine { .. } => {
                if !cfg!(target_os = "macos") || resources.graphics.is_none() {
                    return Err(StorageError::Corrupt.into());
                }
                Some(self.prepared_runtime()?.ok_or(StorageError::Corrupt)?.plan)
            }
        };
        let plan = Plan {
            id,
            installation: installed.intent,
            runtime,
            resources,
        };
        let name = format!("launch-plan-{id}.json");
        if self
            .directory
            .root
            .join(&name)
            .try_exists()
            .map_err(|_| StorageError::Io)?
        {
            return Err(ContractError::IdentityConflict.into());
        }
        self.write_launch(&name, &plan)?;
        let (_, dispatch) =
            self.operations_mut()?
                .begin(id, OperationKind::Launch, plan.digest()?, revision)?;
        Ok(Admission { plan, dispatch })
    }
    pub fn launch_plan(&self) -> Result<Option<Plan>, IntentError> {
        let Some(operation) = self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .filter(|op| op.kind == OperationKind::Launch)
        else {
            return Ok(None);
        };
        let plan: Plan = read(
            &self
                .directory
                .root
                .join(format!("launch-plan-{}.json", operation.id)),
        )?
        .ok_or(StorageError::Corrupt)?;
        if plan.id != operation.id || plan.digest()? != operation.intent_digest {
            return Err(StorageError::Corrupt.into());
        }
        Ok(Some(plan))
    }
    pub fn launch_observation(&self) -> Result<Option<Observation>, IntentError> {
        let Some(plan) = self.launch_plan()? else {
            return Ok(None);
        };
        // A recorded process-started line never proves liveness after restart.
        if self.requires_reopen()
            || self
                .operations
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| op.state == OperationState::ReconciliationRequired)
        {
            return Ok(Some(Observation::Unknown));
        }
        let record: Option<Record> = read(
            &self
                .directory
                .root
                .join(format!("launch-result-{}.json", plan.id)),
        )?;
        record
            .map(|record| {
                if record.id != plan.id || record.digest != plan.digest()? {
                    return Err(StorageError::Corrupt.into());
                }
                Ok(record.observation)
            })
            .transpose()
    }
    fn record_launch(&mut self, plan: &Plan, observation: Observation) -> Result<(), IntentError> {
        if self.launch_plan()?.as_ref() != Some(plan) {
            return Err(ContractError::IdentityConflict.into());
        }
        self.write_launch(
            &format!("launch-result-{}.json", plan.id),
            &Record {
                id: plan.id,
                digest: plan.digest()?,
                observation,
            },
        )
    }
    fn write_launch<T: Serialize>(&mut self, name: &str, value: &T) -> Result<(), IntentError> {
        let result = atomic::write(&self.directory.root, name, value);
        self.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
        result.map_err(Into::into)
    }
}
#[cfg(test)]
mod tests;
