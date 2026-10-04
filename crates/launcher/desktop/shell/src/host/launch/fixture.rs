//! Inert files and successful fixture prerequisite evidence; never runs a game.
use super::*;
use cimmeria_launcher_engine::{ExtractionBackend, OperationKind, OperationState};
use sha2::{Digest, Sha256};
pub(super) fn fixture() -> (tempfile::TempDir, NativeHost) {
    let root = tempfile::tempdir().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    let mut host = NativeHost::new(canonical.join("state"));
    let path = canonical.join("inert.dll");
    std::fs::write(&path, b"").unwrap();
    let artifact = launch::Artifact::open(
        path,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    )
    .unwrap();
    host.launch_resources = Some(launch::Resources {
        helper: artifact.clone(),
        client_patches: Some(artifact.clone()),
        graphics: Some(launch::Graphics {
            d3d9: artifact,
            rosetta_x87: None,
        }),
    });
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    state
        .save_preferences(Some(canonical.join("game")), true, 0)
        .unwrap();
    let id = Uuid::new_v4();
    use ed25519_dalek::{Signer, SigningKey};
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"http://127.0.0.1:9/seed","size":1,"sha256":"a".repeat(64)},"patches":[],"min_launcher":"launcher-20261004-fixture"})).unwrap();
    let signature: String = SigningKey::from_bytes(&[0x2a; 32])
        .sign(&body)
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let release =
        cimmeria_launcher_engine::catalog::verify_release(&body, signature.as_bytes()).unwrap();
    let intent = state
        .admit_install_backend(cimmeria_launcher_engine::AdmissionRequest {
            id,
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
    let game = intent.destination.join("game");
    std::fs::create_dir_all(game.join("Working/Binaries")).unwrap();
    std::fs::create_dir_all(game.join("Working/SGWGame")).unwrap();
    std::fs::write(game.join("Working/Binaries/SGW.exe"), b"inert").unwrap();
    cimmeria_launcher_engine::state::InstalledState {
        seed_sha256: Some(release.manifest().seed.sha256.clone()),
        ..Default::default()
    }
    .save(&game)
    .unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        std::fs::write(
            intent.destination.join(name),
            serde_json::to_vec(&intent).unwrap(),
        )
        .unwrap();
    }
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Succeeded)
        .unwrap();
    state.installed_content().unwrap();
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_runtime_setup(Uuid::new_v4(), revision, id, [7; 32], [9; 32])
        .unwrap()
        .plan;
    state.begin_runtime_dispatch(plan.id).unwrap();
    state.record_runtime_host(plan.id, 123).unwrap();
    let mut report =
        cimmeria_runtime_probe::collect(cimmeria_runtime_probe::LoadResult::Loaded {}, |_| {
            cimmeria_runtime_probe::LoadResult::Loaded {}
        });
    report.physx_sdk = cimmeria_runtime_probe::physx::SdkResult::InitializedAndReleased {};
    state
        .record_runtime_observation(
            plan.id,
            cimmeria_runtime_probe::prerequisite::PrepareResult {
                schema_version: 1,
                operation_id: plan.id,
                prefix_generation: plan.prefix_generation,
                result: cimmeria_runtime_probe::prerequisite::ResultKind::Probed { report },
            },
        )
        .unwrap();
    state.finish_runtime_after_stop(plan.id).unwrap();
    drop(state);
    drop(store);
    (root, host)
}
pub(super) fn admit(host: &NativeHost, id: Uuid) -> launch::Plan {
    let status = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    host.store()
        .unwrap()
        .lock()
        .unwrap()
        .admit_launch(
            id,
            status.native.operation.revision,
            status.installation_id.unwrap(),
            host.launch_resources.clone().unwrap(),
        )
        .unwrap()
        .plan
}
pub(super) fn observe(host: &NativeHost, phase: &str) {
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    let plan = state.launch_plan().unwrap().unwrap();
    let observation = match phase {
        "started" => launch::Observation::ProcessStarted {
            host_pid: 11,
            guest_pid: 22,
        },
        "exit" => launch::Observation::ProcessExited {
            host_pid: 11,
            guest_pid: 22,
            code: 0,
            early: true,
        },
        _ => launch::Observation::Unknown,
    };
    let digest: [u8; 32] = Sha256::digest(serde_json::to_vec(&plan).unwrap()).into();
    std::fs::write(
        host.root.join(format!("launch-result-{}.json", plan.id)),
        serde_json::to_vec(
            &serde_json::json!({"id":plan.id,"digest":digest,"observation":observation}),
        )
        .unwrap(),
    )
    .unwrap();
    let ops = state.operations_mut().unwrap();
    match phase {
        "started" => {
            ops.observe(plan.id, OperationState::Running).unwrap();
        }
        "exit" => {
            ops.observe(plan.id, OperationState::Succeeded).unwrap();
        }
        _ => {
            ops.mark_uncertain(plan.id).unwrap();
        }
    }
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .kind,
        OperationKind::Launch
    );
}

pub(super) fn reopen_identity(host: NativeHost, older: bool) -> NativeHost {
    use cimmeria_launcher_engine::launcher_compatibility::{CompatibilityPolicy, Identity};
    let path = host.root.clone();
    let resources = host.launch_resources.clone();
    drop(host);
    let policy = if older {
        CompatibilityPolicy::new(
            Identity::from_parts(
                "0.1.0",
                None,
                Some("launcher-20261003-fixture"),
                Some("1791000000"),
            ),
            vec![],
        )
    } else {
        CompatibilityPolicy::default()
    };
    let state = DesktopState::open_with_compatibility(&path, policy).unwrap();
    let mut host = NativeHost::new(path);
    host.launch_resources = resources;
    *host.state.lock().unwrap() = Some(Arc::new(Mutex::new(state)));
    host
}
