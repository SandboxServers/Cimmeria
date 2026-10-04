//! Retained native prerequisite task. The game prefix is separate from extraction
//! and remains headless during setup; a successful result never enables Play.
use super::*;
use crate::prerequisites::supervisor;
use crate::{runtime_setup::Plan, ContractError, IntentError, OperationState, StorageError};
use cimmeria_runtime_probe::prerequisite::PrepareRequest;
use futures_util::FutureExt;
use std::panic::AssertUnwindSafe;
use tokio::sync::watch;
mod prefix;
mod recovery;
pub use recovery::reconcile;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    PrerequisitesVerified,
    Failed,
    Cancelled,
    ReconciliationRequired,
}
pub struct Worker {
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    cancel: CancellationToken,
    pub result: watch::Receiver<Option<Outcome>>,
}
impl Worker {
    pub fn request_cancel(&self) -> Result<(), IntentError> {
        let mut state = self.state.lock().map_err(|_| StorageError::Io)?;
        state.operations_mut()?.request_cancel(self.id)?;
        self.cancel.cancel();
        Ok(())
    }
}
/// Caller admits a plan using an independently trusted bundled helper hash.
/// No webview path/environment is accepted here. A second dispatch is refused.
pub fn dispatch(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    helper: PathBuf,
) -> Result<Worker, IntentError> {
    let handle = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (plan, root) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let plan = owner
            .runtime_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        if plan.id != id {
            return Err(ContractError::IdentityConflict.into());
        }
        verify_file(&helper, &plan.helper_sha256).map_err(|_| StorageError::UnsafeFile)?;
        (
            owner.begin_runtime_dispatch(id)?,
            owner.state_root().to_path_buf(),
        )
    };
    let cancel = CancellationToken::new();
    let owned_cancel = cancel.clone();
    let owner = state.clone();
    let (result, observed) = watch::channel(None);
    handle.spawn(async move {
        let attempt = AssertUnwindSafe(run(&owner, plan, root, helper, owned_cancel))
            .catch_unwind()
            .await;
        let outcome = match attempt {
            Ok(Ok(outcome)) => outcome,
            _ => Outcome::ReconciliationRequired,
        };
        if outcome == Outcome::ReconciliationRequired {
            if let Ok(mut owner) = owner.lock() {
                if let Ok(operations) = owner.operations_mut() {
                    let _ = operations.mark_uncertain(id);
                }
            }
        }
        result.send_replace(Some(outcome));
    });
    Ok(Worker {
        state,
        id,
        cancel,
        result: observed,
    })
}
async fn run(
    state: &Arc<Mutex<DesktopState>>,
    plan: Plan,
    root: PathBuf,
    helper: PathBuf,
    cancel: CancellationToken,
) -> Result<Outcome, IntentError> {
    let candidate = plan.clone();
    let cache_root = root.clone();
    let executable = helper.clone();
    let claim = tokio::task::spawn_blocking(move || {
        prefix::Resources::claim(&candidate, &cache_root, &executable)
    })
    .await;
    let resources = match claim {
        Ok(Ok(resources)) => resources,
        // No Wine process has been started by resource validation/claim.
        _ => return finish_not_dispatched(state, plan.id, cancel.is_cancelled()),
    };
    let env =
        environment(&resources.runtime, &resources.prefix).map_err(|_| StorageError::UnsafeFile)?;
    let game = plan.installation.destination.join("game");
    let binaries = crate::install_layout::sgw_exe(&game)
        .parent()
        .ok_or(StorageError::Corrupt)?
        .to_path_buf();
    let package = game.join(".cimmeria-prerequisites/PhysX/PhysX_7.11.13_SystemSoftware.exe");
    let request = PrepareRequest {
        schema_version: 1,
        operation_id: plan.id,
        prefix_generation: plan.prefix_generation,
        game_binaries: paths::guest(&binaries)
            .map_err(|_| StorageError::UnsafeFile)?
            .into(),
        package: paths::guest(&package)
            .map_err(|_| StorageError::UnsafeFile)?
            .into(),
        scratch: paths::guest(&resources.root.join("work"))
            .map_err(|_| StorageError::UnsafeFile)?
            .into(),
    };
    let spec = HelperCommand {
        executable: resources.runtime.join("bin/wine"),
        arguments: vec![paths::guest(&helper)
            .map_err(|_| StorageError::UnsafeFile)?
            .into()],
        directory: resources.root.clone(),
        environment: env.clone(),
    };
    let outcome = supervisor::run(
        spec,
        request,
        cancel.clone(),
        Deadlines {
            operation: std::time::Duration::from_secs(180),
            ..Deadlines::default()
        },
        |pid| {
            state
                .lock()
                .map_err(|_| ())?
                .record_runtime_host(plan.id, pid)
                .map_err(|_| ())
        },
    )
    .await;
    // Always stop the verified exclusively held prefix, including uncertain host
    // results. That uncertainty remains gated even if stop/wait succeeds.
    let not_spawned = matches!(
        &outcome,
        supervisor::Outcome::NotStarted(
            helper_supervisor::Fault::Cancelled
                | helper_supervisor::Fault::InvalidRequest
                | helper_supervisor::Fault::Spawn
        )
    );
    let observed = match outcome {
        supervisor::Outcome::Observed(result) => Some((|| {
            state
                .lock()
                .map_err(|_| StorageError::Io)?
                .record_runtime_observation(plan.id, result)
        })()),
        _ => None,
    };
    stop_prefix(&resources.runtime, &env)
        .await
        .map_err(|_| StorageError::Io)?;
    if let Some(observed) = observed {
        observed?;
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        owner.finish_runtime_after_stop(plan.id)?;
        let succeeded = owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| op.state == OperationState::Succeeded);
        return Ok(if succeeded {
            Outcome::PrerequisitesVerified
        } else {
            Outcome::Failed
        });
    }
    if not_spawned {
        finish_not_dispatched(state, plan.id, cancel.is_cancelled())
    } else {
        Ok(Outcome::ReconciliationRequired)
    }
}
fn finish_not_dispatched(
    state: &Mutex<DesktopState>,
    id: Uuid,
    cancelled: bool,
) -> Result<Outcome, IntentError> {
    let mut owner = state.lock().map_err(|_| StorageError::Io)?;
    let terminal = if cancelled {
        OperationState::Cancelled
    } else {
        OperationState::Failed
    };
    owner.operations_mut()?.observe(id, terminal)?;
    Ok(if cancelled {
        Outcome::Cancelled
    } else {
        Outcome::Failed
    })
}
#[cfg(test)]
mod tests;
