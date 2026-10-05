use super::*;
use std::fs;
fn fixture() -> (tempfile::TempDir, NativeHost, LegacySource) {
    let root = tempfile::tempdir().unwrap();
    let source = LegacySource {
        launcher_directory: root.path().join("legacy"),
        game_directory: root.path().join("game"),
    };
    fs::create_dir(&source.launcher_directory).unwrap();
    fs::create_dir(&source.game_directory).unwrap();
    fs::write(source.launcher_directory.join("launcher-config.json"), r#"{"schema_version":2,"install_path":"C:\\Legacy","manifest_url":"https://example.invalid/manifest","telemetry":{"enabled":true,"opted_in":false},"client_patches":{"enabled":false,"dll_override":"C:\\custom.dll"}}"#).unwrap();
    fs::write(source.launcher_directory.join("install.json"), r#"{"schema_version":1,"install_id":"12345678-1234-4234-8234-123456789abc","machine_id":"fixture-machine","first_seen_ms":12,"created_by_launcher_version":"0.9"}"#).unwrap();
    fs::write(
        source.game_directory.join("launcher-installed.json"),
        r#"{"applied_patches":["two","one","two"],"seed_adopted":true}"#,
    )
    .unwrap();
    let host = NativeHost::new(root.path().join("state"));
    (root, host, source)
}
fn confirm(p: &Preview) -> MigrationCommand {
    MigrationCommand::Confirm {
        schema_version: 1,
        confirmation: p.imported.confirmation.clone(),
        preferences_revision: p.preferences_revision,
        confirmed: true,
    }
}
#[test]
fn native_preview_confirmation_reopen_preserves_identity_without_ownership() {
    let (_root, host, source) = fixture();
    let preview = host.preview_migration(source.clone()).unwrap();
    assert!(preview.imported.is_none());
    assert!(!host.root.join("legacy-import.json").exists());
    let result = host
        .migration_command(confirm(preview.preview.as_ref().unwrap()))
        .unwrap();
    assert!(result.preview.is_none());
    assert_eq!(
        result.imported.as_ref(),
        Some(&preview.preview.as_ref().unwrap().imported)
    );
    assert_eq!(
        result.native.preferences.install_directory,
        Some(source.game_directory.canonicalize().unwrap())
    );
    assert!(!result.native.preferences.launcher_summary_consent);
    assert!(!host.root.join("installed-content.json").exists());
    assert!(
        host.store()
            .unwrap()
            .lock()
            .unwrap()
            .legacy_import()
            .unwrap()
            .unwrap()
            .ledger
            .seed_adopted
    );
    let path = host.root.clone();
    drop(host);
    let reopened = NativeHost::new(path);
    let saved = reopened
        .migration_command(MigrationCommand::Inspect { schema_version: 1 })
        .unwrap();
    assert_eq!(saved.imported, result.imported);
    assert_eq!(saved.native.preferences, result.native.preferences);
    assert!(saved.preview.is_none());
    assert!(reopened.install_status().unwrap().uninstall.is_none());
    assert!(serde_json::to_value(
        reopened
            .launch_command(super::super::LaunchCommand::Inspect { schema_version: 1 })
            .unwrap()
    )
    .unwrap()["installation_id"]
        .is_null());
}
#[test]
fn confirmation_requires_native_preview_and_rejects_changed_sources() {
    let (_root, host, source) = fixture();
    assert!(serde_json::from_value::<MigrationCommand>(serde_json::json!({"command":"inspect","schema_version":1,"source":{"launcher_directory":"/arbitrary"}})).is_err());
    let preview = host
        .preview_migration(source.clone())
        .unwrap()
        .preview
        .unwrap();
    host.migration_command(MigrationCommand::Dismiss { schema_version: 1 })
        .unwrap();
    assert!(matches!(
        host.migration_command(confirm(&preview)),
        Err(MigrationError::SourceChanged)
    ));
    let preview = host
        .preview_migration(source.clone())
        .unwrap()
        .preview
        .unwrap();
    fs::write(source.game_directory.join("launcher-installed.json"), "{}").unwrap();
    assert!(matches!(
        host.migration_command(confirm(&preview)),
        Err(MigrationError::SourceChanged)
    ));
    assert!(!host.root.join("legacy-import.json").exists());
}
#[test]
fn unconfirmed_and_duplicate_requests_cannot_import_again() {
    let (_root, host, source) = fixture();
    let preview = host
        .preview_migration(source.clone())
        .unwrap()
        .preview
        .unwrap();
    let mut request = confirm(&preview);
    if let MigrationCommand::Confirm { confirmed, .. } = &mut request {
        *confirmed = false;
    }
    assert!(matches!(
        host.migration_command(request),
        Err(MigrationError::SourceChanged)
    ));
    assert!(!host.root.join("legacy-import.json").exists());
    let result = host.migration_command(confirm(&preview)).unwrap();
    assert!(matches!(
        host.migration_command(confirm(&preview)),
        Err(MigrationError::SourceChanged)
    ));
    let again = host.preview_migration(source).unwrap().preview.unwrap();
    let repeated = host.migration_command(confirm(&again)).unwrap();
    assert_eq!(result.imported, repeated.imported);
    assert_eq!(result.native.preferences, repeated.native.preferences);
}
#[test]
fn changed_preferences_cannot_be_overwritten_by_a_previous_preview() {
    let (_root, host, source) = fixture();
    let preview = host.preview_migration(source).unwrap().preview.unwrap();
    host.dispatch(NativeCommand::SavePreferences {
        schema_version: 1,
        expected_revision: 0,
        install_directory: None,
        launcher_summary_consent: true,
    })
    .unwrap();
    assert!(matches!(
        host.migration_command(confirm(&preview)),
        Err(MigrationError::Storage(StorageError::StaleRevision))
    ));
    assert!(!host.root.join("legacy-import.json").exists());
}

/// The JS harness selects fixture folders natively; production IPC accepts no paths.
#[test]
#[ignore = "JSON-lines production-host bridge for migration-native-uat.mjs"]
fn migration_native_uat_bridge() {
    use std::io::{BufRead, Write};
    let (_root, mut host, source) = fixture();
    let original = fs::read(source.launcher_directory.join("launcher-config.json")).unwrap();
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = match value["command"].as_str().unwrap() {
            "choose" => host.preview_migration(source.clone()),
            "reopen" => {
                let path = host.root.clone();
                drop(host);
                host = NativeHost::new(path);
                host.migration_command(MigrationCommand::Inspect { schema_version: 1 })
            }
            _ => host.migration_command(serde_json::from_value(value).unwrap()),
        };
        let reply = match result {
            Ok(status) => serde_json::json!({"ok":status}),
            Err(error) => serde_json::json!({"error":error}),
        };
        println!("MIGRATION_NATIVE_UAT {reply}");
        std::io::stdout().flush().unwrap();
    }
    assert_eq!(
        fs::read(source.launcher_directory.join("launcher-config.json")).unwrap(),
        original
    );
    assert!(!host.root.join("installed-content.json").exists());
}
