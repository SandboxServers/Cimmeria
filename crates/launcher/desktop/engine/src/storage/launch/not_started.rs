//! A Play whose dispatch fails before the game starts ends `Failed` with a
//! logged reason (D-PD2). It first writes a `NotStarted` launch result carrying
//! the failure code (C-PD3), so the launch reads back as not started and the
//! summary reports `launch_not_started`. An unconfirmed save, an operation that
//! already left `Starting`, one with a launch record, or one whose launch state
//! cannot be read stays for reconciliation.
use super::*;
use crate::failure::FailureCode;
use std::sync::Mutex;

/// Fails the launch `id` when its dispatch failed before anything was recorded:
/// the operation is a `Launch` still in `Starting` with no launch observation,
/// and `error` is not `persistence_uncertain`. Writes a `NotStarted` record with
/// `failure`, then observes `Failed` (the worker's record-then-observe order),
/// logs `launch_not_started` with the error's code and `failure`, and returns
/// `true`. Any other case, or any error on the way, returns `false` and leaves
/// the operation as it was, so the caller keeps its uncertain path.
pub fn dispatch_failed(
    state: &Mutex<DesktopState>,
    id: Uuid,
    error: &IntentError,
    failure: FailureCode,
) -> bool {
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
    let Ok(Some(plan)) = owner.launch_plan() else {
        return false;
    };
    if owner.record_not_started(&plan, failure).is_err() {
        return false;
    }
    let Ok(operations) = owner.operations_mut() else {
        return false;
    };
    if operations.observe(id, OperationState::Failed).is_err() {
        return false;
    }
    events::launch_not_started(id, reason, failure);
    true
}

#[cfg(test)]
#[path = "not_started_tests.rs"]
mod tests;
