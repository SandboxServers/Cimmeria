use super::*;

#[test]
fn ipc_rejects_paths_urls_and_forged_outcomes() {
    for value in [
        r#"{"command":"inspect","schema_version":1,"url":"https://other"}"#,
        r#"{"command":"install","schema_version":1,"path":"/other"}"#,
        r#"{"command":"succeeded","schema_version":1}"#,
        r#"{"command":"cancel","schema_version":1,"operation_id":"not-a-uuid"}"#,
    ] {
        assert!(serde_json::from_str::<InstallCommand>(value).is_err());
    }
}

#[test]
fn unsupported_command_schema_cannot_initialize_storage() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let host = NativeHost::new(path.clone());
    assert_eq!(
        host.install_command(InstallCommand::Inspect { schema_version: 99 }, None)
            .unwrap_err(),
        JobError::UnsupportedSchema
    );
    assert!(!path.exists());
}

#[test]
fn observation_uses_saved_native_state_and_has_no_invented_progress() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    let snapshot = host
        .install_command(InstallCommand::Inspect { schema_version: 1 }, None)
        .unwrap();
    assert!(snapshot.native.operation.operation.is_none());
    assert!(snapshot.progress.is_none());
    assert!(snapshot.outcome.is_none());
    assert_eq!(snapshot.install_supported, cfg!(windows));
    let folder = root.path().join("selected");
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(folder.clone()),
        launcher_summary_consent: true,
    })
    .unwrap();
    assert_eq!(
        host.install_status()
            .unwrap()
            .native
            .preferences
            .install_directory,
        Some(folder)
    );
    assert!(
        host.install_status()
            .unwrap()
            .native
            .preferences
            .launcher_summary_consent
    );
}

#[cfg(not(windows))]
#[test]
fn unsupported_platform_rejects_install_before_storage_or_network() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let host = NativeHost::new(path.clone());
    assert_eq!(
        host.require_install_support(),
        Err(JobError::PlatformUnavailable)
    );
    assert_eq!(
        host.install_command(
            InstallCommand::Install {
                schema_version: 1,
                operation_id: Uuid::new_v4(),
                operation_revision: 0,
                preferences_revision: 0
            },
            None
        )
        .unwrap_err(),
        JobError::PlatformUnavailable
    );
    assert!(!path.exists());
}

#[test]
fn unknown_cancel_and_stale_reconcile_cannot_mutate_native_operation() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    assert_eq!(
        host.install_command(
            InstallCommand::Cancel {
                schema_version: 1,
                operation_id: Uuid::new_v4()
            },
            None
        )
        .unwrap_err(),
        JobError::UnknownOperation
    );
    assert_eq!(
        host.install_command(
            InstallCommand::Reconcile {
                schema_version: 1,
                operation_id: Uuid::new_v4(),
                operation_revision: 99
            },
            None
        )
        .unwrap_err(),
        JobError::StaleRevision
    );
    assert_eq!(host.install_status().unwrap().native.operation.revision, 0);
}

#[test]
fn progress_projection_excludes_paths_and_caps_javascript_integers() {
    let event = Progress::Extracting {
        label: "private label".into(),
        current: 5,
        total: 10,
        filename: "/private/file".into(),
    };
    let value = serde_json::to_string(&progress(&event)).unwrap();
    assert_eq!(value, r#"{"phase":"extraction","current":5,"total":10}"#);
    let event = Progress::Downloading {
        label: "private".into(),
        downloaded: u64::MAX,
        total: u64::MAX,
    };
    let value = serde_json::to_value(progress(&event)).unwrap();
    assert_eq!(value["current"], 9_007_199_254_740_991u64);
}

#[test]
fn shared_native_owner_outlives_host_while_worker_retains_it() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let host = NativeHost::new(path.clone());
    let retained = host.store().unwrap();
    drop(host);
    assert!(matches!(
        DesktopState::open(&path),
        Err(StorageError::InUse)
    ));
    drop(retained);
    assert!(DesktopState::open(&path).is_ok());
}

fn fixture_release() -> VerifiedRelease {
    fixture_release_padded(0)
}
fn fixture_release_padded(padding: usize) -> VerifiedRelease {
    use ed25519_dalek::{Signer, SigningKey};
    // HTTPS-only production transport rejects this loopback HTTP URL locally.
    // No server, GUI, game download or production endpoint is contacted.
    let mut body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"http://127.0.0.1:9/seed","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    body.extend(std::iter::repeat_n(b' ', padding));
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    let signature: String = signature
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    cimmeria_launcher_engine::catalog::verify_release(&body, signature.as_bytes()).unwrap()
}

