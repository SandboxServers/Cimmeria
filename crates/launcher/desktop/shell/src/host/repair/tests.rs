use super::*;
#[test]
fn unconfirmed_repair_actions_do_not_open_storage() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let host = NativeHost::new(path.clone());
    let id = Uuid::new_v4();
    for request in [
        InstallCommand::Repair {
            schema_version: 1,
            operation_id: id,
            operation_revision: 0,
            installation_id: Uuid::new_v4(),
            confirmed: false,
        },
        InstallCommand::RecoverRepair {
            schema_version: 1,
            operation_id: id,
            operation_revision: 0,
            confirmed: false,
        },
        InstallCommand::AbandonRepair {
            schema_version: 1,
            operation_id: id,
            operation_revision: 0,
            confirmed: false,
        },
        InstallCommand::CleanupRepair {
            schema_version: 1,
            operation_id: id,
            operation_revision: 0,
            confirmed: false,
        },
    ] {
        assert_eq!(
            host.install_command(request, None).unwrap_err(),
            JobError::RecoveryRequired
        );
    }
    assert!(!path.exists());
}
#[test]
fn repair_schema_rejects_forged_destination_and_requires_confirmation() {
    let id = Uuid::new_v4();
    for extra in [",\"directory\":\"/other\",\"confirmed\":true", ""] {
        let value = format!(
            r#"{{"command":"repair","schema_version":1,"operation_id":"{id}","installation_id":"{id}","operation_revision":0{extra}}}"#
        );
        assert!(serde_json::from_str::<InstallCommand>(&value).is_err());
    }
}
#[cfg(target_os = "macos")]
fn fixture() -> (tempfile::TempDir, NativeHost, Uuid) {
    let root = tempfile::tempdir().unwrap();
    let mut host = NativeHost::new(root.path().join("state"));
    let helper = root.path().join("helper.exe");
    std::fs::write(&helper, b"").unwrap();
    host.helper = Some(
        cimmeria_launcher_engine::mac_wine::HelperResource::open(
            helper,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap(),
    );
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    state
        .save_preferences(Some(root.path().join("installation")), true, 0)
        .unwrap();
    let id = Uuid::new_v4();
    let intent = state
        .admit_install_backend(cimmeria_launcher_engine::AdmissionRequest {
            id,
            operation_revision: 0,
            preferences_revision: 1,
            release: &super::super::install::tests::fixture_release(),
            login_servers: vec![],
            backend: host.helper.as_ref().unwrap().backend(),
        })
        .unwrap()
        .intent;
    std::fs::create_dir_all(intent.destination.join("game")).unwrap();
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
    state
        .save_preferences(Some(root.path().join("different-selection")), true, 1)
        .unwrap();
    drop(state);
    (root, host, id)
}
#[cfg(target_os = "macos")]
#[tokio::test]
async fn missing_game_identity_survives_reopen_and_explicit_abandonment() {
    let (_root, host, installation) = fixture();
    let status = host.install_status().unwrap();
    let target = status.repair.target.unwrap();
    assert_eq!(target.installation_id, installation);
    std::fs::remove_dir_all(target.directory.join("game")).unwrap();
    let store = host.store().unwrap();
    let work = Uuid::new_v4();
    let plan = store
        .lock()
        .unwrap()
        .admit_repair(work, status.native.operation.revision, installation, true)
        .unwrap()
        .plan;
    assert!(!plan.original_present);
    let path = host.root.clone();
    let helper = host.helper.clone();
    drop(store);
    drop(host);
    let mut host = NativeHost::new(path);
    host.helper = helper;
    let status = host.install_status().unwrap();
    assert!(status.repair.recovery);
    assert!(!status.can_resume);
    assert!(!status.can_reconcile);
    let revision = status.native.operation.revision;
    let host = Arc::new(host);
    let response = tokio::task::spawn_blocking(move || {
        assert_eq!(
            host.install_command(
                InstallCommand::RecoverRepair {
                    schema_version: 1,
                    operation_id: work,
                    operation_revision: revision,
                    confirmed: true
                },
                None
            )
            .unwrap_err(),
            JobError::CorruptState
        );
        host.install_command(
            InstallCommand::AbandonRepair {
                schema_version: 1,
                operation_id: work,
                operation_revision: revision,
                confirmed: true,
            },
            None,
        )
        .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(
        response.native.operation.operation.unwrap().state,
        OperationState::Cancelled
    );
    assert!(response.native.preferences.launcher_summary_consent);
    assert!(response.repair.target.is_some());
    assert!(!plan.stage().exists());
    assert!(!plan.backup().exists());
}

/// JS drives real durable admission/cancellation/reopen/abandonment. Preparation
/// is held at the engine admission seam; no Wine process/download is started.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "interactive JSON-lines fixture for repair-uat.mjs"]
fn repair_uat_bridge() {
    use std::io::{BufRead, Write};
    let (_root, mut host, _) = fixture();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result: Result<InstallStatus, JobError> = if value["command"] == "reopen" {
            let path = host.root.clone();
            let helper = host.helper.clone();
            drop(host);
            host = NativeHost::new(path);
            host.helper = helper;
            host.install_status()
        } else {
            let request: InstallCommand = serde_json::from_value(value).unwrap();
            match request {
                InstallCommand::Repair {
                    operation_id,
                    operation_revision,
                    installation_id,
                    confirmed,
                    ..
                } => {
                    let store = host.store().unwrap();
                    let mut state = store.lock().unwrap();
                    state
                        .admit_repair(operation_id, operation_revision, installation_id, confirmed)
                        .unwrap();
                    state
                        .operations_mut()
                        .unwrap()
                        .observe(operation_id, OperationState::Running)
                        .unwrap();
                    drop(state);
                    host.install_status()
                }
                InstallCommand::Cancel { operation_id, .. } => {
                    let store = host.store().unwrap();
                    let mut state = store.lock().unwrap();
                    state
                        .operations_mut()
                        .unwrap()
                        .request_cancel(operation_id)
                        .unwrap();
                    state
                        .operations_mut()
                        .unwrap()
                        .observe(operation_id, OperationState::Cancelled)
                        .unwrap();
                    drop(state);
                    host.install_status()
                }
                _ => host.install_command(request, None),
            }
        };
        let reply = match result {
            Ok(status) => serde_json::json!({"ok":status}),
            Err(error) => serde_json::json!({"error":error}),
        };
        println!("REPAIR_UAT {reply}");
        std::io::stdout().flush().unwrap();
    }
}

#[cfg(target_os = "macos")]
#[test]
fn admitted_dispatch_failure_is_gated_and_duplicate_does_not_restart_work() {
    let (_root, host, installation) = fixture();
    let before = host.install_status().unwrap();
    let id = Uuid::new_v4();
    let request = InstallCommand::Repair {
        schema_version: 1,
        operation_id: id,
        operation_revision: before.native.operation.revision,
        installation_id: installation,
        confirmed: true,
    };
    assert_eq!(
        host.install_command(request.clone(), None).unwrap_err(),
        JobError::Io
    );
    let after = host.install_status().unwrap();
    assert_eq!(
        after.native.operation.operation.as_ref().unwrap().state,
        OperationState::ReconciliationRequired
    );
    assert!(after.repair.recovery);
    assert!(host.repair_worker.lock().unwrap().is_none());
    let duplicate = host.install_command(request, None).unwrap();
    assert_eq!(
        duplicate.native.operation.revision,
        after.native.operation.revision
    );
    assert_eq!(duplicate.native.preferences, before.native.preferences);
    let plan = host
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .repair_plan()
        .unwrap()
        .unwrap();
    assert!(!plan.stage().exists());
    assert!(!plan.backup().exists());
    assert!(plan.installation.destination.join("game").exists());
}
