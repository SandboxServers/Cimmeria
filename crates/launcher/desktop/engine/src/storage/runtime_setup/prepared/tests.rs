use super::*;
use cimmeria_runtime_probe::{collect, physx::SdkResult, LoadResult};
fn finish(state: &mut DesktopState, installation: &InstallIntent) -> Plan {
    let id = Uuid::new_v4();
    let plan = state
        .admit_runtime_setup(
            id,
            state.operations().snapshot().revision,
            installation.operation_id,
            [7; 32],
            [9; 32],
        )
        .unwrap()
        .plan;
    state.begin_runtime_dispatch(id).unwrap();
    state.record_runtime_host(id, 42).unwrap();
    let mut report = collect(LoadResult::Loaded {}, |_| LoadResult::Loaded {});
    report.physx_sdk = SdkResult::InitializedAndReleased {};
    state
        .record_runtime_observation(
            id,
            PrepareResult {
                schema_version: 1,
                operation_id: id,
                prefix_generation: plan.prefix_generation,
                result: ResultKind::Probed { report },
            },
        )
        .unwrap();
    state.finish_runtime_after_stop(id).unwrap();
    plan
}
#[test]
fn evidence_survives_reopen_and_preferences_but_new_attempt_invalidates_old_success() {
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    assert!(state.prepared_runtime().unwrap().is_none());
    let plan = finish(&mut state, &installed);
    assert_eq!(state.prepared_runtime().unwrap().unwrap().plan, plan);
    state
        .save_preferences(Some(root.path().join("other")), true, 1)
        .unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(state.prepared_runtime().unwrap().unwrap().plan, plan);
    assert!(state.preferences().launcher_summary_consent);
    let id = Uuid::new_v4();
    state
        .admit_runtime_setup(
            id,
            state.operations().snapshot().revision,
            installed.operation_id,
            [7; 32],
            [8; 32],
        )
        .unwrap();
    assert!(state.prepared_runtime().unwrap().is_none());
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Failed)
        .unwrap();
    assert!(state.prepared_runtime().unwrap().is_none());
    // A later non-runtime operation must not resurrect the superseded prefix.
    let work = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(work, OperationKind::Repair, [4; 32], revision)
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(work, OperationState::Failed)
        .unwrap();
    assert!(state.prepared_runtime().unwrap().is_none());
}
#[test]
fn active_work_damaged_content_and_tampered_evidence_never_return_prepared_runtime() {
    let (_root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    let plan = finish(&mut state, &installed);
    let id = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(id, OperationKind::Repair, [4; 32], revision)
        .unwrap();
    assert!(state.prepared_runtime().unwrap().is_none());
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Failed)
        .unwrap();
    assert_eq!(state.prepared_runtime().unwrap().unwrap().plan, plan);
    let game = installed.destination.join("game/Working/Binaries/SGW.exe");
    std::fs::remove_file(&game).unwrap();
    assert!(state.prepared_runtime().unwrap().is_none());
    std::fs::write(game, b"fixture").unwrap();
    let path = state.state_root().join(record_name(plan.id));
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["plan_digest"] = serde_json::json!(vec![0; 32]);
    std::fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(state.prepared_runtime().is_err());
}

#[test]
fn clearing_installed_reference_invalidates_historical_prefix_without_changing_consent() {
    let (_root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    finish(&mut state, &installed);
    assert!(state.prepared_runtime().unwrap().is_some());
    // Exercise the same reference removal used after confirmed deletion.
    state.forget_installed_content(&installed).unwrap();
    assert!(state.prepared_runtime().unwrap().is_none());
    assert!(!state.preferences().launcher_summary_consent);
}

#[test]
fn a_new_installation_never_inherits_previous_compatibility_evidence() {
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    finish(&mut state, &installed);
    let release = state.installed_content().unwrap().unwrap().release;
    state
        .save_preferences(Some(root.path().join("replacement")), false, 1)
        .unwrap();
    let id = Uuid::new_v4();
    let admission = state
        .admit_install_backend(AdmissionRequest {
            id,
            operation_revision: state.operations().snapshot().revision,
            preferences_revision: 2,
            release: &release,
            login_servers: vec![],
            backend: installed.backend.clone(),
        })
        .unwrap();
    let next = admission.intent;
    let game = next.destination.join("game");
    std::fs::create_dir_all(game.join("Working/Binaries")).unwrap();
    std::fs::create_dir_all(game.join("Working/SGWGame")).unwrap();
    std::fs::write(
        game.join("Working/Binaries/SGW.exe"),
        b"replacement fixture",
    )
    .unwrap();
    crate::state::InstalledState {
        seed_sha256: Some("a".repeat(64)),
        ..Default::default()
    }
    .save(&game)
    .unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        atomic::write(&next.destination, name, &next).unwrap();
    }
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Succeeded)
        .unwrap();
    assert!(state.prepared_runtime().unwrap().is_none());
}