#[tokio::test]
async fn host_retains_real_worker_and_reports_its_failure_without_duplicate_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(root.path().join("install")),
        launcher_summary_consent: false,
    })
    .unwrap();
    let id = Uuid::new_v4();
    {
        let mut worker = host.worker.lock().unwrap();
        // Exercise the same coordinator called behind the platform guard; Mac
        // public IPC remains unavailable until its compatibility adapter exists.
        start_install(
            host.store().unwrap(),
            &mut worker,
            id,
            0,
            1,
            fixture_release(),
        )
        .unwrap();
    }
    let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let status = host.install_status().unwrap();
            if status.outcome.is_some() {
                break status;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(terminal.outcome, Some(Outcome::InstallFailed));
    assert_eq!(
        terminal.native.operation.operation.as_ref().unwrap().state,
        cimmeria_launcher_engine::OperationState::Failed
    );
    assert!(root.path().join("install/.cimmeria-install.json").is_file());
    let revision = terminal.native.operation.revision;
    let mut worker = host.worker.lock().unwrap();
    start_install(
        host.store().unwrap(),
        &mut worker,
        id,
        0,
        1,
        fixture_release(),
    )
    .unwrap();
    drop(worker);
    assert_eq!(
        host.install_status().unwrap().native.operation.revision,
        revision
    );
}

#[test]
fn dispatch_without_runtime_is_immediately_gated_for_recovery() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(root.path().join("install")),
        launcher_summary_consent: false,
    })
    .unwrap();
    let id = Uuid::new_v4();
    let mut worker = None;
    assert_eq!(
        start_install(
            host.store().unwrap(),
            &mut worker,
            id,
            0,
            1,
            fixture_release()
        ),
        Err(JobError::Io)
    );
    assert!(worker.is_none());
    assert_eq!(
        host.install_status()
            .unwrap()
            .native
            .operation
            .operation
            .unwrap()
            .state,
        cimmeria_launcher_engine::OperationState::ReconciliationRequired
    );
    assert!(!root.path().join("install").exists());
}

#[tokio::test]
async fn admitted_retry_uses_original_release_offline_or_when_catalog_changes() {
    let root = tempfile::tempdir().unwrap();
    let host = Arc::new(NativeHost::new(root.path().join("state")));
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(root.path().join("install")),
        launcher_summary_consent: false,
    })
    .unwrap();
    let id = Uuid::new_v4();
    let original = fixture_release();
    let digest = original.digest();
    host.store()
        .unwrap()
        .lock()
        .unwrap()
        .admit_install(
            id,
            0,
            1,
            &original,
            cimmeria_launcher_engine::client_setup::login_servers::default_servers(),
        )
        .unwrap();
    let request = InstallCommand::Install {
        schema_version: 1,
        operation_id: id,
        operation_revision: 0,
        preferences_revision: 1,
    };
    let offline = crate::select_install_release(host.clone(), request.clone(), || async {
        Err(CatalogError::Network)
    })
    .await
    .unwrap();
    assert_eq!(offline.digest(), digest);
    let newer = fixture_release_padded(1);
    assert_ne!(newer.digest(), digest);
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let cached = crate::select_install_release(host, request, || {
        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async { Ok(newer) }
    })
    .await
    .unwrap();
    assert_eq!(cached.digest(), digest);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[tokio::test]
async fn same_session_reconciliation_discards_stale_uncertain_worker_result() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(root.path().join("install")),
        launcher_summary_consent: false,
    })
    .unwrap();
    let id = Uuid::new_v4();
    start_install(
        host.store().unwrap(),
        &mut host.worker.lock().unwrap(),
        id,
        0,
        1,
        fixture_release(),
    )
    .unwrap();
    // Current-thread runtime: the worker has not run until this test yields.
    let journal = root.path().join("state/operation.json");
    let original = std::fs::read(&journal).unwrap();
    std::fs::remove_file(&journal).unwrap();
    std::fs::create_dir(&journal).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let result = {
                let worker = host.worker.lock().unwrap();
                let outcome = *worker.as_ref().unwrap().result.borrow();
                outcome
            };
            if result == Some(Outcome::ReconciliationRequired) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    std::fs::remove_dir(&journal).unwrap();
    std::fs::write(&journal, original).unwrap();
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    state.operations_mut().unwrap().mark_uncertain(id).unwrap();
    let intent = state.install_intent().unwrap().unwrap();
    let revision = state.operations().snapshot().revision;
    drop(state);
    // Model prepared content whose journal completion was lost; this fixture
    // tests observation replacement, not extraction or executable compatibility.
    let game = root.path().join("install/game");
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
    std::fs::write(
        root.path().join("install/content-ready.json"),
        serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    let recovered = host
        .install_command(
            InstallCommand::Reconcile {
                schema_version: 1,
                operation_id: id,
                operation_revision: revision,
            },
            None,
        )
        .unwrap();
    assert_eq!(
        recovered.native.operation.operation.unwrap().state,
        cimmeria_launcher_engine::OperationState::Succeeded
    );
    assert!(recovered.outcome.is_none());
    assert!(recovered.progress.is_none());
}
