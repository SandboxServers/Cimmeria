//! `dispatch_failed` (PD-02, C-PD2; PD-05, C-PD3): which pre-start dispatch
//! failures end `Failed`, and the `NotStarted` record they write first.
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
    let (failed, events) =
        capture(|| dispatch_failed(&state, plan.id, &error, FailureCode::StatusStale));
    assert!(failed);
    assert_eq!(state_of(&state), OperationState::Failed);
    let owner = state.lock().unwrap();
    assert_eq!(
        owner.launch_observation(),
        Ok(Some(Observation::NotStarted))
    );
    assert_eq!(owner.launch_failure(), Ok(Some(FailureCode::StatusStale)));
    let logged = not_started(&events);
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].level, Level::WARN);
    assert_eq!(logged[0].field("reason"), Some("unknown_operation"));
    assert_eq!(logged[0].field("failure_code"), Some("status_stale"));
    assert!(logged[0].field("operation").is_some());
}

#[test]
fn a_launch_that_already_recorded_preparing_is_left_alone() {
    let (_root, mut state, plan) = super::super::tests::fixture();
    state.record_launch(&plan, Observation::Preparing).unwrap();
    let state = Mutex::new(state);
    let error: IntentError = StorageError::Io.into();
    let (failed, events) =
        capture(|| dispatch_failed(&state, plan.id, &error, FailureCode::LocalIo));
    assert!(!failed);
    assert_eq!(state_of(&state), OperationState::Starting);
    let owner = state.lock().unwrap();
    assert_eq!(owner.launch_observation(), Ok(Some(Observation::Preparing)));
    assert_eq!(owner.launch_failure(), Ok(None));
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
            dispatch_failed(
                &state,
                plan.id,
                &uncertain,
                FailureCode::PersistenceUncertain,
            ),
            dispatch_failed(&state, Uuid::new_v4(), &other, FailureCode::LocalIo),
        ]
    });
    assert_eq!(failed, [false, false]);
    assert_eq!(state_of(&state), OperationState::Starting);
    assert_eq!(state.lock().unwrap().launch_observation(), Ok(None));
    assert!(not_started(&events).is_empty());
}
