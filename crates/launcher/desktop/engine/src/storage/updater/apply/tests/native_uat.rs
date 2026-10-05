use super::*;
#[test]
#[cfg(target_os = "macos")]
#[ignore = "Driven by updater-apply-native-uat.mjs over stdin/stdout"]
fn updater_apply_native_uat_bridge() {
    use std::io::{BufRead, Write};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let mut state = Some(DesktopState::open(&root.join("state")).unwrap());
    ready(state.as_mut().unwrap(), &config, offer, &bytes);
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result: Result<Snapshot, Error> = (|| {
            if value["command"] == "reopen" || value["command"] == "acknowledge" {
                drop(state.take());
                state = Some(DesktopState::open(&root.join("state")).unwrap());
                state.as_mut().unwrap().reconcile_launcher_update(
                    &target,
                    if value["command"] == "acknowledge" {
                        "1.1.0"
                    } else {
                        "1.0.0"
                    },
                )?;
            }
            let state = state.as_mut().unwrap();
            if value["command"] == "apply" {
                state.apply_launcher_update(
                    Some(&config),
                    &target,
                    serde_json::from_value(value["offer_id"].clone()).unwrap(),
                    value["revision"].as_u64().unwrap(),
                    value["operation_revision"].as_u64().unwrap(),
                )?;
                for _ in 0..100 {
                    if fs::read(root.join("started")).ok().as_deref() == Some(b"started") {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                assert_eq!(fs::read(root.join("started")).unwrap(), b"started");
            }
            if value["command"] == "try_game_mutation" {
                state.operations_mut()?;
            }
            state.launcher_update_snapshot(Some(&config))
        })();
        let result = match result {
            Ok(snapshot) => serde_json::json!({"ok":snapshot}),
            Err(error) => serde_json::json!({"error":error}),
        };
        println!("UPDATER_APPLY_UAT {result}");
        std::io::stdout().flush().unwrap();
    }
}

#[test]
#[cfg(target_os = "macos")]
fn interrupted_atomic_swap_keeps_visible_app_and_new_startup_can_acknowledge() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (config, offer, bytes, target) = fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    ready(&mut state, &config, offer, &bytes);
    let owner = Uuid::new_v4();
    let installed = root.join("Launcher.app");
    let original = bundle::fingerprint(&installed).unwrap();
    let stage = root.join(format!(".cimmeria-update-{owner}"));
    let extracted = bundle::extract(&bytes, &stage, owner).unwrap();
    let mut record = state.update_record().unwrap();
    record.phase = Phase::Installing;
    record.owner = Some(owner);
    record.apply = Some(Attempt {
        target: target.0,
        original,
        replacement: Some(bundle::fingerprint(&extracted).unwrap()),
        staged_bundle: Some("Launcher.app".into()),
        handoff: false,
    });
    state.save_update(record).unwrap();
    exchange(&installed, &extracted).unwrap();
    drop(state);
    assert!(installed.join("Contents/MacOS/launcher").is_file());
    let target =
        InstalledTarget::from_executable(&installed.join("Contents/MacOS/launcher")).unwrap();
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&target, "1.1.0").unwrap();
    assert_eq!(
        state.launcher_update_snapshot(Some(&config)).unwrap().phase,
        Phase::Installed
    );
    assert!(state.ensure_updater_idle().is_ok());
    assert!(!stage.exists());
}
