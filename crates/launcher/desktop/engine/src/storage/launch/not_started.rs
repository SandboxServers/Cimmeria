//! A Play whose dispatch fails before the game starts ends `Failed` with a
//! logged reason (D-PD2). Only an unconfirmed save, an operation that already
//! left `Starting`, or one with a launch record stays for reconciliation.
use super::*;
use std::sync::Mutex;

/// Fails the launch `id` when its dispatch failed before anything was recorded:
/// the operation is a `Launch` still in `Starting` with no launch observation,
/// and `error` is not `persistence_uncertain`. Logs `launch_not_started` with
/// the error's code and returns `true`. Any other case, or any error on the way,
/// returns `false` and changes nothing, so the caller keeps its uncertain path.
pub fn dispatch_failed(state: &Mutex<DesktopState>, id: Uuid, error: &IntentError) -> bool {
    let reason = error.code();
    if reason == "persistence_uncertain" {
        return false;
    }
    let Ok(mut owner) = state.lock() else {
        return false;
    };
    let starting = owner
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .is_some_and(|op| {
            op.id == id && op.kind == OperationKind::Launch && op.state == OperationState::Starting
        });
    if !starting || !matches!(owner.launch_observation(), Ok(None)) {
        return false;
    }
    let Ok(operations) = owner.operations_mut() else {
        return false;
    };
    if operations.observe(id, OperationState::Failed).is_err() {
        return false;
    }
    events::launch_not_started(id, reason);
    true
}

#[cfg(test)]
#[path = "not_started_tests.rs"]
mod tests;
