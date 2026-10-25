//! Known offers remain mandatory through rechecks, preparation failure and reopen.
use super::*;
use crate::launch::{Artifact, Resources};
use sha2::{Digest, Sha256};

fn resources(root: &std::path::Path) -> Resources {
    let helper = root.join("inert-helper");
    std::fs::write(&helper, PAYLOAD).unwrap();
    let helper = Artifact::open(
        helper.canonicalize().unwrap(),
        &Sha256::digest(PAYLOAD)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
    )
    .unwrap();
    Resources {
        proton: None,
        helper,
        client_patches: None,
        client_telemetry: None,
        graphics: None,
    }
}

fn assert_play_refused(state: &mut DesktopState, root: &std::path::Path, config: &Config) {
    let before = state.operations.snapshot().clone();
    let result = state.admit_launch_with_updater(
        Uuid::new_v4(),
        before.revision,
        Uuid::new_v4(),
        resources(root),
        Some(config),
    );
    assert!(matches!(
        result,
        Err(crate::IntentError::Storage(StorageError::Busy))
    ));
    assert_eq!(state.operations.snapshot().operation, before.operation);
}

#[test]
fn known_offer_blocks_native_play_after_failed_check_and_reopen() {
    let root = tempfile::tempdir().unwrap();
    let (config, offer) = fixture("1.1.0");
    let mut state = DesktopState::open(root.path()).unwrap();
    assert!(!state.launcher_update_blocks_play(Some(&config)).unwrap());
    let offered = available(&mut state, &config, offer);
    assert_play_refused(&mut state, root.path(), &config);
    let ticket = state
        .begin_launcher_update_check(Some(&config), offered.revision, offered.operation_revision)
        .unwrap();
    state
        .finish_launcher_update_check(ticket, Err(Error::Transport))
        .unwrap();
    assert!(state.launcher_update_blocks_play(Some(&config)).unwrap());
    drop(state);
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_play_refused(&mut state, root.path(), &config);
    let saved = state.launcher_update_snapshot(Some(&config)).unwrap();
    assert!(saved.offer.is_some());
    let ticket = state
        .begin_launcher_update_check(Some(&config), saved.revision, saved.operation_revision)
        .unwrap();
    state
        .finish_launcher_update_check(ticket, Ok(None))
        .unwrap();
    assert!(!state.launcher_update_blocks_play(Some(&config)).unwrap());
}

#[test]
fn ready_and_failed_download_keep_update_mandatory_but_initial_network_failure_does_not() {
    for bytes in [PAYLOAD, &b"tampered"[..]] {
        let root = tempfile::tempdir().unwrap();
        let (config, offer) = fixture("1.1.0");
        let mut state = DesktopState::open(root.path()).unwrap();
        let available = available(&mut state, &config, offer);
        prepare(&mut state, &config, available, bytes);
        assert_play_refused(&mut state, root.path(), &config);
    }
    let root = tempfile::tempdir().unwrap();
    let (config, _) = fixture("1.1.0");
    let mut state = DesktopState::open(root.path()).unwrap();
    let ticket = state
        .begin_launcher_update_check(Some(&config), 0, 0)
        .unwrap();
    state
        .finish_launcher_update_check(ticket, Err(Error::Transport))
        .unwrap();
    assert!(!state.launcher_update_blocks_play(Some(&config)).unwrap());
}

#[test]
fn disabled_build_reopens_inactive_offers_without_an_updater_play_gate() {
    for phase in [Phase::Available, Phase::Ready, Phase::Failed] {
        let root = tempfile::tempdir().unwrap();
        let (config, offer) = fixture("1.1.0");
        let mut state = DesktopState::open(root.path()).unwrap();
        let offered = available(&mut state, &config, offer);
        if phase != Phase::Available {
            prepare(
                &mut state,
                &config,
                offered,
                if phase == Phase::Ready {
                    PAYLOAD
                } else {
                    b"tampered"
                },
            );
        }
        drop(state);
        let mut state = DesktopState::open(root.path()).unwrap();
        let snapshot = state.launcher_update_snapshot(None).unwrap();
        assert_eq!(snapshot.phase, Phase::Disabled);
        assert!(snapshot.offer.is_none());
        assert!(!state.launcher_update_blocks_play(None).unwrap());
        let result = state.admit_launch(Uuid::new_v4(), 0, Uuid::new_v4(), resources(root.path()));
        assert!(!matches!(
            result,
            Err(crate::IntentError::Storage(StorageError::Busy))
        ));
        assert_play_refused(&mut state, root.path(), &config);
    }
}

#[test]
fn disabled_build_still_blocks_active_update_work() {
    let root = tempfile::tempdir().unwrap();
    let (config, _) = fixture("1.1.0");
    let mut state = DesktopState::open(root.path()).unwrap();
    state
        .begin_launcher_update_check(Some(&config), 0, 0)
        .unwrap();
    assert!(state.launcher_update_blocks_play(None).unwrap());
    let result = state.admit_launch(Uuid::new_v4(), 0, Uuid::new_v4(), resources(root.path()));
    assert!(matches!(
        result,
        Err(crate::IntentError::Storage(StorageError::Busy))
    ));
}

#[test]
fn failed_recheck_does_not_require_an_already_installed_offer() {
    for current in ["1.1.0", "1.2.0"] {
        let root = tempfile::tempdir().unwrap();
        let (config, offer) = fixture_with_current("1.1.0", current);
        let mut state = DesktopState::open(root.path()).unwrap();
        // Acknowledgment retains the offer for cleanup retries; only the check
        // transition is under test, not package replacement/startup health.
        let mut record = state.update_record().unwrap();
        record.phase = Phase::Installed;
        record.offer = Some(offer);
        state.save_update(record).unwrap();
        let saved = state.launcher_update_snapshot(Some(&config)).unwrap();
        assert!(saved.offer.is_some());
        let ticket = state
            .begin_launcher_update_check(Some(&config), saved.revision, saved.operation_revision)
            .unwrap();
        state
            .finish_launcher_update_check(ticket, Err(Error::Transport))
            .unwrap();
        let failed = state.launcher_update_snapshot(Some(&config)).unwrap();
        assert_eq!(failed.phase, Phase::Failed);
        assert!(
            failed.offer.is_none(),
            "an installed offer must not reach the frontend as pending"
        );
        assert!(!state.launcher_update_blocks_play(Some(&config)).unwrap());
    }
}
