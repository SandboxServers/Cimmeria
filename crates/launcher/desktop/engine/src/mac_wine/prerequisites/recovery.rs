//! Explicit reconciliation of observed results only. Unknown helper loss remains
//! gated until separate crash/descendant validation justifies a wider policy.
use super::*;
use crate::runtime_setup::{Phase, Record};

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
}
