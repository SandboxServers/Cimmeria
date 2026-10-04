//! A result describes one confirmed journal revision; it never proves completion.
use super::*;
use crate::{OperationKind, OperationState};
use install_worker::Outcome;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    operation_id: Uuid,
    intent_digest: [u8; 32],
    terminal_revision: u64,
    outcome: Outcome,
}

fn terminal(outcome: Outcome) -> Option<OperationState> {
    match outcome {
        Outcome::ContentPrepared => Some(OperationState::Succeeded),
        Outcome::Cancelled => Some(OperationState::Cancelled),
        Outcome::ReconciliationRequired => None,
        _ => Some(OperationState::Failed),
    }
}

impl DesktopState {
    /// Read only a result bound to the currently confirmed terminal commit.
    /// Legacy journals and later reconciliation commits may have no result.
    pub fn install_outcome(&self) -> Result<Option<Outcome>, StorageError> {
        if self.requires_reopen() {
            return Ok(None);
        }
        let snapshot = self.operations.snapshot();
        let Some(operation) = snapshot.operation.as_ref() else {
            return Ok(None);
        };
        if operation.kind != OperationKind::Install || !operation.state.terminal() {
            return Ok(None);
        }
        let Some(record): Option<Record> = read(&self.directory.root.join("install-result.json"))?
        else {
            return Ok(None);
        };
        if record.schema_version != 1 {
            return Err(StorageError::UnsupportedSchema);
        }
        // A previous operation/reconciliation result is historical, not current.
        if record.operation_id != operation.id || record.terminal_revision != snapshot.revision {
            return Ok(None);
        }
        if record.intent_digest != operation.intent_digest
            || terminal(record.outcome) != Some(operation.state)
        {
            return Err(StorageError::Corrupt);
        }
        Ok(Some(record.outcome))
    }

    /// Write before the terminal journal commit. A crash between the writes
    /// restores the active journal to recovery and cannot expose this as success.
    pub(super) fn prepare_install_result(
        &self,
        id: Uuid,
        outcome: Outcome,
    ) -> Result<(), StorageError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        let snapshot = self.operations.snapshot();
        let operation = snapshot.operation.as_ref().ok_or(StorageError::Corrupt)?;
        if operation.id != id
            || operation.kind != OperationKind::Install
            || operation.state.terminal()
            || terminal(outcome).is_none()
        {
            return Err(StorageError::Corrupt);
        }
        atomic::write(
            &self.directory.root,
            "install-result.json",
            &Record {
                schema_version: 1,
                operation_id: id,
                intent_digest: operation.intent_digest,
                terminal_revision: snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(StorageError::Corrupt)?,
                outcome,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn running(root: &Path) -> (DesktopState, Uuid) {
        let mut state = DesktopState::open(root).unwrap();
        let id = Uuid::new_v4();
        state
            .operations_mut()
            .unwrap()
            .begin(id, OperationKind::Install, [1; 32], 0)
            .unwrap();
        state
            .operations_mut()
            .unwrap()
            .observe(id, OperationState::Running)
            .unwrap();
        (state, id)
    }

    #[test]
    fn terminal_results_survive_restart_without_worker() {
        for outcome in [
            Outcome::ContentPrepared,
            Outcome::Cancelled,
            Outcome::DestinationUnavailable,
            Outcome::InstallFailed,
            Outcome::ContentInvalid,
            Outcome::RosettaRequired,
            Outcome::RuntimeUnavailable,
        ] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, id) = running(root.path());
            if outcome == Outcome::Cancelled {
                state.operations_mut().unwrap().request_cancel(id).unwrap();
            }
            state.prepare_install_result(id, outcome).unwrap();
            assert_eq!(state.install_outcome(), Ok(None));
            state
                .operations_mut()
                .unwrap()
                .observe(id, terminal(outcome).unwrap())
                .unwrap();
            drop(state);
            let state = DesktopState::open(root.path()).unwrap();
            assert_eq!(state.install_outcome(), Ok(Some(outcome)));
        }
    }

    #[test]
    fn crash_before_terminal_commit_does_not_claim_success_or_reuse_result() {
        let root = tempfile::tempdir().unwrap();
        let (state, id) = running(root.path());
        state
            .prepare_install_result(id, Outcome::ContentPrepared)
            .unwrap();
        drop(state);
        let mut state = DesktopState::open(root.path()).unwrap();
        assert_eq!(state.install_outcome(), Ok(None));
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
        state
            .operations_mut()
            .unwrap()
            .reconcile(id, OperationState::Failed)
            .unwrap();
        assert_eq!(state.install_outcome(), Ok(None));
    }

    #[test]
    fn missing_legacy_record_is_allowed_but_matching_corruption_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let (mut state, id) = running(root.path());
        state
            .prepare_install_result(id, Outcome::RosettaRequired)
            .unwrap();
        state
            .operations_mut()
            .unwrap()
            .observe(id, OperationState::Failed)
            .unwrap();
        let path = root.path().join("install-result.json");
        let mut record: Record = read(&path).unwrap().unwrap();
        record.intent_digest = [2; 32];
        atomic::write(root.path(), "install-result.json", &record).unwrap();
        assert_eq!(state.install_outcome(), Err(StorageError::Corrupt));
        record.schema_version = 2;
        atomic::write(root.path(), "install-result.json", &record).unwrap();
        assert_eq!(
            state.install_outcome(),
            Err(StorageError::UnsupportedSchema)
        );
        std::fs::write(&path, b"{").unwrap();
        assert_eq!(state.install_outcome(), Err(StorageError::Corrupt));
        std::fs::remove_file(path).unwrap();
        assert_eq!(state.install_outcome(), Ok(None));
    }
}
