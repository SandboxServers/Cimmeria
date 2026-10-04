//! Real retained host dispatch against signed fixture bytes and isolated storage.
use super::*;
use crate::host::held_download;
use cimmeria_launcher_engine::{install_worker::fixtures, update, OperationState};
use std::time::Duration;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

async fn fixture(interrupt: bool) -> (tempfile::TempDir, Arc<NativeHost>, MockServer) {
    let (root, mut host) = super::tests::fixture();
    let server = MockServer::start().await;
    let seed = fixtures::archive(true);
    Mock::given(path("/seed.zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(seed.clone())
                .set_delay(Duration::from_millis(150)),
        )
        .mount(&server)
        .await;
    host.game_update_fixture = Some(TestDispatch {
        url: format!("{}/manifest.json", server.uri()),
        interrupt,
    });
    std::fs::write(
        root.path().join("install/game/custom.txt"),
        b"old modifications",
    )
    .unwrap();
    let revision = host.game_update_status().unwrap().native.operation.revision;
    let check = host.begin_game_update_check(revision).unwrap();
    host.finish_game_update_check(check, fixtures::verified(&seed))
        .unwrap();
    (root, Arc::new(host), server)
}
fn request(host: &NativeHost, confirmed: bool) -> GameUpdateCommand {
    let status = host.game_update_status().unwrap();
    GameUpdateCommand::Apply {
        schema_version: 1,
        offer_id: status.offer.unwrap().id,
        operation_id: Uuid::new_v4(),
        operation_revision: status.native.operation.revision,
        confirmed,
    }
}
async fn observe(host: &NativeHost, expected: OperationState) -> GameUpdateStatus {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let status = host.game_update_status().unwrap();
            let actual = status.native.operation.operation.as_ref().unwrap().state;
            assert!(
                !actual.terminal() || actual == expected,
                "unexpected Update outcome: {status:?}"
            );
            if actual == expected {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("retained Update dispatcher reaches durable outcome")
}
fn plan(host: &NativeHost) -> update::Plan {
    host.store()
        .unwrap()
        .lock()
        .unwrap()
        .update_plan()
        .unwrap()
        .unwrap()
}
async fn maintain(host: Arc<NativeHost>, action: MaintenanceAction) -> GameUpdateStatus {
    let status = host.game_update_status().unwrap();
    let request = GameUpdateCommand::Maintain {
        schema_version: 1,
        action,
        confirmed: true,
        operation_id: status.maintenance.unwrap().operation_id,
        operation_revision: status.native.operation.revision,
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        tokio::task::spawn_blocking(move || host.maintain_game_update(request)),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap()
}
#[tokio::test]
async fn confirmed_apply_retains_handoff_and_duplicate_does_not_download_twice() {
    let (root, host, server) = fixture(false).await;
    let before = host.game_update_status().unwrap().native;
    assert_eq!(
        host.apply_game_update(request(&host, false)).unwrap_err(),
        JobError::RecoveryRequired
    );
    assert_eq!(
        host.game_update_status().unwrap().native.operation,
        before.operation
    );
    let command = request(&host, true);
    host.apply_game_update(command.clone()).unwrap();
    host.apply_game_update(command).unwrap();
    let status = observe(&host, OperationState::Succeeded).await;
    let plan = plan(&host);
    assert_eq!(
        std::fs::read(root.path().join("install/game/later.txt")).unwrap(),
        b"later entry"
    );
    assert_eq!(
        std::fs::read(plan.backup().join("custom.txt")).unwrap(),
        b"old modifications"
    );
    assert!(!root.path().join("install/game/custom.txt").exists());
    assert_eq!(status.native.preferences, before.preferences);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let status = maintain(host.clone(), MaintenanceAction::Cleanup).await;
    assert_eq!(
        status.maintenance.unwrap().backup,
        update::cleanup::BackupStatus::Removed
    );
    assert!(!plan.backup().exists());
    let state_path = host.root.clone();
    drop(host);
    let reopened = NativeHost::new(state_path);
    let status = reopened.game_update_status().unwrap();
    assert_eq!(
        status.maintenance.unwrap().backup,
        update::cleanup::BackupStatus::Removed
    );
    let store = reopened.store().unwrap();
    let installed = store.lock().unwrap().installed_content().unwrap().unwrap();
    assert_eq!(installed.current_release, plan.target);
    assert_eq!(installed.intent, plan.owner);
}
#[tokio::test]
async fn preparation_cancel_preserves_old_tree_and_reports_terminal_outcome() {
    let (root, mut host, _server) = fixture(false).await;
    let download = held_download::HeldDownload::new(fixtures::archive(true));
    Arc::get_mut(&mut host)
        .unwrap()
        .game_update_fixture
        .as_mut()
        .unwrap()
        .url = download.url.clone();
    host.apply_game_update(request(&host, true)).unwrap();
    let id = plan(&host).id;
    host.apply_game_update(GameUpdateCommand::Cancel {
        schema_version: 1,
        operation_id: id,
    })
    .unwrap();
    let status = observe(&host, OperationState::Cancelled).await;
    assert!(status.maintenance.unwrap().discard);
    assert_eq!(
        std::fs::read(root.path().join("install/game/custom.txt")).unwrap(),
        b"old modifications"
    );
    assert!(!plan(&host).backup().exists());
}
#[tokio::test]
async fn interrupted_publication_recovers_without_redownloading() {
    let (root, host, server) = fixture(true).await;
    host.apply_game_update(request(&host, true)).unwrap();
    let status = observe(&host, OperationState::ReconciliationRequired).await;
    assert!(status.maintenance.unwrap().recovery);
    let status = maintain(host.clone(), MaintenanceAction::Recover).await;
    assert_eq!(
        status.native.operation.operation.unwrap().state,
        OperationState::Succeeded
    );
    assert_eq!(
        std::fs::read(root.path().join("install/game/later.txt")).unwrap(),
        b"later entry"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// Actual host, signed archive HTTP download and filesystem publication for JS UAT.
#[test]
#[ignore = "JSON-lines real-host bridge for game-update-apply-native-uat.mjs"]
fn game_update_apply_uat_bridge() {
    use std::io::{BufRead, Write};
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_root, mut host, _server) = runtime.block_on(fixture(false));
    let _entered = runtime.enter();
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = if value["command"] == "wait" {
            Ok(runtime.block_on(observe(&host, OperationState::Succeeded)))
        } else if value["command"] == "reopen" {
            let path = host.root.clone();
            drop(host);
            host = Arc::new(NativeHost::new(path));
            host.game_update_status()
        } else {
            let request: GameUpdateCommand = serde_json::from_value(value).unwrap();
            match request {
                GameUpdateCommand::Inspect { .. } => host.game_update_status(),
                request @ (GameUpdateCommand::Apply { .. } | GameUpdateCommand::Cancel { .. }) => {
                    host.apply_game_update(request)
                }
                request @ (GameUpdateCommand::Maintain { .. }
                | GameUpdateCommand::Rollback { .. }) => host.maintain_game_update(request),
                GameUpdateCommand::Check {
                    operation_revision, ..
                } => host
                    .begin_game_update_check(operation_revision)
                    .and_then(|check| {
                        host.finish_game_update_check(
                            check,
                            fixtures::verified(&fixtures::archive(true)),
                        )
                    }),
            }
        };
        let reply = match result {
            Ok(status) => serde_json::json!({"ok":status}),
            Err(error) => serde_json::json!({"error":error}),
        };
        println!("GAME_UPDATE_APPLY_UAT {reply}");
        std::io::stdout().flush().unwrap();
    }
}
