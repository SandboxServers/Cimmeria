//! Launch and Play lifecycle events (launcher-telemetry LT-17a; C5, C6). The
//! launch worker and supervisor log through these, one target and one shape
//! each. Logging never changes the outcome: every helper returns nothing.
use super::Observation;
use crate::operation_numbers;
use crate::ContractError;
use std::time::Duration;
use uuid::Uuid;

/// A launch request was refused before it was admitted; `reason` is its code.
pub(crate) fn launch_refused(id: Uuid, reason: &'static str) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.launch",
        event = "launch_refused",
        operation,
        reason,
        "launch: refused"
    );
}

/// The admitted launch began; `telemetry_dll` says whether the DLL goes in.
pub(crate) fn launch_started(id: Uuid, telemetry_dll: bool) {
    let operation = operation_numbers::number(id);
    tracing::info!(
        target: "launcher.launch",
        event = "launch_started",
        operation,
        telemetry_dll,
        "launch: started"
    );
}

/// The game did not start; the reason is its closed code.
pub(crate) fn launch_not_started(id: Uuid, reason: &'static str) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.launch",
        event = "launch_not_started",
        operation,
        reason,
        "launch: not started"
    );
}

/// The launch failed after it began; the reason is its closed code.
pub(crate) fn launch_failed(id: Uuid, reason: &'static str) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.launch",
        event = "launch_failed",
        operation,
        reason,
        "launch: failed"
    );
}

/// The game process is running.
pub(crate) fn game_started(id: Uuid) {
    let operation = operation_numbers::number(id);
    tracing::info!(
        target: "launcher.launch",
        event = "game_started",
        operation,
        "launch: game started"
    );
}

/// The game process exited. `early` marks an exit within ten seconds.
pub(crate) fn game_exited(id: Uuid, code: i32, early: bool, ran: Duration) {
    let operation = operation_numbers::number(id);
    let exit_code = i64::from(code);
    let duration_ms = u64::try_from(ran.as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        target: "launcher.launch",
        event = "game_exited",
        operation,
        exit_code,
        early,
        duration_ms,
        "launch: game exited"
    );
}

/// The launch reached its final observation.
pub(crate) fn launch_finished(id: Uuid, observation: Observation) {
    let operation = operation_numbers::number(id);
    let outcome = match observation {
        Observation::ProcessExited { .. } => "exited",
        Observation::NotStarted => "not_started",
        Observation::Cancelled => "cancelled",
        _ => "unknown",
    };
    let duration_ms = operation_numbers::elapsed_ms(id);
    tracing::info!(
        target: "launcher.launch",
        event = "launch_finished",
        operation,
        outcome,
        duration_ms,
        "launch: finished"
    );
}

/// The client-patches events could not be queued at game exit.
pub(crate) fn patch_events_queue_failed(id: Uuid) {
    let operation = operation_numbers::number(id);
    tracing::warn!(
        target: "launcher.launch",
        event = "patch_events_queue_failed",
        operation,
        "launch: client patches events not queued"
    );
}

/// The launch could not record its uncertain state; the shell calls this too.
pub fn mark_uncertain_failed(error: ContractError) {
    tracing::warn!(
        target: "launcher.launch",
        event = "mark_uncertain_failed",
        reason = error.code(),
        "launch: recording the uncertain state failed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::{capture, Captured};
    use tracing::Level;

    fn event_named<'a>(events: &'a [Captured], name: &str) -> Vec<&'a Captured> {
        events
            .iter()
            .filter(|e| e.target == "launcher.launch" && e.field("event") == Some(name))
            .collect()
    }

    #[test]
    fn launch_finished_maps_each_observation() {
        let cases = [
            (
                Observation::ProcessExited {
                    host_pid: 1,
                    guest_pid: 2,
                    code: 0,
                    early: false,
                },
                "exited",
            ),
            (Observation::NotStarted, "not_started"),
            (Observation::Cancelled, "cancelled"),
            (Observation::Unknown, "unknown"),
        ];
        for (observation, outcome) in cases {
            let id = Uuid::new_v4();
            let ((), events) = capture(|| launch_finished(id, observation));
            let finished = event_named(&events, "launch_finished");
            assert_eq!(finished.len(), 1);
            assert_eq!(finished[0].level, Level::INFO);
            assert_eq!(finished[0].field("outcome"), Some(outcome));
            assert!(finished[0].field("duration_ms").is_some());
        }
    }

    #[test]
    fn game_exited_carries_code_and_duration() {
        let id = Uuid::new_v4();
        let ((), events) = capture(|| game_exited(id, -1, true, Duration::from_millis(250)));
        let exited = event_named(&events, "game_exited");
        assert_eq!(exited.len(), 1);
        assert_eq!(exited[0].level, Level::INFO);
        assert_eq!(exited[0].field("exit_code"), Some("-1"));
        assert_eq!(exited[0].field("early"), Some("true"));
        assert_eq!(exited[0].field("duration_ms"), Some("250"));
    }

    #[test]
    fn refusals_carry_their_reason() {
        let id = Uuid::new_v4();
        let ((), events) = capture(|| launch_refused(id, "plan_exists"));
        let refused = event_named(&events, "launch_refused");
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].level, Level::WARN);
        assert_eq!(refused[0].field("reason"), Some("plan_exists"));
        assert!(refused[0].field("operation").is_some());
    }

    #[test]
    fn mark_uncertain_failed_carries_the_contract_code() {
        let ((), events) = capture(|| mark_uncertain_failed(ContractError::IdentityConflict));
        let failed: Vec<&Captured> = events
            .iter()
            .filter(|e| e.field("event") == Some("mark_uncertain_failed"))
            .collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].target, "launcher.launch");
        assert_eq!(failed[0].level, Level::WARN);
        assert_eq!(
            failed[0].field("reason"),
            Some(ContractError::IdentityConflict.code())
        );
    }
}
