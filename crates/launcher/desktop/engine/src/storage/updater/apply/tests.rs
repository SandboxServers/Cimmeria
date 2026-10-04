use super::super::{
    tests::{available, prepare},
    Offer,
};
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Cursor;

fn fixture(root: &Path) -> (Config, Offer, Vec<u8>, InstalledTarget) {
    let installed = root.join("Launcher.app");
    make_bundle(&installed, "1.0.0", b"#!/bin/sh\nexit 0\n");
    let next = root.join("next/Launcher.app");
    make_bundle(
        &next,
        "1.1.0",
        b"#!/bin/sh\nprintf started > \"$(dirname \"$0\")/../../../started\"\n",
    );
    let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    tar.append_dir_all("Launcher.app", &next).unwrap();
    let bytes = tar.into_inner().unwrap().finish().unwrap();
    let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let signature = minisign::sign(
        Some(&keys.pk),
        &keys.sk,
        Cursor::new(&bytes),
        Some("version:1.1.0"),
        None,
    )
    .unwrap();
    let config = Config::new(
        "https://updates.example.test/feed",
        &STANDARD.encode(keys.pk.to_box().unwrap().to_string()),
        "1.0.0",
        "darwin-aarch64",
        vec!["updates.example.test".into()],
    )
    .unwrap();
    let offer = Offer {
        id: Uuid::new_v4(),
        version: "1.1.0".into(),
        notes: "Fixture bundle".into(),
        url: "https://updates.example.test/Launcher.app.tar.gz".into(),
        signature: STANDARD.encode(signature.to_string()),
    };
    let target = InstalledTarget(Target::Mac {
        bundle: installed,
        executable: "Contents/MacOS/launcher".into(),
        identifier: "test.fixture.launcher".into(),
    });
    (config, offer, bytes, target)
}
fn make_bundle(path: &Path, version: &str, code: &[u8]) {
    fs::create_dir_all(path.join("Contents/MacOS")).unwrap();
    let dict = plist::Dictionary::from_iter([
        (
            "CFBundleIdentifier",
            plist::Value::String("test.fixture.launcher".into()),
        ),
        (
            "CFBundleExecutable",
            plist::Value::String("launcher".into()),
        ),
        (
            "CFBundleShortVersionString",
            plist::Value::String(version.into()),
        ),
    ]);
    plist::Value::Dictionary(dict)
        .to_file_xml(path.join("Contents/Info.plist"))
        .unwrap();
    let exe = path.join("Contents/MacOS/launcher");
    fs::write(&exe, code).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(exe, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
fn ready(state: &mut DesktopState, config: &Config, offer: Offer, bytes: &[u8]) -> Snapshot {
    let offered = available(state, config, offer);
    prepare(state, config, offered, bytes)
}
#[test]
#[cfg(target_os = "macos")]
fn real_bundle_swap_spawns_replacement_and_only_compiled_reopen_acknowledges() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    let status = state
        .apply_launcher_update(
            Some(&config),
            &target,
            ready.offer.unwrap().id,
            ready.revision,
            0,
        )
        .unwrap();
    assert_eq!(status.phase, Phase::RestartRequired);
    assert!(matches!(
        state.operations_mut(),
        Err(super::super::StorageError::Busy)
    ));
    for _ in 0..100 {
        if fs::read(root.join("started")).ok().as_deref() == Some(b"started") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(fs::read(root.join("started")).unwrap(), b"started");
    drop(state);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&target, "1.0.0").unwrap();
    assert_eq!(
        state.launcher_update_snapshot(Some(&config)).unwrap().phase,
        Phase::ReconciliationRequired
    );
    assert!(state.ensure_updater_idle().is_err());
    state.reconcile_launcher_update(&target, "1.1.0").unwrap();
    assert_eq!(
        state.launcher_update_snapshot(Some(&config)).unwrap().phase,
        Phase::Installed
    );
    assert!(state.ensure_updater_idle().is_ok());
    assert!(!fs::read_dir(&root).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".cimmeria-")));
}
#[test]
fn failed_replacement_spawn_restores_old_bundle_and_releases_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let before = bundle::fingerprint(&root.join("Launcher.app")).unwrap();
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    assert_eq!(
        state
            .apply_update_with(
                Some(&config),
                &target,
                ready.offer.unwrap().id,
                ready.revision,
                0,
                |_, _| Err(Error::Spawn)
            )
            .unwrap_err(),
        Error::Spawn
    );
    assert_eq!(
        bundle::fingerprint(&root.join("Launcher.app")).unwrap(),
        before
    );
    assert!(state.ensure_updater_idle().is_ok());
    assert_eq!(
        state.launcher_update_snapshot(Some(&config)).unwrap().phase,
        Phase::Failed
    );
}
#[test]
fn saved_payload_tamper_is_rejected_before_any_replacement_or_process() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let before = bundle::fingerprint(&root.join("Launcher.app")).unwrap();
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    fs::write(root.join("state").join(super::super::ARTIFACT), b"tampered").unwrap();
    assert_eq!(
        state
            .apply_update_with(
                Some(&config),
                &target,
                ready.offer.unwrap().id,
                ready.revision,
                0,
                |_, _| panic!("unverified executable")
            )
            .unwrap_err(),
        Error::Signature
    );
    assert_eq!(
        bundle::fingerprint(&root.join("Launcher.app")).unwrap(),
        before
    );
    assert!(!fs::read_dir(&root).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".cimmeria-")));
}
#[test]
fn interruption_after_old_rename_restores_original_without_spawning() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let _ready = ready(&mut state, &config, offer, &bytes);
    let owner = Uuid::new_v4();
    let installed = root.join("Launcher.app");
    let mut record = state.update_record().unwrap();
    record.phase = Phase::Installing;
    record.owner = Some(owner);
    record.apply = Some(Attempt {
        target: target.0.clone(),
        original: bundle::fingerprint(&installed).unwrap(),
        replacement: None,
        staged_bundle: None,
        handoff: false,
    });
    state.save_update(record.clone()).unwrap();
    let (_, backup) = record
        .apply
        .as_ref()
        .unwrap()
        .paths(owner, state.state_root())
        .unwrap();
    rename_new(&installed, &backup).unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    assert!(state.ensure_updater_idle().is_err());
    state.reconcile_launcher_update(&target, "1.0.0").unwrap();
    assert_eq!(
        bundle::fingerprint(&installed).unwrap(),
        record.apply.unwrap().original
    );
    assert_eq!(
        state
            .launcher_update_snapshot(Some(&config))
            .unwrap()
            .failure,
        Some(Error::Interrupted)
    );
    assert!(!backup.exists());
}
#[test]
fn package_identity_mismatch_never_moves_installed_bundle() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, mut target) = fixture(&root);
    if let Target::Mac { identifier, .. } = &mut target.0 {
        *identifier = "wrong".into();
    }
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    assert_eq!(
        state
            .apply_update_with(
                Some(&config),
                &target,
                ready.offer.unwrap().id,
                ready.revision,
                0,
                |_, _| panic!()
            )
            .unwrap_err(),
        Error::Target
    );
    assert!(state.ensure_updater_idle().is_ok());
}

