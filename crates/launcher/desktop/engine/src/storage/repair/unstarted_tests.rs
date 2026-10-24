use super::*;
use std::sync::Mutex;

/// A repair admitted on the PD-01 fixture, then left uncertain before its
/// worker claimed anything: the state a lost dispatch leaves behind.
async fn stuck() -> (
    tempfile::TempDir,
    Arc<Mutex<DesktopState>>,
    Plan,
    wiremock::MockServer,
) {
    let (root, state, installation, server) = install_worker::tests::prepared_fixture().await;
    let plan = {
        let mut owner = state.lock().unwrap();
        let revision = owner.operations().snapshot().revision;
        let plan = owner
            .admit_repair(Uuid::new_v4(), revision, installation, true)
            .unwrap()
            .plan;
        owner
            .operations_mut()
            .unwrap()
            .mark_uncertain(plan.id)
            .unwrap();
        plan
    };
    (root, state, plan, server)
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

#[tokio::test]
async fn an_unstarted_repair_with_nothing_on_disk_ends_failed() {
    let (_root, state, plan, _server) = stuck().await;
    state.lock().unwrap().set_game_telemetry(true).unwrap();
    let (result, events) =
        crate::test_capture::capture(|| state.lock().unwrap().reconcile_unstarted_repair());
    assert_eq!(result.unwrap(), Some(plan.id));
    assert_eq!(current(&state), OperationState::Failed);
    let reconciled: Vec<_> = events
        .iter()
        .filter(|e| e.field("event") == Some("repair_reconciled_unstarted"))
        .collect();
    assert_eq!(reconciled.len(), 1);
    assert_eq!(reconciled[0].level, tracing::Level::INFO);
    assert_eq!(reconciled[0].target, "launcher.repair");
    assert_eq!(reconciled[0].field("outcome"), Some("failed"));
    assert!(reconciled[0].field("operation").is_some());
    let queued: Vec<_> = events
        .iter()
        .filter(|e| e.field("event") == Some("install_result_queued"))
        .collect();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].field("action"), Some("repair"));
}

#[tokio::test]
async fn a_work_directory_on_disk_still_needs_reconciliation() {
    let (_root, state, plan, _server) = stuck().await;
    std::fs::create_dir(plan.work_directory()).unwrap();
    let (result, events) =
        crate::test_capture::capture(|| state.lock().unwrap().reconcile_unstarted_repair());
    assert_eq!(result.unwrap(), None);
    assert_eq!(current(&state), OperationState::ReconciliationRequired);
    assert!(events
        .iter()
        .all(|e| e.field("event") != Some("repair_reconciled_unstarted")));
}

#[tokio::test]
async fn a_commit_record_still_needs_reconciliation() {
    let (_root, state, plan, _server) = stuck().await;
    let mut owner = state.lock().unwrap();
    let record = owner
        .directory
        .root
        .join(format!("repair-commit-{}.json", plan.id));
    std::fs::write(record, b"{}").unwrap();
    assert_eq!(owner.reconcile_unstarted_repair().unwrap(), None);
    drop(owner);
    assert_eq!(current(&state), OperationState::ReconciliationRequired);
}

#[tokio::test]
async fn an_uncertain_install_is_not_a_repair() {
    let (_root, state, _installation, _server) = install_worker::tests::prepared_fixture().await;
    let mut owner = state.lock().unwrap();
    let revision = owner.operations().snapshot().revision;
    let id = Uuid::new_v4();
    let operations = owner.operations_mut().unwrap();
    operations
        .begin(id, OperationKind::Install, [7; 32], revision)
        .unwrap();
    operations.mark_uncertain(id).unwrap();
    assert_eq!(owner.reconcile_unstarted_repair().unwrap(), None);
    drop(owner);
    assert_eq!(current(&state), OperationState::ReconciliationRequired);
}
