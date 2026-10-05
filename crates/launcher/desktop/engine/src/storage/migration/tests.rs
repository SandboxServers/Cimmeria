use super::*;
use serde_json::json;

struct Fixture {
    _root: tempfile::TempDir,
    state: PathBuf,
    source: LegacySource,
}
impl Fixture {
    fn new(schema: Option<u32>, telemetry: serde_json::Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        let source = LegacySource {
            launcher_directory: root.path().join("legacy"),
            game_directory: root.path().join("game"),
        };
        std::fs::create_dir(&source.launcher_directory).unwrap();
        std::fs::create_dir(&source.game_directory).unwrap();
        let source = source::canonical(&source).unwrap();
        let mut config = json!({"install_path": "C:\\Games\\SGW", "manifest_url": "https://example.invalid/content", "telemetry": telemetry,
            "login_servers": [{"name":"Custom", "url":"https://example.invalid/login"}],
            "client_patches":{"enabled":false,"dll_override":"custom.dll"},"future_field":42});
        if let Some(schema) = schema {
            config["schema_version"] = schema.into();
        }
        std::fs::write(
            source.launcher_directory.join("launcher-config.json"),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        std::fs::write(source.launcher_directory.join("install.json"), br#"{"schema_version":1,"install_id":"72a8a13b-2a4e-4ea0-b5ba-5ba3cf0a619d","machine_id":"fixture-machine","first_seen_ms":123456,"created_by_launcher_version":"0.8.4"}"#).unwrap();
        std::fs::write(source.game_directory.join("launcher-installed.json"), br#"{"applied_patches":["003","001","003"],"seed_sha256":"historical-claim","seed_adopted":true}"#).unwrap();
        let state = root.path().join("desktop");
        Self {
            _root: root,
            state,
            source,
        }
    }
    fn preview(&self, state: &DesktopState) -> LegacyImport {
        state.preview_legacy_import(&self.source).unwrap()
    }
}
#[test]
fn schemas_preserve_identity_metadata_settings_order_bytes_and_separate_consent() {
    for schema in [None, Some(1), Some(2)] {
        let f = Fixture::new(
            schema,
            json!({"enabled":true,"prompt_answered":true,"auth_url":"http://localhost:8443/api"}),
        );
        let mut state = DesktopState::open(&f.state).unwrap();
        let before = source::load(&f.source).unwrap();
        let preview = f.preview(&state);
        assert!(!preview.config.telemetry.opted_in);
        assert_eq!(
            preview.config.telemetry.auth_url,
            "http://localhost:8443/api"
        );
        assert_eq!(preview.identity.first_seen_ms, 123456);
        assert_eq!(preview.identity.created_by_launcher_version, "0.8.4");
        assert_eq!(preview.config.login_servers[0].name, "Custom");
        assert!(!preview.config.client_patches.enabled);
        assert_eq!(
            preview.config.client_patches.dll_override,
            Some("custom.dll".into())
        );
        let imported = state
            .import_legacy(&f.source, &preview.confirmation, 0)
            .unwrap();
        assert_eq!(imported, preview);
        assert_eq!(imported.ledger.applied_patches, ["003", "001", "003"]);
        assert!(imported.ledger.seed_adopted);
        assert_eq!(source::load(&f.source).unwrap(), before);
        assert!(!state.preferences().launcher_summary_consent);
        assert_eq!(
            state.preferences().install_directory.as_ref(),
            Some(&f.source.game_directory)
        );
        assert!(state.installed_content().unwrap().is_none());
        assert!(state.uninstall_target().unwrap().is_none());
        assert!(state
            .uninstall(uuid::Uuid::new_v4(), 0, imported.identity.install_id, true)
            .is_err());
        assert!(f
            .source
            .game_directory
            .join("launcher-installed.json")
            .exists());
        assert!(!f.state.join("installed-content.json").exists());
        assert_eq!(
            state
                .import_legacy(&f.source, &preview.confirmation, 0)
                .unwrap(),
            preview
        );
        assert_eq!(state.preferences().revision, 1);
        drop(state);
        let state = DesktopState::open(&f.state).unwrap();
        assert_eq!(state.legacy_import().unwrap(), Some(preview));
        let record = state.migration_record().unwrap().unwrap();
        assert_eq!(record.sources, before);
    }
}
#[test]
fn explicit_game_consent_is_preserved_without_enabling_summaries() {
    let f = Fixture::new(Some(2), json!({"opted_in":true}));
    let mut state = DesktopState::open(&f.state).unwrap();
    let preview = f.preview(&state);
    assert!(
        state
            .import_legacy(&f.source, &preview.confirmation, 0)
            .unwrap()
            .config
            .telemetry
            .opted_in
    );
    assert!(!state.preferences().launcher_summary_consent);
}
#[test]
fn source_edits_and_conflicting_imports_do_not_overwrite_identity() {
    let f = Fixture::new(Some(2), json!({}));
    let mut state = DesktopState::open(&f.state).unwrap();
    let preview = f.preview(&state);
    let path = f.source.launcher_directory.join("install.json");
    let original = std::fs::read(&path).unwrap();
    let mut changed = original.clone();
    changed.push(b' ');
    std::fs::write(&path, &changed).unwrap();
    assert_eq!(
        state.import_legacy(&f.source, &preview.confirmation, 0),
        Err(MigrationError::SourceChanged)
    );
    assert!(!f.state.join(NAME).exists());
    std::fs::write(&path, original).unwrap();
    state
        .import_legacy(&f.source, &preview.confirmation, 0)
        .unwrap();
    std::fs::write(&path, changed).unwrap();
    let second = f.preview(&state);
    assert_eq!(
        state.import_legacy(&f.source, &second.confirmation, 1),
        Err(MigrationError::Conflict)
    );
    assert_eq!(state.legacy_import().unwrap(), Some(preview));
}
#[test]
fn missing_corrupt_unsupported_and_oversized_sources_fail_closed() {
    for (name, bytes, expected) in [
        (
            "install.json",
            b"{".to_vec(),
            MigrationError::Storage(StorageError::Corrupt),
        ),
        (
            "launcher-config.json",
            br#"{"schema_version":3,"install_path":"x","manifest_url":"x"}"#.to_vec(),
            MigrationError::UnsupportedSchema,
        ),
        (
            "install.json",
            vec![b' '; 12289],
            MigrationError::Storage(StorageError::TooLarge),
        ),
    ] {
        let f = Fixture::new(None, json!({}));
        let state = DesktopState::open(&f.state).unwrap();
        std::fs::write(f.source.launcher_directory.join(name), &bytes).unwrap();
        assert_eq!(state.preview_legacy_import(&f.source), Err(expected));
        assert_eq!(
            std::fs::read(f.source.launcher_directory.join(name)).unwrap(),
            bytes
        );
        assert!(!f.state.join(NAME).exists());
    }
    let f = Fixture::new(None, json!({}));
    std::fs::remove_file(f.source.launcher_directory.join("install.json")).unwrap();
    let state = DesktopState::open(&f.state).unwrap();
    assert_eq!(
        state.preview_legacy_import(&f.source),
        Err(MigrationError::MissingSource)
    );
    assert!(!f.source.launcher_directory.join("install.json").exists());
}
#[test]
fn desktop_receipts_and_stale_preferences_are_not_overwritten() {
    let f = Fixture::new(None, json!({}));
    let mut state = DesktopState::open(&f.state).unwrap();
    let preview = f.preview(&state);
    state.save_preferences(None, true, 0).unwrap();
    assert_eq!(
        state.import_legacy(&f.source, &preview.confirmation, 0),
        Err(StorageError::StaleRevision.into())
    );
    std::fs::write(f.state.join("installed-content.json"), b"existing").unwrap();
    assert_eq!(
        state.import_legacy(&f.source, &preview.confirmation, 1),
        Err(MigrationError::Conflict)
    );
    assert_eq!(
        std::fs::read(f.state.join("installed-content.json")).unwrap(),
        b"existing"
    );
    assert!(state.preferences().launcher_summary_consent);
}
#[test]
fn failed_initial_replace_is_retryable_and_uncertain_replace_recovers_on_open() {
    for checkpoint in [
        atomic::Checkpoint::BeforeReplace,
        atomic::Checkpoint::AfterReplace,
    ] {
        let f = Fixture::new(None, json!({}));
        let mut state = DesktopState::open(&f.state).unwrap();
        let preview = f.preview(&state);
        let result =
            state.import_legacy_with(&f.source, &preview.confirmation, 0, |root, name, record| {
                atomic::write_with(root, name, record, |at| {
                    if at == checkpoint {
                        Err(std::io::Error::other("fault"))
                    } else {
                        Ok(())
                    }
                })
            });
        assert!(result.is_err());
        assert_eq!(
            state.requires_reopen(),
            checkpoint == atomic::Checkpoint::AfterReplace
        );
        drop(state);
        let mut state = DesktopState::open(&f.state).unwrap();
        if checkpoint == atomic::Checkpoint::BeforeReplace {
            assert!(state.legacy_import().unwrap().is_none());
        } else {
            assert_eq!(state.legacy_import().unwrap(), Some(preview.clone()));
        }
        state
            .import_legacy(&f.source, &preview.confirmation, 0)
            .unwrap();
        assert_eq!(state.preferences().revision, 1);
    }
}
#[test]
fn pending_record_replays_after_preference_failure_without_original_source_bytes() {
    let f = Fixture::new(None, json!({}));
    let mut state = DesktopState::open(&f.state).unwrap();
    let preview = f.preview(&state);
    std::fs::create_dir(f.state.join("preferences.json")).unwrap();
    assert!(state
        .import_legacy(&f.source, &preview.confirmation, 0)
        .is_err());
    assert!(state.requires_reopen());
    std::fs::remove_dir(f.state.join("preferences.json")).unwrap();
    std::fs::write(
        f.source.launcher_directory.join("install.json"),
        b"corrupted later",
    )
    .unwrap();
    drop(state);
    let state = DesktopState::open(&f.state).unwrap();
    assert_eq!(state.legacy_import().unwrap(), Some(preview));
    assert_eq!(state.preferences().revision, 1);
}
#[cfg(unix)]
#[test]
fn actual_legacy_flock_in_a_separate_process_blocks_import() {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    let f = Fixture::new(None, json!({}));
    let mut state = DesktopState::open(&f.state).unwrap();
    let preview = f.preview(&state);
    let mut child = Command::new("python3").arg("-c")
        .arg("import fcntl,sys; f=open(sys.argv[1],'a'); fcntl.flock(f,fcntl.LOCK_EX); print('locked',flush=True); sys.stdin.read()")
        .arg(f.source.launcher_directory.join("launcher.lock"))
        .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut ready = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert_eq!(ready.trim(), "locked");
    let result = state.import_legacy(&f.source, &preview.confirmation, 0);
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
    assert_eq!(result, Err(MigrationError::Busy));
    assert!(!f.state.join(NAME).exists());
    state
        .import_legacy(&f.source, &preview.confirmation, 0)
        .unwrap();
}

#[test]
fn interrupted_completion_after_preferences_is_idempotent_and_conflicts_fail_closed() {
    for conflict in [false, true] {
        let f = Fixture::new(None, json!({}));
        let mut state = DesktopState::open(&f.state).unwrap();
        let preview = f.preview(&state);
        state
            .import_legacy(&f.source, &preview.confirmation, 0)
            .unwrap();
        let mut record = state.migration_record().unwrap().unwrap();
        record.complete = false;
        atomic::write(&f.state, NAME, &record).unwrap();
        if conflict {
            let mut changed = state.preferences().clone();
            changed.revision += 1;
            changed.install_directory = None;
            atomic::write(&f.state, "preferences.json", &changed).unwrap();
        }
        drop(state);
        let reopened = DesktopState::open(&f.state);
        if conflict {
            assert!(matches!(reopened, Err(StorageError::Corrupt)));
            let preferences: Preferences =
                read(&f.state.join("preferences.json")).unwrap().unwrap();
            assert_eq!(preferences.revision, 2);
            assert!(preferences.install_directory.is_none());
        } else {
            let state = reopened.unwrap();
            assert_eq!(state.legacy_import().unwrap(), Some(preview));
            assert_eq!(state.preferences().revision, 1);
        }
    }
}
#[test]
fn old_defaults_preserve_absent_adoption_and_missing_consent() {
    let f = Fixture::new(None, json!({}));
    std::fs::write(
        f.source.game_directory.join("launcher-installed.json"),
        b"{}",
    )
    .unwrap();
    std::fs::write(
        f.source.launcher_directory.join("launcher-config.json"),
        br#"{"install_path":"C:\\SGW","manifest_url":"https://example.invalid"}"#,
    )
    .unwrap();
    let state = DesktopState::open(&f.state).unwrap();
    let preview = f.preview(&state);
    assert!(preview.ledger.applied_patches.is_empty());
    assert!(!preview.ledger.seed_adopted);
    assert_eq!(preview.config.login_servers[0].name, "Cimmeria");
    assert!(preview.config.client_patches.enabled);
    assert!(!preview.config.telemetry.opted_in);
}
#[cfg(unix)]
#[test]
fn symlink_sources_are_rejected_without_touching_target() {
    let f = Fixture::new(None, json!({}));
    let path = f.source.launcher_directory.join("install.json");
    let moved = f.source.launcher_directory.join("identity-original.json");
    std::fs::rename(&path, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &path).unwrap();
    let state = DesktopState::open(&f.state).unwrap();
    assert_eq!(
        state.preview_legacy_import(&f.source),
        Err(StorageError::UnsafeFile.into())
    );
    assert!(moved.exists());
}