#[test]
fn duplicate_apply_and_wrong_target_reconciliation_cannot_repeat_effects() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    let id = ready.offer.unwrap().id;
    state
        .apply_update_with(Some(&config), &target, id, ready.revision, 0, |_, _| Ok(()))
        .unwrap();
    assert_eq!(
        state
            .apply_update_with(
                Some(&config),
                &target,
                id,
                ready.revision,
                0,
                |_, _| panic!("duplicate spawn")
            )
            .unwrap_err(),
        Error::Busy
    );
    let mut wrong = target.clone();
    if let Target::Mac { bundle, .. } = &mut wrong.0 {
        *bundle = root.join("other.app");
    }
    assert_eq!(
        state.reconcile_launcher_update(&wrong, "1.1.0"),
        Err(Error::Target)
    );
    assert!(state.ensure_updater_idle().is_err());
}

#[test]
fn windows_installer_handoff_stays_owned_until_compiled_version_acknowledges() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (mut config, mut offer, bytes, _) = fixture(&root);
    config.platform = "windows-x86_64".into();
    offer.url = "https://updates.example.test/launcher.exe".into();
    let exe = root.join("launcher.exe");
    fs::write(&exe, b"old native executable").unwrap();
    let target = InstalledTarget(Target::Windows {
        executable: exe.clone(),
    });
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    let mut seen = false;
    let status = state
        .apply_update_with(
            Some(&config),
            &target,
            ready.offer.unwrap().id,
            ready.revision,
            0,
            |path, args| {
                assert_eq!(fs::read(path).unwrap(), bytes);
                assert_eq!(
                    args,
                    &[
                        "/P".to_owned(),
                        "/R".to_owned(),
                        "/UPDATE".to_owned(),
                        format!("/D={}", root.display())
                    ]
                );
                seen = true;
                Ok(())
            },
        )
        .unwrap();
    assert!(seen);
    assert_eq!(status.phase, Phase::RestartRequired);
    assert_eq!(fs::read(exe).unwrap(), b"old native executable");
    drop(state);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&target, "1.0.0").unwrap();
    assert!(state.ensure_updater_idle().is_err());
    state.reconcile_launcher_update(&target, "1.1.0").unwrap();
    assert_eq!(
        state.launcher_update_snapshot(Some(&config)).unwrap().phase,
        Phase::Installed
    );
}

