use super::*;
use crate::{helper_supervisor::Outcome, OperationState};
#[test]
fn repair_helper_identity_binds_work_plan_without_replacing_original_installation() {
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
        .unwrap()
        .plan;
    let work = state.extraction_work(plan.id).unwrap();
    assert_eq!(work.installation, installed);
    assert_ne!(work.operation_id, installed.operation_id);
    assert_eq!(work.stage, plan.stage());
    assert_eq!(work.cache, plan.work_directory().join("cache"));
    assert!(work.stage.starts_with(plan.work_directory()));
    assert_ne!(work.stage, installed.destination.join("game"));
    assert!(state.install_intent().unwrap().is_none());
    assert!(state.extraction_work(installed.operation_id).is_err());
    assert!(state.begin_helper(plan.id).is_err()); // admission is not dispatch
    state
        .operations_mut()
        .unwrap()
        .observe(plan.id, OperationState::Running)
        .unwrap();
    let record = state.begin_helper(plan.id).unwrap();
    assert_eq!(record.operation_id, work.operation_id);
    assert_eq!(record.intent_digest, work.intent_digest);
    assert_ne!(record.intent_digest, installed.digest().unwrap());
    assert!(state.begin_helper(plan.id).is_err());
    assert!(state
        .record_helper_host(installed.operation_id, record.attempt_id, 42)
        .is_err());
    state
        .record_helper_host(plan.id, record.attempt_id, 42)
        .unwrap();
    state
        .finish_helper(plan.id, record.attempt_id, Outcome::Completed)
        .unwrap();
    assert!(!root
        .path()
        .join(format!("state/helper-{}.json", installed.operation_id))
        .exists());
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(state.extraction_work(plan.id).unwrap(), work);
    assert_eq!(
        state.helper_record(plan.id).unwrap().unwrap().intent_digest,
        work.intent_digest
    );
    assert_eq!(
        state.installed_content().unwrap().unwrap().intent,
        installed
    );
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert!(state.begin_helper(plan.id).is_err());
}
#[tokio::test]
async fn native_repair_cannot_dispatch_a_wine_helper() {
    let (_root, state, installation, _server) = install_worker::tests::prepared_fixture().await;
    let mut state = state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_repair(Uuid::new_v4(), revision, installation, true)
        .unwrap()
        .plan;
    state
        .operations_mut()
        .unwrap()
        .observe(plan.id, OperationState::Running)
        .unwrap();
    assert!(state
        .extraction_work(plan.id)
        .unwrap()
        .installation
        .backend
        .is_native());
    assert!(state.begin_helper(plan.id).is_err());
}
#[test]
fn changed_repair_plan_cannot_rebind_an_existing_helper_attempt() {
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    let revision = state.operations().snapshot().revision;
    let mut plan = state
        .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
        .unwrap()
        .plan;
    state
        .operations_mut()
        .unwrap()
        .observe(plan.id, OperationState::Running)
        .unwrap();
    state.begin_helper(plan.id).unwrap();
    plan.installation.destination = root.path().join("foreign");
    std::fs::write(
        root.path()
            .join(format!("state/repair-plan-{}.json", plan.id)),
        serde_json::to_vec(&plan).unwrap(),
    )
    .unwrap();
    assert!(state.extraction_work(plan.id).is_err());
    assert!(state.helper_record(plan.id).is_err());
}
