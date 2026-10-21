//! Game-update shell events (launcher-telemetry LT-13; C5, C6). One info row per
//! apply or cancel request, and a warn row when an operation cannot be marked
//! uncertain. Only closed codes and counts are written (R5).
use serde::Serialize;
use std::time::Duration;

/// The serde code of `value` (snake_case), or `"other"` when it has none.
fn code<T: Serialize>(value: T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "other".to_owned(),
    }
}

/// A game-update request finished. `outcome` is `ok` or the closed code of its error.
pub(super) fn action_finished<T, E: Serialize>(
    action: &'static str,
    result: &Result<T, E>,
    elapsed: Duration,
) {
    let outcome = match result {
        Ok(_) => "ok".to_owned(),
        Err(error) => code(error),
    };
    let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        target: "launcher.game_update",
        event = "game_update_action_finished",
        action,
        outcome,
        duration_ms,
        "game update: the request finished; expected the operation to reach a recorded state; see the outcome"
    );
}

/// An operation could not be marked uncertain, so a restart must reconcile it.
pub(super) fn mark_uncertain_failed<E: Serialize>(error: E) {
    let error_kind = code(error);
    tracing::warn!(
        target: "launcher.game_update",
        event = "mark_uncertain_failed",
        error_kind,
        "game update: the operation could not be marked uncertain; expected a recovery record; the next start must reconcile it"
    );
}
