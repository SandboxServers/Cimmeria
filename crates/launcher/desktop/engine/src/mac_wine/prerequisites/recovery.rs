//! Explicit reconciliation of observed results only. Unknown helper loss remains
//! gated until separate crash/descendant validation justifies a wider policy.
use super::*;
use crate::runtime_setup::{Phase, Record};

/// Advisory availability only. Reconciliation repeats every check under owned
/// resource locks before stopping a prefix or committing a terminal result.
pub fn can_reconcile(state: &DesktopState) -> bool {
    let snapshot = state.operations().snapshot();
    let Some(operation) = &snapshot.operation else {
        return false;
    };
    current(state, operation.id, snapshot.revision).is_ok()
        && state
            .runtime_record()
            .ok()
            .flatten()
            .is_some_and(|record| observed_host_absent(&record).is_ok())
}

pub async fn reconcile(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<Outcome, IntentError> {
    let (plan, root) = {
        let owner = state.lock().map_err(|_| StorageError::Io)?;
        let plan = current(&owner, id, revision)?;
        observed_host_absent(&owner.runtime_record()?.ok_or(StorageError::Corrupt)?)?;
        (plan, owner.state_root().to_path_buf())
    };
    let resources = tokio::task::spawn_blocking(move || prefix::Resources::reopen(&plan, &root))
        .await
        .map_err(|_| StorageError::Io)??;
    {
        // Another explicit recovery could have completed while locks were acquired.
        let owner = state.lock().map_err(|_| StorageError::Io)?;
        current(&owner, id, revision)?;
        observed_host_absent(&owner.runtime_record()?.ok_or(StorageError::Corrupt)?)?;
    }
    let env =
        environment(&resources.runtime, &resources.prefix).map_err(|_| StorageError::Corrupt)?;
    stop_prefix(&resources.runtime, &env)
        .await
        .map_err(|_| StorageError::Io)?;
    let mut owner = state.lock().map_err(|_| StorageError::Io)?;
    current(&owner, id, revision)?;
    owner.finish_runtime_after_stop(id)?;
    Ok(
        if owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state
            == OperationState::Succeeded
        {
            Outcome::PrerequisitesVerified
        } else {
            Outcome::Failed
        },
    )
}
fn current(state: &DesktopState, id: Uuid, revision: u64) -> Result<Plan, IntentError> {
    let plan = state
        .runtime_plan()?
        .ok_or(ContractError::UnknownOperation)?;
    let snapshot = state.operations().snapshot();
    if snapshot.revision != revision {
        return Err(ContractError::StaleRevision.into());
    }
    if plan.id != id {
        return Err(ContractError::IdentityConflict.into());
    }
    if snapshot.operation.as_ref().unwrap().state != OperationState::ReconciliationRequired {
        return Err(ContractError::InvalidTransition.into());
    }
    Ok(plan)
}
fn observed_host_absent(record: &Record) -> Result<(), StorageError> {
    if !matches!(record.phase, Phase::Observed | Phase::Quiescent) {
        return Err(StorageError::InUse);
    }
    let pid = record
        .host_pid
        .filter(|pid| *pid > 0 && *pid <= i32::MAX as u32)
        .ok_or(StorageError::Corrupt)?;
    let status = unsafe { libc::kill(pid as i32, 0) };
    if status != -1 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
        return Err(StorageError::InUse);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_reused_and_unobserved_host_identity_never_authorizes_stop() {
        let mut value = serde_json::json!({"schema_version":1,"operation_id":Uuid::new_v4(),
            "plan_digest":vec![0;32],"phase":"observed","host_pid":std::process::id(),"result":null});
        let record: Record = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(observed_host_absent(&record), Err(StorageError::InUse));
        value["phase"] = "host_started".into();
        value["host_pid"] = 2147483647_u32.into();
        let record: Record = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(observed_host_absent(&record), Err(StorageError::InUse));
        value["phase"] = "observed".into();
        value["host_pid"] = u32::MAX.into();
        assert_eq!(
            observed_host_absent(&serde_json::from_value(value).unwrap()),
            Err(StorageError::Corrupt)
        );
    }
    #[test]
    fn availability_requires_recovery_observation_and_absent_host() {
        use cimmeria_runtime_probe::prerequisite::{Failure, PrepareResult, ResultKind};
        for live in [false, true] {
            let (_root, mut state, installed) =
                crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
            let id = Uuid::new_v4();
            let plan = state
                .admit_runtime_setup(
                    id,
                    state.operations().snapshot().revision,
                    installed.operation_id,
                    [7; 32],
                    [9; 32],
                )
                .unwrap()
                .plan;
            assert!(!can_reconcile(&state));
            state.begin_runtime_dispatch(id).unwrap();
            let pid = if live {
                std::process::id()
            } else {
                let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
                let pid = child.id();
                child.wait().unwrap();
                pid
            };
            state.record_runtime_host(id, pid).unwrap();
            assert!(!can_reconcile(&state));
            state
                .record_runtime_observation(
                    id,
                    PrepareResult {
                        schema_version: 1,
                        operation_id: id,
                        prefix_generation: plan.prefix_generation,
                        result: ResultKind::Failed {
                            reason: Failure::Probe,
                        },
                    },
                )
                .unwrap();
            assert!(!can_reconcile(&state));
            state.operations_mut().unwrap().mark_uncertain(id).unwrap();
            assert_eq!(can_reconcile(&state), !live);
            // Availability is not successful execution or permission to replay.
            assert!(!plan.prefix_directory(state.state_root()).exists());
        }
    }
}
