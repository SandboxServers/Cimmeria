use super::*;
use crate::{catalog::verify_release, state::InstalledState};
use cimmeria_runtime_probe::{collect, physx::SdkResult, LoadResult};
use ed25519_dalek::{Signer, SigningKey};
fn fixture() -> (tempfile::TempDir, DesktopState, InstallIntent) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    let sig = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let sig: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let release = verify_release(&body, sig.as_bytes()).unwrap();
    let owner = Uuid::new_v4();
    let intent = state
        .admit_install_backend(AdmissionRequest {
            id: owner,
            operation_revision: 0,
            preferences_revision: 1,
            release: &release,
            login_servers: vec![],
            backend: ExtractionBackend::Wine {
                runtime_sha256: [7; 32],
                helper_sha256: [8; 32],
            },
        })
        .unwrap()
        .intent;
    state
        .operations_mut()
        .unwrap()
        .observe(owner, OperationState::Running)
        .unwrap();
    let game = intent.destination.join("game");
    std::fs::create_dir_all(game.join("Working/Binaries")).unwrap();
    std::fs::create_dir_all(game.join("Working/SGWGame")).unwrap();
    std::fs::write(game.join("Working/Binaries/SGW.exe"), b"inert fixture").unwrap();
    InstalledState {
        seed_sha256: Some("a".repeat(64)),
        ..Default::default()
    }
    .save(&game)
    .unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        atomic::write(&intent.destination, name, &intent).unwrap();
    }
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(owner, OperationState::Succeeded)
        .unwrap();
    (root, state, intent)
}
fn admit(state: &mut DesktopState, installation: &InstallIntent) -> Plan {
    let revision = state.operations.snapshot().revision;
    state
        .admit_runtime_setup(
            Uuid::new_v4(),
            revision,
            installation.operation_id,
            [7; 32],
            [9; 32],
        )
        .unwrap()
        .plan
}
fn observation(plan: &Plan, sdk: SdkResult) -> PrepareResult {
    let mut report = collect(LoadResult::Loaded {}, |_| LoadResult::Loaded {});
    report.physx_sdk = sdk;
    PrepareResult {
        schema_version: 1,
        operation_id: plan.id,
        prefix_generation: plan.prefix_generation,
        result: ResultKind::Probed { report },
    }
}
#[test]
fn binds_installed_identity_not_preferences_and_duplicate_does_not_dispatch() {
    let (root, mut state, installed) = fixture();
    state
        .save_preferences(Some(root.path().join("another")), true, 1)
        .unwrap();
    let preferences = state.preferences().clone();
    let revision = state.operations.snapshot().revision;
    let plan = admit(&mut state, &installed);
    assert_eq!(plan.installation, installed);
    assert!(!plan.prefix_directory(state.state_root()).exists());
    let retry = state
        .admit_runtime_setup(plan.id, revision, installed.operation_id, [7; 32], [9; 32])
        .unwrap();
    assert!(!retry.dispatch);
    assert_eq!(retry.plan, plan);
    assert!(state
        .admit_runtime_setup(plan.id, revision, installed.operation_id, [7; 32], [10; 32])
        .is_err());
    state.begin_runtime_dispatch(plan.id).unwrap();
    assert!(state.begin_runtime_dispatch(plan.id).is_err());
    assert_eq!(
        state.runtime_record().unwrap().unwrap().phase,
        Phase::LaunchIntent
    );
    assert_eq!(state.preferences(), &preferences);
}
#[test]
fn restart_never_replays_each_dispatch_boundary_or_promotes_observed_success() {
    for boundary in 0..4 {
        let (root, mut state, installed) = fixture();
        let plan = admit(&mut state, &installed);
        if boundary >= 1 {
            state.begin_runtime_dispatch(plan.id).unwrap();
        }
        if boundary >= 2 {
            state.record_runtime_host(plan.id, 123).unwrap();
        }
        if boundary >= 3 {
            state
                .record_runtime_observation(
                    plan.id,
                    observation(&plan, SdkResult::InitializedAndReleased {}),
                )
                .unwrap();
        }
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        assert_eq!(
            state
                .operations
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
        assert_eq!(state.runtime_plan().unwrap().unwrap(), plan);
        assert!(
            !state
                .admit_runtime_setup(plan.id, 0, installed.operation_id, [7; 32], [9; 32])
                .unwrap()
                .dispatch
        );
        assert!(state.begin_runtime_dispatch(plan.id).is_err());
        assert!(state.record_runtime_host(plan.id, 124).is_err());
        if boundary < 3 {
            assert!(state.finish_runtime_after_stop(plan.id).is_err());
        }
        assert!(!plan.prefix_directory(state.state_root()).exists());
    }
}
#[test]
fn only_observed_complete_checks_after_stop_can_commit_success() {
    for verified in [false, true] {
        let (root, mut state, installed) = fixture();
        let plan = admit(&mut state, &installed);
        state.begin_runtime_dispatch(plan.id).unwrap();
        assert!(state.record_runtime_host(plan.id, 0).is_err());
        assert!(state.finish_runtime_after_stop(plan.id).is_err());
        state.record_runtime_host(plan.id, 123).unwrap();
        assert!(state.record_runtime_host(plan.id, 124).is_err());
        let sdk = if verified {
            SdkResult::InitializedAndReleased {}
        } else {
            SdkResult::CreateFailed { sdk_error: Some(1) }
        };
        state
            .record_runtime_observation(plan.id, observation(&plan, sdk))
            .unwrap();
        assert_eq!(
            state
                .operations
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::Running
        );
        // Fixture supplies the native coordinator's quiescence observation only;
        // no processes are started by this persistence test.
        state.finish_runtime_after_stop(plan.id).unwrap();
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        assert_eq!(
            state
                .operations
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            if verified {
                OperationState::Succeeded
            } else {
                OperationState::Failed
            }
        );
        assert_eq!(
            state.runtime_record().unwrap().unwrap().phase,
            Phase::Quiescent
        );
        assert_eq!(
            state.installed_content().unwrap().unwrap().intent,
            installed
        );
        assert!(!state.preferences().launcher_summary_consent);
    }
}
#[test]
fn mismatched_results_and_plan_tampering_are_refused() {
    let (root, mut state, installed) = fixture();
    let plan = admit(&mut state, &installed);
    state.begin_runtime_dispatch(plan.id).unwrap();
    state.record_runtime_host(plan.id, 123).unwrap();
    let mut wrong = observation(&plan, SdkResult::InitializedAndReleased {});
    wrong.prefix_generation = Uuid::new_v4();
    assert!(state.record_runtime_observation(plan.id, wrong).is_err());
    let mut tampered = plan.clone();
    tampered.helper_sha256 = [1; 32];
    atomic::write(&root.path().join("state"), &plan_name(plan.id), &tampered).unwrap();
    assert!(state.runtime_plan().is_err());
    assert!(state.finish_runtime_after_stop(plan.id).is_err());
}
#[test]
fn admission_and_dispatch_recheck_identity_content_and_active_owner() {
    let (_root, mut state, installed) = fixture();
    let revision = state.operations.snapshot().revision;
    for (id, revision, owner, runtime, helper) in [
        (
            Uuid::new_v4(),
            revision - 1,
            installed.operation_id,
            [7; 32],
            [9; 32],
        ),
        (Uuid::new_v4(), revision, Uuid::new_v4(), [7; 32], [9; 32]),
        (
            Uuid::nil(),
            revision,
            installed.operation_id,
            [7; 32],
            [9; 32],
        ),
        (
            Uuid::new_v4(),
            revision,
            installed.operation_id,
            [6; 32],
            [9; 32],
        ),
        (
            Uuid::new_v4(),
            revision,
            installed.operation_id,
            [7; 32],
            [0; 32],
        ),
    ] {
        assert!(state
            .admit_runtime_setup(id, revision, owner, runtime, helper)
            .is_err());
    }
    let plan = admit(&mut state, &installed);
    assert!(state
        .admit_runtime_setup(
            Uuid::new_v4(),
            revision,
            installed.operation_id,
            [7; 32],
            [9; 32]
        )
        .is_err());
    std::fs::remove_file(installed.destination.join("game/Working/Binaries/SGW.exe")).unwrap();
    assert!(state.begin_runtime_dispatch(plan.id).is_err());
    assert!(state.runtime_record().unwrap().is_none());
}
#[test]
fn failed_dispatch_record_write_leaves_recovery_gate_and_cannot_spawn() {
    let (root, mut state, installed) = fixture();
    let plan = admit(&mut state, &installed);
    std::fs::create_dir(root.path().join("state").join(record_name(plan.id))).unwrap();
    assert!(state.begin_runtime_dispatch(plan.id).is_err());
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert!(state.begin_runtime_dispatch(plan.id).is_err());
    assert!(state.runtime_record().is_err());
}

#[test]
fn crash_between_quiescent_record_and_terminal_commit_stays_gated_on_reopen() {
    let (root, mut state, installed) = fixture();
    let plan = admit(&mut state, &installed);
    state.begin_runtime_dispatch(plan.id).unwrap();
    state.record_runtime_host(plan.id, 123).unwrap();
    state
        .record_runtime_observation(
            plan.id,
            observation(&plan, SdkResult::InitializedAndReleased {}),
        )
        .unwrap();
    assert!(state
        .finish_runtime_after_stop_with(plan.id, || Err(StorageError::Io.into()))
        .is_err());
    assert_eq!(
        state.runtime_record().unwrap().unwrap().phase,
        Phase::Quiescent
    );
    assert_eq!(
        state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Running
    );
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert_eq!(
        state.runtime_record().unwrap().unwrap().phase,
        Phase::Quiescent
    );
    // Explicit native reconciliation after a new prefix stop/wait observation.
    state.finish_runtime_after_stop(plan.id).unwrap();
    assert_eq!(
        state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Succeeded
    );
}

#[test]
fn cancellation_before_host_dispatch_refuses_request_and_blocks_other_mutations() {
    let (_root, mut state, installed) = fixture();
    let plan = admit(&mut state, &installed);
    state.begin_runtime_dispatch(plan.id).unwrap();
    state
        .operations_mut()
        .unwrap()
        .request_cancel(plan.id)
        .unwrap();
    assert!(state.record_runtime_host(plan.id, 123).is_err());
    assert_eq!(
        state.runtime_record().unwrap().unwrap().phase,
        Phase::LaunchIntent
    );
    assert!(state.finish_runtime_after_stop(plan.id).is_err());
    let revision = state.operations.snapshot().revision;
    assert!(state
        .uninstall(Uuid::new_v4(), revision, installed.operation_id, true)
        .is_err());
    assert!(installed
        .destination
        .join("game/Working/Binaries/SGW.exe")
        .exists());
}
