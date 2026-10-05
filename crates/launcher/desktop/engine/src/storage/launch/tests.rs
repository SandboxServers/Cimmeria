use super::*;
pub(super) fn artifact(root: &Path, name: &str) -> Artifact {
    let path = root.join(name);
    std::fs::write(&path, b"fixture").unwrap();
    Artifact::open(
        path,
        &Sha256::digest(b"fixture")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
    )
    .unwrap()
}
pub(super) fn fixture() -> (tempfile::TempDir, DesktopState, Plan) {
    let (root, mut state, installation) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let resources = Resources {
        helper: artifact(&root.path().canonicalize().unwrap(), "helper.exe"),
        client_patches: None,
        client_telemetry: None,
        graphics: None,
    };
    // A persisted plan lets lifecycle tests run without claiming platform setup.
    let plan = Plan {
        id: Uuid::new_v4(),
        installation,
        runtime: None,
        resources,
    };
    state
        .write_launch(&format!("launch-plan-{}.json", plan.id), &plan)
        .unwrap();
    let revision = state.operations.snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(
            plan.id,
            OperationKind::Launch,
            plan.digest().unwrap(),
            revision,
        )
        .unwrap();
    (root, state, plan)
}
#[test]
fn retry_never_dispatches_twice_and_restart_never_reports_live_guest() {
    let (root, mut state, plan) = fixture();
    state
        .record_launch(
            &plan,
            Observation::ProcessStarted {
                host_pid: 42,
                guest_pid: 80,
            },
        )
        .unwrap();
    let result = state
        .admit_launch(
            plan.id,
            0,
            plan.installation.operation_id,
            plan.resources.clone(),
        )
        .unwrap();
    assert!(!result.dispatch);
    assert!(matches!(
        state.launch_observation().unwrap(),
        Some(Observation::ProcessStarted {
            host_pid: 42,
            guest_pid: 80
        })
    ));
    let revision = state.operations.snapshot().revision;
    assert!(matches!(
        state.admit_launch(
            Uuid::new_v4(),
            revision,
            plan.installation.operation_id,
            plan.resources.clone()
        ),
        Err(IntentError::Operation(ContractError::Busy))
    ));
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state.launch_observation().unwrap(),
        Some(Observation::Unknown)
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
}
#[test]
fn patch_selection_adds_no_plan_field_so_earlier_plans_keep_their_digest() {
    let (_root, state, plan) = fixture();
    let value = serde_json::to_value(&plan).unwrap();
    let keys = |value: &serde_json::Value| -> Vec<String> {
        value.as_object().unwrap().keys().cloned().collect()
    };
    // The effective selection is the nullable artifact already in every plan. A
    // new field, even a defaulted one, would change the bytes this digest covers.
    assert_eq!(keys(&value), ["id", "installation", "resources", "runtime"]);
    assert_eq!(
        keys(&value["resources"]),
        ["client_patches", "graphics", "helper"]
    );
    assert!(value["resources"]["client_patches"].is_null());
    let stored: Plan = serde_json::from_value(value).unwrap();
    assert_eq!(stored.digest().unwrap(), plan.digest().unwrap());
    assert_eq!(state.launch_plan().unwrap(), Some(plan));
}
#[test]
fn resource_replacement_and_plan_tampering_fail_closed() {
    let (_root, mut state, plan) = fixture();
    std::fs::write(plan.resources.helper.path(), b"replacement").unwrap();
    assert!(plan.resources.verify().is_err());
    let mut altered = plan.clone();
    altered.installation.login_servers.clear();
    state
        .write_launch(&format!("launch-plan-{}.json", plan.id), &altered)
        .unwrap();
    assert!(state.launch_plan().is_err());
}
#[tokio::test]
async fn duplicate_dispatch_and_pre_spawn_cancel_are_observed_without_game() {
    let (_root, state, plan) = fixture();
    let state = Arc::new(std::sync::Mutex::new(state));
    let mut worker = dispatch(state.clone(), plan.id).unwrap();
    assert!(dispatch(state.clone(), plan.id).is_err());
    worker.request_cancel().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while *worker.observation.borrow_and_update() != Observation::Cancelled {
            worker.observation.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let owner = state.lock().unwrap();
    assert_eq!(
        owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Cancelled
    );
    assert_eq!(
        owner.launch_observation().unwrap(),
        Some(Observation::Cancelled)
    );
}
#[cfg(target_os = "macos")]
#[test]
fn admission_requires_prepared_runtime_and_graphics_and_preserves_consent() {
    use cimmeria_runtime_probe::{
        collect,
        physx::SdkResult,
        prerequisite::{PrepareResult, ResultKind},
        LoadResult,
    };
    let (root, mut state, installed) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let directory = root.path().canonicalize().unwrap();
    let resources = Resources {
        helper: artifact(&directory, "helper.exe"),
        client_patches: None,
        client_telemetry: None,
        graphics: Some(Graphics {
            d3d9: artifact(&directory, "d3d9.dll"),
            rosetta_x87: None,
        }),
    };
    assert!(state
        .admit_launch(
            Uuid::new_v4(),
            state.operations.snapshot().revision,
            installed.operation_id,
            resources.clone()
        )
        .is_err());
    let runtime_id = Uuid::new_v4();
    let plan = state
        .admit_runtime_setup(
            runtime_id,
            state.operations.snapshot().revision,
            installed.operation_id,
            [7; 32],
            [9; 32],
        )
        .unwrap()
        .plan;
    state.begin_runtime_dispatch(runtime_id).unwrap();
    state.record_runtime_host(runtime_id, 42).unwrap();
    let mut report = collect(LoadResult::Loaded {}, |_| LoadResult::Loaded {});
    report.physx_sdk = SdkResult::InitializedAndReleased {};
    state
        .record_runtime_observation(
            runtime_id,
            PrepareResult {
                schema_version: 1,
                operation_id: runtime_id,
                prefix_generation: plan.prefix_generation,
                result: ResultKind::Probed { report },
            },
        )
        .unwrap();
    state.finish_runtime_after_stop(runtime_id).unwrap();
    let mut missing = resources.clone();
    missing.graphics = None;
    assert!(state
        .admit_launch(
            Uuid::new_v4(),
            state.operations.snapshot().revision,
            installed.operation_id,
            missing
        )
        .is_err());
    let id = Uuid::new_v4();
    let admission = state
        .admit_launch(
            id,
            state.operations.snapshot().revision,
            installed.operation_id,
            resources.clone(),
        )
        .unwrap();
    assert!(admission.dispatch);
    assert_eq!(admission.plan.runtime, Some(plan));
    assert!(!state.preferences().launcher_summary_consent);
    assert!(
        !state
            .admit_launch(id, 0, installed.operation_id, resources)
            .unwrap()
            .dispatch
    );
}
#[tokio::test]
async fn dropping_observer_retains_native_task_and_persists_preparation_failure() {
    let (_root, state, plan) = fixture();
    // A replaced bundle fails before touching the game, on either native OS.
    std::fs::write(plan.resources.helper.path(), b"replacement").unwrap();
    let state = Arc::new(std::sync::Mutex::new(state));
    drop(dispatch(state.clone(), plan.id).unwrap());
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if state
                .lock()
                .unwrap()
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state
                .terminal()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let owner = state.lock().unwrap();
    assert_eq!(
        owner.launch_observation().unwrap(),
        Some(Observation::NotStarted)
    );
    assert_eq!(
        owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Failed
    );
}

#[test]
fn a_plan_without_telemetry_serializes_as_it_did_before_the_field_existed() {
    let (root, _state, plan) = fixture();
    // Stored plans are digested from their JSON: an absent DLL must add no key,
    // or every plan written by an earlier launcher would stop matching.
    let text = serde_json::to_string(&plan.resources).unwrap();
    assert!(!text.contains("client_telemetry"));
    assert_eq!(
        serde_json::from_str::<Resources>(&text).unwrap(),
        plan.resources
    );
    let with = Resources {
        client_telemetry: Some(artifact(
            &root.path().canonicalize().unwrap(),
            "telemetry.dll",
        )),
        ..plan.resources.clone()
    };
    let text = serde_json::to_string(&with).unwrap();
    assert!(text.contains("client_telemetry"));
    assert_eq!(serde_json::from_str::<Resources>(&text).unwrap(), with);
    assert_ne!(
        Plan {
            resources: with,
            ..plan.clone()
        }
        .digest()
        .unwrap(),
        plan.digest().unwrap()
    );
}

#[test]
fn the_telemetry_dll_is_resolved_only_for_a_player_who_opted_in() {
    let (root, mut state, installed) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let directory = root.path().canonicalize().unwrap();
    let bundled = Resources {
        helper: artifact(&directory, "helper.exe"),
        client_patches: Some(artifact(&directory, "patches.dll")),
        client_telemetry: Some(artifact(&directory, "telemetry.dll")),
        graphics: None,
    };
    let resolve = |state: &DesktopState| {
        state
            .resolve_play_resources(bundled.clone())
            .unwrap()
            .expect("patches are bundled")
    };
    assert_eq!(resolve(&state).client_telemetry, None);
    assert!(resolve(&state).client_patches.is_some());
    // Launcher-summary consent is a different choice and changes nothing here.
    let preferences = state.preferences().clone();
    state
        .save_preferences(preferences.install_directory, true, preferences.revision)
        .unwrap();
    assert_eq!(resolve(&state).client_telemetry, None);

    // A host that skipped resolution cannot inject it either.
    let revision = state.operations.snapshot().revision;
    assert!(matches!(
        state.admit_launch(
            Uuid::new_v4(),
            revision,
            installed.operation_id,
            bundled.clone()
        ),
        Err(IntentError::Operation(ContractError::IdentityConflict))
    ));

    state.set_game_telemetry(true).unwrap();
    assert_eq!(resolve(&state).client_telemetry, bundled.client_telemetry);
    state.set_game_telemetry(false).unwrap();
    assert_eq!(resolve(&state).client_telemetry, None);
}

#[test]
fn a_lab_build_of_the_telemetry_dll_is_refused_even_with_consent() {
    let (root, mut state, installed) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let directory = root.path().canonicalize().unwrap();
    let bytes = b"MZ...cimmeria-client-telemetry build flavour: lab-bridge...";
    std::fs::write(directory.join("lab.dll"), bytes).unwrap();
    let lab = Artifact::open(
        directory.join("lab.dll"),
        &Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
    )
    .unwrap();
    assert!(lab.refuse_lab_build().is_err());
    assert!(artifact(&directory, "player.dll")
        .refuse_lab_build()
        .is_ok());
    state.set_game_telemetry(true).unwrap();
    let revision = state.operations.snapshot().revision;
    assert!(matches!(
        state.admit_launch(
            Uuid::new_v4(),
            revision,
            installed.operation_id,
            Resources {
                helper: artifact(&directory, "helper.exe"),
                client_patches: None,
                client_telemetry: Some(lab),
                graphics: None,
            }
        ),
        Err(IntentError::Storage(StorageError::UnsafeFile))
    ));
}
