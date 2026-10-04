use super::*;
use crate::OperationState;
fn fixture() -> (tempfile::TempDir, DesktopState, ExtractionWork) {
    let runtime: Vec<u8> = mac_runtime::ARCHIVE_SHA256
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect();
    let (root, mut state, installed) =
        crate::runtime_setup::tests::fixture_with_runtime(runtime.try_into().unwrap());
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
        .unwrap()
        .plan;
    state
        .operations_mut()
        .unwrap()
        .observe(plan.id, OperationState::Running)
        .unwrap();
    let work = state.extraction_work(plan.id).unwrap();
    (root, state, work)
}
#[test]
fn no_dispatch_allows_abandonment_but_never_promotion_or_cleanup() {
    let (_root, state, work) = fixture();
    assert!(stop(&state, work.operation_id, false).is_ok());
    assert!(stop(&state, work.operation_id, true).is_err());
    let (_prefix, owner) = prefix::claim_work_prefix(state.state_root(), &work).unwrap();
    assert!(stop(&state, work.operation_id, false).is_err());
    drop(owner);
    let guard = stop(&state, work.operation_id, false).unwrap();
    assert!(stop(&state, work.operation_id, false).is_err());
    drop(guard);
}
#[test]
fn live_unknown_and_failed_hosts_never_authorize_completed_repair_recovery() {
    for completed in [false, true] {
        let (_root, mut state, work) = fixture();
        let record = state.begin_helper(work.operation_id).unwrap();
        assert!(stop(&state, work.operation_id, false).is_err());
        state
            .record_helper_host(work.operation_id, record.attempt_id, std::process::id())
            .unwrap();
        assert!(stop(&state, work.operation_id, false).is_err());
        state
            .finish_helper(
                work.operation_id,
                record.attempt_id,
                if completed {
                    helper_supervisor::Outcome::Completed
                } else {
                    helper_supervisor::Outcome::Cancelled
                },
            )
            .unwrap();
        assert!(stop(&state, work.operation_id, true).is_err());
        assert!(stop(&state, work.operation_id, false).is_err());
    }
}
#[test]
fn changed_prefix_descriptor_and_symlinked_owner_are_refused() {
    let (root, state, work) = fixture();
    let (bottle, owner) = prefix::claim_work_prefix(state.state_root(), &work).unwrap();
    drop(owner);
    let marker = bottle.parent().unwrap().join("owner.json");
    let original = std::fs::read(&marker).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["work"]["operation_id"] = serde_json::Value::String(Uuid::new_v4().to_string());
    std::fs::write(&marker, serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(stop(&state, work.operation_id, false).is_err());
    let outside = root.path().join("outside.json");
    std::fs::write(&outside, original).unwrap();
    std::fs::remove_file(&marker).unwrap();
    std::os::unix::fs::symlink(outside, marker).unwrap();
    assert!(stop(&state, work.operation_id, false).is_err());
}
