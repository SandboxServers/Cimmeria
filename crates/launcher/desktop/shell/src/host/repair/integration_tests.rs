//! Real host dispatch with a signed inert ZIP and loopback transport only.
use super::*;
use cimmeria_launcher_engine::{install_worker::fixtures, repair::cleanup::BackupStatus};
use std::time::Duration;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

async fn fixture(
    missing: bool,
    interrupt: bool,
) -> (tempfile::TempDir, Arc<NativeHost>, Uuid, MockServer) {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    let seed = fixtures::archive(true);
    let release = fixtures::verified(&seed);
    Mock::given(path("/seed.zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(seed)
                .set_delay(Duration::from_millis(150)),
        )
        .mount(&server)
        .await;
    let mut host = NativeHost::new(root.path().join("state"));
    host.repair_fixture = Some(TestDispatch {
        url: format!("{}/manifest.json", server.uri()),
        interrupt,
    });
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    state
        .save_preferences(Some(root.path().join("installation")), true, 0)
        .unwrap();
    let installation = Uuid::new_v4();
    let intent = state
        .admit_install(
            installation,
            0,
            1,
            &release,
            cimmeria_launcher_engine::client_setup::login_servers::default_servers(),
        )
        .unwrap()
        .intent;
    std::fs::create_dir_all(&intent.destination).unwrap();
    if !missing {
        std::fs::create_dir_all(intent.destination.join("game")).unwrap();
        std::fs::write(
            intent.destination.join("game/old.txt"),
            b"old modifications",
        )
        .unwrap();
    }
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
        .observe(installation, OperationState::Running)
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(installation, OperationState::Succeeded)
        .unwrap();
    state.installed_content().unwrap();
    drop(state);
    (root, Arc::new(host), installation, server)
}
fn dispatch(host: &NativeHost, installation: Uuid) -> (Uuid, InstallCommand) {
    let id = Uuid::new_v4();
    let request = InstallCommand::Repair {
        schema_version: 1,
        operation_id: id,
        operation_revision: host.install_status().unwrap().native.operation.revision,
        installation_id: installation,
        confirmed: true,
    };
    host.install_command(request.clone(), None).unwrap();
    (id, request)
}
async fn observe(host: &NativeHost, expected: OperationState) -> InstallStatus {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let status = host.install_status().unwrap();
            if status.native.operation.operation.as_ref().unwrap().state == expected {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("production retained repair coordinator reaches expected durable result")
}
async fn action(host: Arc<NativeHost>, request: InstallCommand) -> InstallStatus {
    tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || host.install_command(request, None)),
    )
    .await
    .expect("blocking recovery/cleanup finishes")
    .unwrap()
    .unwrap()
}
#[tokio::test]
async fn retained_host_handoff_commits_signed_seed_and_reports_current_backup_after_cleanup() {
    for missing in [false, true] {
        let (_root, host, installation, server) = fixture(missing, false).await;
        let preferences = host.install_status().unwrap().native.preferences;
        let (id, duplicate) = dispatch(&host, installation);
        host.install_command(duplicate, None).unwrap();
        let status = observe(&host, OperationState::Succeeded).await;
        let plan = host
            .store()
            .unwrap()
            .lock()
            .unwrap()
            .repair_plan()
            .unwrap()
            .unwrap();
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"later entry"
        );
        assert_eq!(status.repair.cleanup, !missing);
        assert_eq!(
            status.repair.backup,
            if missing {
                BackupStatus::NotRetained
            } else {
                BackupStatus::Retained
            }
        );
        assert_eq!(status.native.preferences, preferences);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        if !missing {
            assert_eq!(
                std::fs::read(plan.backup().join("old.txt")).unwrap(),
                b"old modifications"
            );
            let status = action(
                host.clone(),
                InstallCommand::CleanupRepair {
                    schema_version: 1,
                    operation_id: id,
                    operation_revision: status.native.operation.revision,
                    confirmed: true,
                },
            )
            .await;
            assert!(!status.repair.cleanup);
            assert_eq!(status.repair.backup, BackupStatus::Removed);
            assert!(!plan.backup().exists());
            assert_eq!(status.native.preferences, preferences);
            let path = host.root.clone();
            drop(host);
            let host = NativeHost::new(path);
            let status = host.install_status().unwrap();
            assert!(!status.repair.cleanup);
            assert_eq!(status.repair.backup, BackupStatus::Removed);
        }
    }
}
#[tokio::test]
async fn host_progress_and_precommit_cancel_preserve_original_without_backup() {
    let (_root, host, installation, server) = fixture(false, false).await;
    let (id, _) = dispatch(&host, installation);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let status = host.install_status().unwrap();
            if status.progress.is_some() && !server.received_requests().await.unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("real preparation publishes progress while downloading");
    host.install_command(
        InstallCommand::Cancel {
            schema_version: 1,
            operation_id: id,
        },
        None,
    )
    .unwrap();
    observe(&host, OperationState::Cancelled).await;
    let plan = host
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .repair_plan()
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/old.txt")).unwrap(),
        b"old modifications"
    );
    assert!(!plan.backup().exists());
    assert!(!plan
        .installation
        .destination
        .join("game/later.txt")
        .exists());
}
#[tokio::test]
async fn host_recovers_real_interrupted_promotion_then_cleans_current_backup() {
    let (_root, host, installation, _server) = fixture(false, true).await;
    let (id, _) = dispatch(&host, installation);
    let status = observe(&host, OperationState::ReconciliationRequired).await;
    let status = action(
        host.clone(),
        InstallCommand::RecoverRepair {
            schema_version: 1,
            operation_id: id,
            operation_revision: status.native.operation.revision,
            confirmed: true,
        },
    )
    .await;
    assert_eq!(
        status.native.operation.operation.as_ref().unwrap().state,
        OperationState::Succeeded
    );
    assert!(status.repair.cleanup);
    let status = action(
        host.clone(),
        InstallCommand::CleanupRepair {
            schema_version: 1,
            operation_id: id,
            operation_revision: status.native.operation.revision,
            confirmed: true,
        },
    )
    .await;
    assert!(!status.repair.cleanup);
    assert_eq!(status.repair.backup, BackupStatus::Removed);
}

/// JS observes real preparation/retained commit/cleanup; commands are never intercepted.
#[test]
#[ignore = "JSON-lines real-host bridge for repair-native-uat.mjs"]
fn repair_native_uat_bridge() {
    use std::io::{BufRead, Write};
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_root, mut host, _, _server) = runtime.block_on(fixture(false, false));
    let _entered = runtime.enter();
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = if value["command"] == "wait" {
            Ok(runtime.block_on(observe(&host, OperationState::Succeeded)))
        } else if value["command"] == "reopen" {
            let path = host.root.clone();
            drop(host);
            host = Arc::new(NativeHost::new(path));
            host.install_status()
        } else {
            host.install_command(serde_json::from_value(value).unwrap(), None)
        };
        let reply = match result {
            Ok(status) => serde_json::json!({"ok": status}),
            Err(error) => serde_json::json!({"error": error}),
        };
        println!("REPAIR_NATIVE_UAT {reply}");
        std::io::stdout().flush().unwrap();
    }
}
