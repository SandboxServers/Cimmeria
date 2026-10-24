use super::*;

fn admit(state: &Mutex<DesktopState>, installation: Uuid) -> Uuid {
    let mut owner = state.lock().unwrap();
    let revision = owner.operations().snapshot().revision;
    owner
        .admit_repair(Uuid::new_v4(), revision, installation, true)
        .unwrap()
        .plan
        .id
}

fn current(state: &Mutex<DesktopState>) -> OperationState {
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

fn repair_events(events: &[crate::test_capture::Captured]) -> Vec<&crate::test_capture::Captured> {
    events
        .iter()
        .filter(|e| e.field("event") == Some("repair_not_started"))
        .collect()
}

#[tokio::test]
async fn a_starting_repair_ends_failed_with_its_reason() {
    let (_root, state, installation, _server) = install_worker::tests::prepared_fixture().await;
    let id = admit(&state, installation);
    assert_eq!(current(&state), OperationState::Starting);
    let (ended, events) = crate::test_capture::capture(|| not_started(&state, id, "corrupt_state"));
    assert!(ended);
    assert_eq!(current(&state), OperationState::Failed);
    let events = repair_events(&events);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, tracing::Level::WARN);
    assert_eq!(events[0].target, "launcher.repair");
    assert_eq!(events[0].field("reason"), Some("corrupt_state"));
    assert_eq!(events[0].field("failure_code"), Some("repair_not_started"));
    assert!(events[0].field("operation").is_some());
}

#[tokio::test]
async fn a_running_repair_is_left_alone() {
    let (_root, state, installation, _server) = install_worker::tests::prepared_fixture().await;
    let id = admit(&state, installation);
    state
        .lock()
        .unwrap()
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    let (ended, events) = crate::test_capture::capture(|| not_started(&state, id, "corrupt_state"));
    assert!(!ended);
    assert_eq!(current(&state), OperationState::Running);
    assert!(repair_events(&events).is_empty());
}

#[tokio::test]
async fn another_operation_id_is_left_alone() {
    let (_root, state, installation, _server) = install_worker::tests::prepared_fixture().await;
    admit(&state, installation);
    assert!(!not_started(&state, Uuid::new_v4(), "io"));
    assert_eq!(current(&state), OperationState::Starting);
}
