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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::JobError;
    use crate::test_capture::{capture, Captured};

    #[test]
    fn action_row_names_the_action_and_the_outcome_code() {
        let ((), captured) = capture(|| {
            action_finished::<(), JobError>("apply", &Ok(()), Duration::from_millis(4));
            action_finished::<(), JobError>(
                "cancel",
                &Err(JobError::StaleRevision),
                Duration::from_millis(5),
            );
        });
        assert_action(&captured[0], "apply", "ok");
        assert_action(&captured[1], "cancel", "stale_revision");
        assert_eq!(captured[0].field("duration_ms"), Some("4"));
    }

    fn assert_action(row: &Captured, action: &str, outcome: &str) {
        assert_eq!(row.target, "launcher.game_update");
        assert_eq!(row.level, tracing::Level::INFO);
        assert_eq!(row.field("event"), Some("game_update_action_finished"));
        assert_eq!(row.field("action"), Some(action));
        assert_eq!(row.field("outcome"), Some(outcome));
    }

    #[test]
    fn mark_uncertain_row_is_a_warning_with_the_closed_code() {
        let ((), captured) = capture(|| mark_uncertain_failed(JobError::StaleRevision));
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].level, tracing::Level::WARN);
        assert_eq!(captured[0].field("event"), Some("mark_uncertain_failed"));
        assert_eq!(captured[0].field("error_kind"), Some("stale_revision"));
    }
}
