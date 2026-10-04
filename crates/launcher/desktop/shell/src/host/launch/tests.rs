use super::*;
#[test]
fn webview_cannot_supply_executables_hashes_or_resources() {
    for extra in ["path", "resources", "sha256", "environment"] {
        let value = serde_json::json!({"command":"play","schema_version":1,"operation_id":Uuid::new_v4(),"operation_revision":0,"installation_id":Uuid::new_v4(),extra:"untrusted"});
        assert!(serde_json::from_value::<LaunchCommand>(value).is_err());
    }
}
#[test]
fn missing_artifacts_never_admit_or_change_consent() {
    let root = tempfile::tempdir().unwrap();
    let host =
        NativeHost::new(root.path().join("state")).with_launch_resources(root.path().to_path_buf());
    let before = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(!before.resources_available);
    assert!(before.installation_id.is_none());
    assert_eq!(
        host.launch_command(LaunchCommand::Play {
            schema_version: 1,
            operation_id: Uuid::new_v4(),
            operation_revision: 0,
            installation_id: Uuid::new_v4()
        })
        .unwrap_err(),
        JobError::PlatformUnavailable
    );
    drop(host);
    let host = NativeHost::new(root.path().join("state"));
    let after = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert_eq!(after.native.preferences, before.native.preferences);
    assert!(after.native.operation.operation.is_none());
}
#[cfg(target_os = "macos")]
#[test]
fn native_admission_duplicate_lifecycle_and_reopen_are_authoritative() {
    let (_root, host) = super::fixture::fixture();
    let before = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(before.installation_id.is_some());
    let id = Uuid::new_v4();
    let plan = super::fixture::admit(&host, id);
    let duplicate = host
        .launch_command(LaunchCommand::Play {
            schema_version: 1,
            operation_id: id,
            operation_revision: 0,
            installation_id: plan.installation.operation_id,
        })
        .unwrap();
    assert!(duplicate.installation_id.is_none());
    assert!(host.launch_worker.lock().unwrap().is_none());
    super::fixture::observe(&host, "started");
    let started = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(matches!(
        started.observation,
        Some(launch::Observation::ProcessStarted { .. })
    ));
    assert!(started.installation_id.is_none());
    assert_eq!(
        host.launch_command(LaunchCommand::Play {
            schema_version: 1,
            operation_id: Uuid::new_v4(),
            operation_revision: started.native.operation.revision,
            installation_id: plan.installation.operation_id
        })
        .unwrap_err(),
        JobError::Busy
    );
    super::fixture::observe(&host, "exit");
    let exited = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(matches!(
        exited.observation,
        Some(launch::Observation::ProcessExited { early: true, .. })
    ));
    assert!(exited.installation_id.is_some());
    assert_eq!(before.native.preferences, exited.native.preferences);
    super::fixture::admit(&host, Uuid::new_v4());
    super::fixture::observe(&host, "started");
    let path = host.root.clone();
    let resources = host.launch_resources.clone();
    drop(host);
    let mut host = NativeHost::new(path);
    host.launch_resources = resources;
    let reopened = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert_eq!(reopened.observation, Some(launch::Observation::Unknown));
    assert!(reopened.installation_id.is_none());
    assert_eq!(reopened.native.preferences, before.native.preferences);
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "JSON-lines JS UAT fixture, no real game or runtime"]
fn launch_uat_bridge() {
    use std::io::{BufRead, Write};
    let (_root, mut host) = super::fixture::fixture();
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = match value["command"].as_str().unwrap() {
            "too_old" | "development" => {
                host = super::fixture::reopen_identity(host, value["command"] == "too_old");
                host.launch_command(LaunchCommand::Inspect { schema_version: 1 })
            }
            "play" => {
                let id = serde_json::from_value(value["operation_id"].clone()).unwrap();
                super::fixture::admit(&host, id);
                super::fixture::observe(&host, "started");
                host.launch_command(LaunchCommand::Inspect { schema_version: 1 })
            }
            "exit" => {
                super::fixture::observe(&host, "exit");
                host.launch_command(LaunchCommand::Inspect { schema_version: 1 })
            }
            "reopen" => {
                let path = host.root.clone();
                let resources = host.launch_resources.clone();
                drop(host);
                host = NativeHost::new(path);
                host.launch_resources = resources;
                host.launch_command(LaunchCommand::Inspect { schema_version: 1 })
            }
            _ => host.launch_command(serde_json::from_value(value).unwrap()),
        };
        let reply = match result {
            Ok(status) => serde_json::json!({"ok":status}),
            Err(error) => serde_json::json!({"error":error}),
        };
        println!("LAUNCH_UAT {reply}");
        std::io::stdout().flush().unwrap();
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn retained_native_worker_reports_preparation_failure_without_replaying() {
    let (_root, host) = super::fixture::fixture();
    let before = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    let id = Uuid::new_v4();
    let request = || LaunchCommand::Play {
        schema_version: 1,
        operation_id: id,
        operation_revision: before.native.operation.revision,
        installation_id: before.installation_id.unwrap(),
    };
    host.launch_command(request()).unwrap();
    host.launch_command(request()).unwrap();
    assert_eq!(
        host.launch_worker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .operation_id(),
        id
    );
    // No real Wine cache exists in the fixture: the retained worker must persist
    // known NotStarted, not invent process-start or require a live game.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let status = host
                .launch_command(LaunchCommand::Inspect { schema_version: 1 })
                .unwrap();
            if status.observation == Some(launch::Observation::NotStarted) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let after = host.launch_command(request()).unwrap();
    assert_eq!(after.observation, Some(launch::Observation::NotStarted));
    assert_eq!(after.native.preferences, before.native.preferences);
}

#[cfg(target_os = "macos")]
#[test]
fn signed_minimum_inspection_keeps_play_unavailable_without_mutation() {
    let (_root, host) = super::fixture::fixture();
    let before = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    let host = super::fixture::reopen_identity(host, true);
    for _ in 0..3 {
        let status = host
            .launch_command(LaunchCommand::Inspect { schema_version: 1 })
            .unwrap();
        assert!(status.launcher_update_required);
        assert!(status.installation_id.is_none());
        assert_eq!(status.native.operation, before.native.operation);
        assert_eq!(status.native.preferences, before.native.preferences);
    }
    let host = super::fixture::reopen_identity(host, false);
    let status = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(!status.launcher_update_required);
    assert!(status.installation_id.is_some());
}
