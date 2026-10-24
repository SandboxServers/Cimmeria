//! `dispatch_failed` (PD-02, C-PD2): which pre-start dispatch failures end `Failed`.
use super::*;
use crate::test_capture::{capture, Captured};
use tracing::Level;

fn state_of(state: &Mutex<DesktopState>) -> OperationState {
    state
        .lock()
        .unwrap()
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .unwrap()
        .state
}

fn not_started(events: &[Captured]) -> Vec<&Captured> {
    events
        .iter()
        .filter(|e| e.target == "launcher.launch" && e.field("event") == Some("launch_not_started"))
        .collect()
}

#[test]
fn a_starting_launch_with_no_record_fails_and_logs_the_error_code() {
    let (_root, state, plan) = super::super::tests::fixture();
    let state = Mutex::new(state);
    let error: IntentError = ContractError::UnknownOperation.into();
    let (failed, events) = capture(|| dispatch_failed(&state, plan.id, &error));
    assert!(failed);
    assert_eq!(state_of(&state), OperationState::Failed);
    let logged = not_started(&events);
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].level, Level::WARN);
    assert_eq!(logged[0].field("reason"), Some("unknown_operation"));
    assert!(logged[0].field("operation").is_some());
}

#[test]
fn a_launch_that_already_recorded_preparing_is_left_alone() {
    let (_root, mut state, plan) = super::super::tests::fixture();
    state.record_launch(&plan, Observation::Preparing).unwrap();
    let state = Mutex::new(state);
    let error: IntentError = StorageError::Io.into();
    let (failed, events) = capture(|| dispatch_failed(&state, plan.id, &error));
    assert!(!failed);
    assert_eq!(state_of(&state), OperationState::Starting);
    assert!(not_started(&events).is_empty());
}

#[test]
fn an_unconfirmed_save_or_another_operation_is_left_alone() {
    let (_root, state, plan) = super::super::tests::fixture();
    let state = Mutex::new(state);
    let uncertain: IntentError = StorageError::PersistenceUncertain.into();
    let other: IntentError = StorageError::Io.into();
    let (failed, events) = capture(|| {
        [
            dispatch_failed(&state, plan.id, &uncertain),
            dispatch_failed(&state, Uuid::new_v4(), &other),
        ]
    });
    assert_eq!(failed, [false, false]);
    assert_eq!(state_of(&state), OperationState::Starting);
    assert!(not_started(&events).is_empty());
}
