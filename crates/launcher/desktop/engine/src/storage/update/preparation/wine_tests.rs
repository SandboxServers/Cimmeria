use super::*;
use crate::mac_wine::HelperResource;
#[tokio::test]
async fn wine_helper_identity_refusal_and_precancel_keep_old_game_and_target_binding() {
    let bundle = tempfile::tempdir().unwrap();
    let helper_path = bundle.path().join("helper.exe");
    std::fs::write(&helper_path, b"inert helper").unwrap();
    let hash: String = Sha256::digest(b"inert helper")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let helper = HelperResource::open(helper_path.clone(), &hash).unwrap();
    let ExtractionBackend::Wine {
        runtime_sha256,
        helper_sha256,
    } = helper.backend()
    else {
        unreachable!()
    };
    let old_hash: String = Sha256::digest(b"old")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (root, mut state, installed) =
        runtime_setup::tests::fixture_with_backend(runtime_sha256, helper_sha256, &old_hash, 3);
    let target = install_worker::tests::verified(&install_worker::tests::archive(true));
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_update(Request {
            id: Uuid::new_v4(),
            operation_revision: revision,
            installation_id: installed.operation_id,
            expected_current: installed.release_identity(),
            target: &target,
            confirmed: true,
        })
        .unwrap()
        .plan;
    assert_eq!(
        state.cached_extraction_release(plan.id).unwrap().digest(),
        target.digest()
    );
    let state = Arc::new(Mutex::new(state));
    std::fs::remove_file(&helper_path).unwrap();
    assert!(prepare_wine(state.clone(), plan.id, helper.clone()).is_err());
    assert_eq!(
        state
            .lock()
            .unwrap()
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Starting
    );
    std::fs::write(helper_path, b"inert helper").unwrap();
    let work = prepare_wine(state.clone(), plan.id, helper).unwrap();
    work.request_cancel().unwrap();
    assert!(matches!(
        work.result.await.unwrap(),
        Err(Failure::Cancelled)
    ));
    assert!(!plan.work_directory().exists());
    assert!(!plan.backup().exists());
    assert!(!root.path().join("state/wine-repair-prefixes").exists());
    assert_eq!(
        std::fs::read(plan.owner.destination.join("game/Working/Binaries/SGW.exe")).unwrap(),
        b"inert fixture"
    );
}