#[test]
fn unrecognized_backup_is_preserved_and_keeps_reconciliation_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    state
        .apply_update_with(
            Some(&config),
            &target,
            ready.offer.unwrap().id,
            ready.revision,
            0,
            |_, _| Ok(()),
        )
        .unwrap();
    let mut record = state.update_record().unwrap();
    record.apply.as_mut().unwrap().handoff = false;
    let (_, backup) = record
        .apply
        .as_ref()
        .unwrap()
        .paths(record.owner.unwrap(), state.state_root())
        .unwrap();
    fs::write(backup.join("unrecognized"), b"preserve").unwrap();
    state.save_update(record).unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    assert_eq!(
        state.reconcile_launcher_update(&target, "1.0.0"),
        Err(Error::Reconciliation)
    );
    assert_eq!(fs::read(backup.join("unrecognized")).unwrap(), b"preserve");
    assert!(state.ensure_updater_idle().is_err());
}

mod native_uat;

#[test]
fn final_rename_failure_restores_original_bundle_before_releasing_ownership() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let original = bundle::fingerprint(&root.join("Launcher.app")).unwrap();
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &config, offer, &bytes);
    process::FAIL_FINAL_RENAME.with(|flag| flag.set(true));
    assert_eq!(
        state
            .apply_update_with(
                Some(&config),
                &target,
                ready.offer.unwrap().id,
                ready.revision,
                0,
                |_, _| panic!("final rename failed before spawn")
            )
            .unwrap_err(),
        Error::Replace
    );
    assert_eq!(
        bundle::fingerprint(&root.join("Launcher.app")).unwrap(),
        original
    );
    assert!(state.ensure_updater_idle().is_ok());
    assert_eq!(
        state
            .launcher_update_snapshot(Some(&config))
            .unwrap()
            .failure,
        Some(Error::Replace)
    );
}

#[test]
#[cfg(windows)]
fn native_windows_shell_handoff_launches_a_temporary_executable_without_claiming_installation() {
    // The native test runner is an inert executable fixture. Its zero-matching
    // test invocation is a process-launch proof, not an NSIS/MSI installation.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let exe = root.join("native-process-fixture.exe");
    fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    spawn(&exe, &["--exact".into(), "no_such_updater_test".into()]).unwrap();
    assert_eq!(spawn(&root.join("missing.exe"), &[]), Err(Error::Spawn));
}

#[test]
fn foreign_stage_or_installer_collision_is_preserved_during_reopen_recovery() {
    for windows in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (config, offer, bytes, mut target) = fixture(&root);
        if windows {
            let exe = root.join("launcher.exe");
            fs::write(&exe, b"old").unwrap();
            target = InstalledTarget(Target::Windows { executable: exe });
        }
        let original = match &target.0 {
            Target::Mac { bundle, .. } => bundle,
            Target::Windows { executable } => executable,
        };
        let mut state = DesktopState::open(&root.join("state")).unwrap();
        ready(&mut state, &config, offer, &bytes);
        let owner = Uuid::new_v4();
        let mut record = state.update_record().unwrap();
        record.phase = Phase::Installing;
        record.owner = Some(owner);
        record.apply = Some(Attempt {
            target: target.0.clone(),
            original: bundle::fingerprint(original).unwrap(),
            replacement: None,
            staged_bundle: None,
            handoff: false,
        });
        let (stage, _) = record
            .apply
            .as_ref()
            .unwrap()
            .paths(owner, state.state_root())
            .unwrap();
        state.save_update(record).unwrap();
        let sentinel = if windows {
            stage.clone()
        } else {
            fs::create_dir(&stage).unwrap();
            stage.join("foreign")
        };
        fs::write(&sentinel, b"foreign bytes must survive").unwrap();
        drop(state);
        let mut state = DesktopState::open(&root.join("state")).unwrap();
        assert!(state.reconcile_launcher_update(&target, "1.0.0").is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"foreign bytes must survive");
        assert!(state.ensure_updater_idle().is_err());
    }
}
