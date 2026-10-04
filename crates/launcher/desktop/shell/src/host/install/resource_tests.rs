use super::*;
const EMPTY_SHA: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
fn fixture() -> (tempfile::TempDir, NativeHost, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join("helper.exe");
    std::fs::write(&helper, b"").unwrap();
    let mut host = NativeHost::new(root.path().join("state"));
    host.helper = Some(
        cimmeria_launcher_engine::mac_wine::HelperResource::open(helper.clone(), EMPTY_SHA)
            .unwrap(),
    );
    (root, host, helper)
}
#[test]
fn changed_resource_rejects_before_opening_state() {
    let (root, host, helper) = fixture();
    assert!(host.require_install_support().is_ok());
    std::fs::write(helper, b"changed").unwrap();
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
    assert!(!root.path().join("state").exists());
}
#[test]
fn wine_recovery_capabilities_and_commands_refuse_native_replay() {
    let (root, host, _) = fixture();
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(root.path().join("install")),
        launcher_summary_consent: false,
    })
    .unwrap();
    let release = super::tests::fixture_release();
    let id = Uuid::new_v4();
    let state = host.store().unwrap();
    {
        let mut owner = state.lock().unwrap();
        owner
            .admit_install_backend(cimmeria_launcher_engine::AdmissionRequest {
                id,
                operation_revision: 0,
                preferences_revision: 1,
                release: &release,
                login_servers:
                    cimmeria_launcher_engine::client_setup::login_servers::default_servers(),
                backend: host.helper.as_ref().unwrap().backend(),
            })
            .unwrap();
        owner.operations_mut().unwrap().mark_uncertain(id).unwrap();
    }
    let status = host.install_status().unwrap();
    assert!(status.install_supported);
    assert!(!status.can_resume);
    assert!(!status.can_reconcile);
    let revision = status.native.operation.revision;
    assert!(host
        .install_command(
            InstallCommand::Resume {
                schema_version: 1,
                operation_id: id,
                operation_revision: revision
            },
            None
        )
        .is_err());
    assert!(host
        .install_command(
            InstallCommand::Reconcile {
                schema_version: 1,
                operation_id: id,
                operation_revision: revision
            },
            None
        )
        .is_err());
    assert_eq!(
        host.install_status().unwrap().native.operation.revision,
        revision
    );
}

#[tokio::test]
#[ignore = "requires a staged Windows-native helper and matching compile-time digest; no window opened"]
async fn packaged_resource_admits_wine_and_retains_cancellation() {
    let resources = PathBuf::from(
        std::env::var_os("CIMMERIA_TEST_RESOURCE_DIR").expect("set staged resources directory"),
    );
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state")).with_bundled_helper(resources);
    host.require_install_support()
        .expect("compiled digest must match staged helper");
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: Some(root.path().join("install")),
        launcher_summary_consent: false,
    })
    .unwrap();
    let id = Uuid::new_v4();
    let status = host
        .install_command(
            InstallCommand::Install {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                preferences_revision: 1,
            },
            Some(super::tests::fixture_release()),
        )
        .unwrap();
    assert!(status.install_supported);
    assert!(!status.can_resume);
    assert!(!status.can_reconcile);
    host.install_command(
        InstallCommand::Cancel {
            schema_version: 1,
            operation_id: id,
        },
        None,
    )
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let status = host.install_status().unwrap();
            if status.outcome.is_some() {
                assert_eq!(status.outcome, Some(Outcome::Cancelled));
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let state = host.store().unwrap();
    let owner = state.lock().unwrap();
    assert!(matches!(
        owner.install_intent().unwrap().unwrap().backend,
        cimmeria_launcher_engine::ExtractionBackend::Wine { .. }
    ));
    assert!(owner.helper_record(id).unwrap().is_none());
    assert!(!root.path().join("state/runtimes").exists());
    assert!(!owner.preferences().launcher_summary_consent);
}
