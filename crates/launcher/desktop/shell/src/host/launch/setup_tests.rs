//! Setup visibility is not an admission credential; real native fixture state only.
use super::*;

#[test]
fn setup_survives_a_required_launcher_update_without_offering_play() {
    let (_root, host) = fixture::fixture();
    let before = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(before.setup_complete);
    assert!(before.installation_id.is_some());
    let host = fixture::reopen_identity(host, true);
    let blocked = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(blocked.launcher_update_required);
    assert!(blocked.installation_id.is_none());
    assert!(blocked.setup_complete);
    assert_eq!(blocked.native.preferences, before.native.preferences);
    assert_eq!(blocked.native.operation, before.native.operation);
}

#[test]
fn setup_survives_a_running_launch_but_unknown_state_retains_inspection() {
    let (_root, host) = fixture::fixture();
    fixture::admit(&host, Uuid::new_v4());
    fixture::observe(&host, "started");
    let running = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(running.installation_id.is_none());
    assert!(running.setup_complete);
    assert!(matches!(
        running.observation,
        Some(launch::Observation::ProcessStarted { .. })
    ));
    fixture::observe(&host, "unknown");
    let unknown = host
        .launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert!(unknown.installation_id.is_none());
    assert!(!unknown.setup_complete);
}

#[test]
fn setup_is_not_complete_when_content_or_bundled_resources_are_missing() {
    let (_root, host) = fixture::fixture();
    let game = host
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .unwrap()
        .intent
        .content_root();
    std::fs::remove_file(game.join("Working/Binaries/SGW.exe")).unwrap();
    assert!(
        !host
            .launch_command(LaunchCommand::Inspect { schema_version: 1 })
            .unwrap()
            .setup_complete
    );
    let (_root, mut host) = fixture::fixture();
    host.launch_resources = None;
    assert!(
        !host
            .launch_command(LaunchCommand::Inspect { schema_version: 1 })
            .unwrap()
            .setup_complete
    );
}
