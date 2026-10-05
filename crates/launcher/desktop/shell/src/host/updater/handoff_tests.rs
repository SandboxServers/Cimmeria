//! Regression through the real engine apply and retained native worker boundary.
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use cimmeria_launcher_engine::DesktopState;
use std::{
    fs,
    io::Cursor,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

fn ready(state: &mut DesktopState) -> (updater::Config, Snapshot) {
    let bytes = b"inert signed installer fixture";
    let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let signature = minisign::sign(
        Some(&keys.pk),
        &keys.sk,
        Cursor::new(bytes),
        Some("version:1.1.0"),
        None,
    )
    .unwrap();
    let config = updater::Config::new(
        "https://updates.example.test/feed",
        &STANDARD.encode(keys.pk.to_box().unwrap().to_string()),
        "1.0.0",
        "windows-x86_64",
        vec!["updates.example.test".into()],
    )
    .unwrap();
    let initial = state.launcher_update_snapshot(Some(&config)).unwrap();
    let ticket = state
        .begin_launcher_update_check(Some(&config), initial.revision, initial.operation_revision)
        .unwrap();
    state
        .finish_launcher_update_check(
            ticket,
            Ok(Some(
                serde_json::from_value(serde_json::json!({
                    "id": Uuid::new_v4(),
                    "version": "1.1.0",
                    "notes": "",
                    "url": "https://updates.example.test/launcher.exe",
                    "signature": STANDARD.encode(signature.to_string()),
                }))
                .unwrap(),
            )),
        )
        .unwrap();
    let available = state.launcher_update_snapshot(Some(&config)).unwrap();
    let mut ticket = state
        .begin_launcher_update_prepare(
            Some(&config),
            available.offer.unwrap().id,
            available.revision,
            available.operation_revision,
        )
        .unwrap();
    state.mark_launcher_update_verifying(&mut ticket).unwrap();
    state
        .finish_launcher_update_prepare(&config, ticket, Ok(bytes.to_vec()))
        .unwrap();
    let ready = state.launcher_update_snapshot(Some(&config)).unwrap();
    (config, ready)
}

#[tokio::test]
async fn successful_engine_spawn_shuts_down_once_despite_post_spawn_persistence_failure() {
    for after_replace in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let state_path = root.join("state");
        let executable = root.join("launcher.exe");
        fs::write(&executable, b"old native executable").unwrap();
        let mut state = DesktopState::open(&state_path).unwrap();
        let (config, ready) = ready(&mut state);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let reopen_config = config.clone();
        let owner = Arc::new(std::sync::Mutex::new(None));
        let spawned_owner = owner.clone();
        let result = retained_apply(
            move |on_handoff| {
                let result = state.apply_launcher_update_fixture(
                    &config,
                    &executable,
                    ready.clone(),
                    |path, _| {
                        let record: serde_json::Value = serde_json::from_slice(
                            &fs::read(
                                executable
                                    .parent()
                                    .unwrap()
                                    .join("state/launcher-update.json"),
                            )
                            .unwrap(),
                        )
                        .unwrap();
                        assert_eq!(record["phase"], "installing");
                        assert_eq!(record["apply"]["handoff"], true);
                        *spawned_owner.lock().unwrap() = Some(record["owner"].clone());
                        assert_eq!(fs::read(path).unwrap(), b"inert signed installer fixture");
                        // The simulated spawn has succeeded. Fail the NEXT real atomic
                        // save, never the durable intent which precedes process launch.
                        updater::fail_next_update_save_for_test(after_replace);
                        Ok(())
                    },
                    on_handoff,
                );
                assert!(result.is_err());
                assert!(state.ensure_updater_idle().is_err());
                assert!(state
                    .apply_launcher_update_fixture(
                        &config,
                        &executable,
                        ready,
                        |_, _| panic!("duplicate spawn"),
                        || panic!("duplicate handoff"),
                    )
                    .is_err());
                result
            },
            Arc::new(move || {
                observed.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            result.unwrap_err(),
            Error::Storage(if after_replace {
                StorageError::PersistenceUncertain
            } else {
                StorageError::Io
            })
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let mut reopened = DesktopState::open(&state_path).unwrap();
        let snapshot = reopened
            .launcher_update_snapshot(Some(&reopen_config))
            .unwrap();
        assert_eq!(snapshot.phase, updater::Phase::ReconciliationRequired);
        assert!(reopened.ensure_updater_idle().is_err());
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(state_path.join("launcher-update.json")).unwrap())
                .unwrap();
        assert!(record["owner"].is_string());
        assert_eq!(Some(record["owner"].clone()), *owner.lock().unwrap());
        assert_eq!(record["apply"]["handoff"], true);
    }
}

#[tokio::test]
async fn failed_engine_spawn_never_shuts_down_despite_persisted_handoff_intent() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let executable = root.join("launcher.exe");
    fs::write(&executable, b"old native executable").unwrap();
    let (config, ready) = ready(&mut state);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let error = retained_apply(
        move |on_handoff| {
            let result = state.apply_launcher_update_fixture(
                &config,
                &executable,
                ready,
                |_, _| {
                    let record: serde_json::Value = serde_json::from_slice(
                        &fs::read(
                            executable
                                .parent()
                                .unwrap()
                                .join("state/launcher-update.json"),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    assert_eq!(record["apply"]["handoff"], true);
                    Err(Error::Spawn)
                },
                on_handoff,
            );
            assert!(state.ensure_updater_idle().is_ok());
            result
        },
        Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        }),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(error, Error::Spawn);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
