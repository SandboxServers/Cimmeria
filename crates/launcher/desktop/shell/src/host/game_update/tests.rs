use super::super::install::tests::fixture_release_padded;
use super::*;
use cimmeria_launcher_engine::OperationState;

pub(super) fn fixture() -> (tempfile::TempDir, NativeHost) {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    let intent = state
        .admit_install(
            id,
            0,
            1,
            &fixture_release_padded(0),
            cimmeria_launcher_engine::client_setup::login_servers::default_servers(),
        )
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
    drop(state);
    (root, host)
}

#[test]
fn signed_offer_keeps_installed_directory_and_does_not_mutate_game_or_preferences() {
    let (root, host) = fixture();
    let before = host.game_update_status().unwrap();
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 1,
        install_directory: Some(root.path().join("different")),
        launcher_summary_consent: true,
    })
    .unwrap();
    let check = host
        .begin_game_update_check(before.native.operation.revision)
        .unwrap();
    let status = host
        .finish_game_update_check(check, fixture_release_padded(1))
        .unwrap();
    let offer = status.offer.unwrap();
    assert_eq!(
        offer.directory,
        root.path().join("install").canonicalize().unwrap()
    );
    assert_ne!(offer.current_digest, offer.target_digest);
    assert_eq!(status.native.operation, before.native.operation);
    assert!(status.native.preferences.launcher_summary_consent);
    assert_eq!(
        std::fs::read_dir(root.path().join("install/game"))
            .unwrap()
            .count(),
        0
    );
    let path = host.root.clone();
    drop(host);
    let reopened = NativeHost::new(path).game_update_status().unwrap();
    assert!(!reopened.checked);
    assert!(reopened.offer.is_none());
}

#[test]
fn concurrent_checks_and_intervening_operation_cannot_publish_stale_offer() {
    let (_root, host) = fixture();
    let revision = host.game_update_status().unwrap().native.operation.revision;
    let first = host.begin_game_update_check(revision).unwrap();
    let second = host.begin_game_update_check(revision).unwrap();
    assert_eq!(
        host.finish_game_update_check(first, fixture_release_padded(1))
            .unwrap_err(),
        JobError::StaleRevision
    );
    let status = host
        .finish_game_update_check(second, fixture_release_padded(0))
        .unwrap();
    assert!(status.checked && status.offer.is_none());
    let stale = host.begin_game_update_check(revision).unwrap();
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    let id = Uuid::new_v4();
    state
        .operations_mut()
        .unwrap()
        .begin(
            id,
            cimmeria_launcher_engine::OperationKind::Repair,
            [0; 32],
            revision,
        )
        .unwrap();
    drop(state);
    assert_eq!(
        host.finish_game_update_check(stale, fixture_release_padded(1))
            .unwrap_err(),
        JobError::Busy
    );
    assert!(!host.game_update_status().unwrap().can_check);
}

#[test]
fn ipc_cannot_supply_release_or_installation_paths() {
    for value in [
        r#"{"command":"check","schema_version":1,"operation_revision":0,"url":"https://other"}"#,
        r#"{"command":"check","schema_version":1,"operation_revision":0,"directory":"/other"}"#,
        r#"{"command":"check","schema_version":1,"operation_revision":0,"manifest":{}}"#,
    ] {
        assert!(serde_json::from_str::<GameUpdateCommand>(value).is_err());
    }
    assert_eq!(
        GameUpdateCommand::Inspect { schema_version: 2 }.validate(),
        Err(JobError::UnsupportedSchema)
    );
}

/// Uses actual signed release verification, installed ownership and store reopen.
/// The fixture supplies the catalog response; no network or game worker runs.
#[test]
#[ignore = "Driven by game-update-native-uat.mjs over stdin/stdout"]
fn game_update_native_uat_bridge() {
    use std::io::{BufRead, Write};
    let (_root, mut host) = fixture();
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = if value["command"] == "reopen" {
            let path = host.root.clone();
            drop(host);
            host = NativeHost::new(path);
            host.game_update_status()
        } else {
            let request: GameUpdateCommand = serde_json::from_value(value).unwrap();
            match request {
                GameUpdateCommand::Inspect { .. } => host.game_update_status(),
                request @ (GameUpdateCommand::Maintain { .. }
                | GameUpdateCommand::Rollback { .. }) => host.maintain_game_update(request),
                request @ (GameUpdateCommand::Apply { .. } | GameUpdateCommand::Cancel { .. }) => {
                    host.apply_game_update(request)
                }
                GameUpdateCommand::Check {
                    operation_revision, ..
                } => host
                    .begin_game_update_check(operation_revision)
                    .and_then(|check| {
                        host.finish_game_update_check(check, fixture_release_padded(1))
                    }),
            }
        };
        let reply = match result {
            Ok(status) => serde_json::json!({"ok": status}),
            Err(error) => serde_json::json!({"error": error}),
        };
        println!("GAME_UPDATE_UAT {reply}");
        std::io::stdout().flush().unwrap();
    }
}
