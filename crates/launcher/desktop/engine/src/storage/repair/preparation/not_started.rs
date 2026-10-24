//! A repair whose dispatch failed before its worker started (D-PD2). The run
//! never touched disk, so it ends `Failed` with a logged reason instead of
//! waiting for a reconciliation it cannot need.
use super::*;

/// Ends repair `id` as `Failed` when it is still the current `Starting`
/// repair. Returns whether it did; on any other state or error, nothing
/// changes and the caller keeps its own fallback. `reason` is a closed code.
pub fn not_started(state: &Mutex<DesktopState>, id: Uuid, reason: &'static str) -> bool {
    let Ok(mut owner) = state.lock() else {
        return false;
    };
    let Ok(operations) = owner.operations_mut() else {
        return false;
    };
    let starting = operations.snapshot().operation.as_ref().is_some_and(|op| {
        op.id == id
            && op.kind == crate::OperationKind::Repair
            && op.state == OperationState::Starting
    });
    if !starting || operations.observe(id, OperationState::Failed).is_err() {
        return false;
    }
    events::repair_not_started(id, reason);
    owner.queue_install_result(id, Run::Repair, Ending::Failed(reason));
    true
}

#[cfg(test)]
#[path = "not_started_tests.rs"]
mod tests;
