//! Connect supervisor dispatch to durable native ownership checkpoints.
use super::*;
use crate::{ContractError, DesktopState, IntentError, StorageError};
use std::sync::{Arc, Mutex};

/// Native platform adapters supply already-validated executable/environment and
/// host-to-guest paths. This records admission before spawn, host identity before
/// the request, and the observed result before returning it. Journal errors must
/// be mapped to reconciliation by the caller, never ordinary installation failure.
/// Keep this future in the detached native worker scope, not a webview scope.
pub async fn run_owned(
    state: Arc<Mutex<DesktopState>>,
    command: HelperCommand,
    request: ExtractRequest,
    cancel: CancellationToken,
    progress: watch::Sender<Option<Progress>>,
    limits: Deadlines,
) -> Result<Outcome, IntentError> {
    let id = request.operation_id;
    let record = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let release = owner
            .cached_install_release()
            .map_err(|error| match error {
                crate::EvidenceError::Storage(error) => IntentError::Storage(error),
                _ => IntentError::Storage(StorageError::Corrupt),
            })?;
        if request.schema_version != 1 || request.sha256 != release.manifest().seed.sha256 {
            return Err(ContractError::IdentityConflict.into());
        }
        owner.begin_helper(id)?
    };
    let callback_state = state.clone();
    let outcome = run(command, request, cancel, progress, limits, move |pid| {
        callback_state
            .lock()
            .map_err(|_| ())?
            .record_helper_host(id, record.attempt_id, pid)
            .map_err(|_| ())
    })
    .await;
    state
        .lock()
        .map_err(|_| StorageError::Io)?
        .finish_helper(id, record.attempt_id, outcome)?;
    Ok(outcome)
}
