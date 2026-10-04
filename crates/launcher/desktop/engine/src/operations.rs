use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;
// Revisions cross a JavaScript boundary; never exceed its exact integer range.
const MAX_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Adopt,
    Install,
    PrepareRuntime,
    Repair,
    Uninstall,
    Launch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Starting,
    Running,
    CancelRequested,
    Succeeded,
    Failed,
    Cancelled,
    /// No inference about whether a mutation or guest process finished.
    ReconciliationRequired,
}

impl OperationState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: Uuid,
    pub kind: OperationKind,
    /// Digest of the canonical native-validated intent, including configuration.
    pub intent_digest: [u8; 32],
    pub state: OperationState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub operation: Option<Operation>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            revision: 0,
            operation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractError {
    UnsupportedSchema,
    InvalidRevision,
    StaleRevision,
    Busy,
    IdentityConflict,
    UnknownOperation,
    InvalidTransition,
    PersistenceFailed,
    PersistenceUncertain,
}

/// A store atomically replaces a journal. A failure before replacement returns
/// PersistenceFailed; uncertain durability after replacement must return
/// PersistenceUncertain, forcing reopen before any further commands.
/// Commit must not return success before the snapshot is durable. Callers must
/// hold exclusive process ownership for the journal's lifetime.
pub trait Journal {
    fn commit(&mut self, snapshot: &Snapshot) -> Result<(), ContractError>;
}

/// Serialized by the native command adapter, never independently by each view.
/// All changes are committed before publication or dispatch to a mutation worker.
pub struct Operations<J> {
    snapshot: Snapshot,
    journal: J,
    requires_reopen: bool,
}

impl<J: Journal> Operations<J> {
    /// Opening a journal never resumes a mutation. A platform-specific inspector
    /// must reconcile an interrupted operation before another can be admitted.
    pub fn restore(snapshot: Snapshot, journal: J) -> Result<Self, ContractError> {
        if snapshot.schema_version != SCHEMA_VERSION {
            return Err(ContractError::UnsupportedSchema);
        }
        if snapshot.revision > MAX_REVISION {
            return Err(ContractError::InvalidRevision);
        }
        let mut controller = Self {
            snapshot,
            journal,
            requires_reopen: false,
        };
        if controller.snapshot.operation.as_ref().is_some_and(|op| {
            !op.state.terminal() && op.state != OperationState::ReconciliationRequired
        }) {
            let mut next = controller.snapshot.clone();
            next.operation.as_mut().unwrap().state = OperationState::ReconciliationRequired;
            controller.publish(next)?;
        }
        Ok(controller)
    }

    pub fn requires_reopen(&self) -> bool {
        self.requires_reopen
    }

    /// Last confirmed snapshot; adapters must also surface requires_reopen.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// Returns (snapshot, admitted). A retry of the current identical ID never
    /// dispatches work twice. Older requests are rejected by expected_revision,
    /// even after their operation has been replaced by a newer one.
    pub fn begin(
        &mut self,
        id: Uuid,
        kind: OperationKind,
        intent_digest: [u8; 32],
        expected_revision: u64,
    ) -> Result<(Snapshot, bool), ContractError> {
        self.ensure_writable()?;
        if let Some(op) = &self.snapshot.operation {
            if op.id == id {
                return if op.kind == kind && op.intent_digest == intent_digest {
                    Ok((self.snapshot.clone(), false))
                } else {
                    Err(ContractError::IdentityConflict)
                };
            }
        }
        if expected_revision != self.snapshot.revision {
            return Err(ContractError::StaleRevision);
        }
        if self
            .snapshot
            .operation
            .as_ref()
            .is_some_and(|op| !op.state.terminal())
        {
            return Err(ContractError::Busy);
        }
        let mut next = self.snapshot.clone();
        next.operation = Some(Operation {
            id,
            kind,
            intent_digest,
            state: OperationState::Starting,
        });
        self.publish(next)?;
        Ok((self.snapshot.clone(), true))
    }

    pub fn request_cancel(&mut self, id: Uuid) -> Result<Snapshot, ContractError> {
        self.ensure_writable()?;
        let op = self.current(id)?;
        if op.state.terminal() || op.state == OperationState::CancelRequested {
            return Ok(self.snapshot.clone());
        }
        if op.state == OperationState::ReconciliationRequired {
            return Err(ContractError::InvalidTransition);
        }
        self.transition(id, OperationState::CancelRequested)
    }

    /// Native worker observations only; never expose this as a webview command.
    pub fn observe(&mut self, id: Uuid, state: OperationState) -> Result<Snapshot, ContractError> {
        self.ensure_writable()?;
        let previous = self.current(id)?.state;
        let allowed = matches!(
            (previous, state),
            (OperationState::Starting, OperationState::Running)
                | (
                    OperationState::Starting
                        | OperationState::Running
                        | OperationState::CancelRequested,
                    OperationState::Succeeded | OperationState::Failed
                )
                | (OperationState::CancelRequested, OperationState::Cancelled)
        );
        if !allowed {
            return Err(ContractError::InvalidTransition);
        }
        self.transition(id, state)
    }

    /// A native worker lost authoritative completion. Retain ownership until
    /// explicit filesystem/process reconciliation, including without a restart.
    pub fn mark_uncertain(&mut self, id: Uuid) -> Result<Snapshot, ContractError> {
        self.ensure_writable()?;
        let current = self.current(id)?;
        if current.state.terminal() {
            return Err(ContractError::InvalidTransition);
        }
        if current.state == OperationState::ReconciliationRequired {
            return Ok(self.snapshot.clone());
        }
        self.transition(id, OperationState::ReconciliationRequired)
    }

    /// Only an authoritative filesystem/process inspection can call this.
    pub fn reconcile(
        &mut self,
        id: Uuid,
        state: OperationState,
    ) -> Result<Snapshot, ContractError> {
        self.ensure_writable()?;
        if self.current(id)?.state != OperationState::ReconciliationRequired
            || !(state.terminal() || state == OperationState::Running)
        {
            return Err(ContractError::InvalidTransition);
        }
        self.transition(id, state)
    }

    fn current(&self, id: Uuid) -> Result<&Operation, ContractError> {
        self.snapshot
            .operation
            .as_ref()
            .filter(|op| op.id == id)
            .ok_or(ContractError::UnknownOperation)
    }

    fn transition(&mut self, id: Uuid, state: OperationState) -> Result<Snapshot, ContractError> {
        self.current(id)?;
        let mut next = self.snapshot.clone();
        next.operation.as_mut().unwrap().state = state;
        self.publish(next)?;
        Ok(self.snapshot.clone())
    }

    fn ensure_writable(&self) -> Result<(), ContractError> {
        if self.requires_reopen {
            Err(ContractError::PersistenceUncertain)
        } else {
            Ok(())
        }
    }

    fn publish(&mut self, mut next: Snapshot) -> Result<(), ContractError> {
        next.revision = self
            .snapshot
            .revision
            .checked_add(1)
            .filter(|revision| *revision <= MAX_REVISION)
            .ok_or(ContractError::InvalidRevision)?;
        if let Err(error) = self.journal.commit(&next) {
            if error == ContractError::PersistenceUncertain {
                self.requires_reopen = true;
            }
            return Err(error);
        }
        self.snapshot = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
