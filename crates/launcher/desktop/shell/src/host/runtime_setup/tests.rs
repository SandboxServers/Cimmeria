use super::*;
fn request() -> InstallCommand {
    InstallCommand::PrepareRuntime {
        schema_version: 1,
        operation_id: Uuid::new_v4(),
        operation_revision: 0,
        installation_id: Uuid::new_v4(),
    }
}
#[test]
fn missing_resource_rejects_before_storage_or_release_fetch() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let host = NativeHost::new(path.clone());
    assert!(!request().needs_release());
    assert_eq!(
        host.install_command(request(), None).unwrap_err(),
        JobError::PlatformUnavailable
    );
    assert!(!path.exists());
    for extra in [
        "path",
        "helper_sha256",
        "runtime_sha256",
        "result",
        "prefix_generation",
    ] {
        let mut value = serde_json::json!({"command":"prepare_runtime","schema_version":1,
            "operation_id":Uuid::new_v4(),"operation_revision":0,"installation_id":Uuid::new_v4()});
        value[extra] = serde_json::json!("forged");
        assert!(serde_json::from_value::<InstallCommand>(value).is_err());
    }
}
#[cfg(target_os = "macos")]
#[test]
fn replacement_rejects_before_admission_and_selection_is_not_installed_identity() {
    use cimmeria_launcher_engine::mac_wine::PrerequisiteResource;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let helper = root.path().join("worker.exe");
    std::fs::write(&helper, b"").unwrap();
    let mut host = NativeHost::new(path.clone());
    host.prerequisite_helper = Some(
        PrerequisiteResource::open(
            helper.clone(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap(),
    );
    std::fs::write(&helper, b"changed").unwrap();
    assert_eq!(
        host.install_command(request(), None).unwrap_err(),
        JobError::PlatformUnavailable
    );
    assert!(!path.exists());
    std::fs::write(helper, b"").unwrap();
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(root.path().join("selected")),
        launcher_summary_consent: true,
    })
    .unwrap();
    assert_eq!(
        host.install_command(request(), None).unwrap_err(),
        JobError::IdentityConflict
    );
    let snapshot = host.install_status().unwrap().native;
    assert!(snapshot.operation.operation.is_none());
    assert!(snapshot.preferences.launcher_summary_consent);
    assert!(!path.join("game-prefixes").exists());
    assert!(host.runtime_worker.lock().unwrap().is_none());
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires a staged native Windows x86 helper and matching compile-time digest"]
fn compiled_prerequisite_resource_resolves_without_starting_work() {
    let resources = PathBuf::from(std::env::var_os("CIMMERIA_TEST_RESOURCE_DIR").unwrap());
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state")).with_bundled_helper(resources);
    host.prerequisite_helper
        .as_ref()
        .expect("compile-time pin must match bundled resource")
        .verify()
        .unwrap();
    assert!(!root.path().join("state").exists());
    assert!(host.runtime_worker.lock().unwrap().is_none());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn admitted_setup_is_retained_and_duplicate_does_not_replace_worker() {
    use cimmeria_launcher_engine::{
        mac_wine::{HelperResource, PrerequisiteResource},
        OperationState,
    };
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join("worker.exe");
    std::fs::write(&helper, b"").unwrap();
    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let mut host = NativeHost::new(root.path().join("state"));
    host.prerequisite_helper = Some(PrerequisiteResource::open(helper.clone(), EMPTY).unwrap());
    let backend = HelperResource::open(helper, EMPTY).unwrap().backend();
    let installation = Uuid::new_v4();
    let state = host.store().unwrap();
    let revision = {
        let mut owner = state.lock().unwrap();
        owner
            .save_preferences(Some(root.path().join("install")), true, 0)
            .unwrap();
        // This signed inert content fixture proves dispatch ownership, not game compatibility.
        let release = super::super::install::tests::fixture_release();
        let intent = owner
            .admit_install_backend(cimmeria_launcher_engine::AdmissionRequest {
                id: installation,
                operation_revision: 0,
                preferences_revision: 1,
                release: &release,
                login_servers: vec![],
                backend,
            })
            .unwrap()
            .intent;
        let game = intent.destination.join("game");
        std::fs::create_dir_all(game.join("Working/Binaries")).unwrap();
        std::fs::create_dir_all(game.join("Working/SGWGame")).unwrap();
        std::fs::write(game.join("Working/Binaries/SGW.exe"), b"fixture").unwrap();
        cimmeria_launcher_engine::state::InstalledState {
            seed_sha256: Some("a".repeat(64)),
            applied_patches: vec![],
            seed_adopted: false,
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
        owner
            .operations_mut()
            .unwrap()
            .observe(installation, OperationState::Running)
            .unwrap();
        owner
            .operations_mut()
            .unwrap()
            .observe(installation, OperationState::Succeeded)
            .unwrap();
        // Public inspection migrates the completed content reference if needed.
        owner.installed_content().unwrap().unwrap();
        owner.operations().snapshot().revision
    };
    assert_eq!(
        host.install_status().unwrap().runtime_setup,
        Some(installation)
    );
    let id = Uuid::new_v4();
    let request = || InstallCommand::PrepareRuntime {
        schema_version: 1,
        operation_id: id,
        operation_revision: revision,
        installation_id: installation,
    };
    host.install_command(request(), None).unwrap();
    assert!(host.install_status().unwrap().runtime_setup.is_none());
    host.install_command(request(), None).unwrap();
    host.install_command(
        InstallCommand::Cancel {
            schema_version: 1,
            operation_id: id,
        },
        None,
    )
    .unwrap();
    let mut result = host
        .runtime_worker
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .result
        .clone();
    assert_eq!(
        host.runtime_worker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .operation_id(),
        id
    );
    // Cancellation is routed to the retained runtime worker. No cache exists,
    // so resource claim stops before any Wine process/download.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while result.borrow().is_none() {
            result.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let status = host.install_status().unwrap();
    assert_eq!(
        status.native.operation.operation.unwrap().state,
        OperationState::Cancelled
    );
    assert!(status.native.preferences.launcher_summary_consent);
    assert_eq!(status.runtime_setup, Some(installation));
    assert!(!root.path().join("state/game-prefixes").exists());
    assert!(state.lock().unwrap().runtime_record().unwrap().is_some());
    drop(host);
    drop(state);
    let reopened = NativeHost::new(root.path().join("state"));
    assert_eq!(
        reopened
            .install_status()
            .unwrap()
            .native
            .operation
            .operation
            .unwrap()
            .id,
        id
    );
}
