//! Known offers remain mandatory through rechecks, preparation failure and reopen.
use super::*;
use crate::launch::{Artifact, Resources};
use sha2::{Digest, Sha256};

fn assert_play_refused(state: &mut DesktopState, root: &std::path::Path) {
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
    let before = state.operations.snapshot().clone();
    let result = state.admit_launch(
        Uuid::new_v4(),
        before.revision,
        Uuid::new_v4(),
        Resources {
            helper,
            client_patches: None,
            client_telemetry: None,
            graphics: None,
        },
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
    assert!(!state.launcher_update_blocks_play().unwrap());
    let offered = available(&mut state, &config, offer);
    assert_play_refused(&mut state, root.path());
    let ticket = state
        .begin_launcher_update_check(Some(&config), offered.revision, offered.operation_revision)
        .unwrap();
    state
        .finish_launcher_update_check(ticket, Err(Error::Transport))
        .unwrap();
    assert!(state.launcher_update_blocks_play().unwrap());
    drop(state);
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_play_refused(&mut state, root.path());
    let saved = state.launcher_update_snapshot(Some(&config)).unwrap();
    assert!(saved.offer.is_some());
    let ticket = state
        .begin_launcher_update_check(Some(&config), saved.revision, saved.operation_revision)
        .unwrap();
    state
        .finish_launcher_update_check(ticket, Ok(None))
        .unwrap();
    assert!(!state.launcher_update_blocks_play().unwrap());
}

#[test]
fn ready_and_failed_download_keep_update_mandatory_but_initial_network_failure_does_not() {
    for bytes in [PAYLOAD, &b"tampered"[..]] {
        let root = tempfile::tempdir().unwrap();
        let (config, offer) = fixture("1.1.0");
        let mut state = DesktopState::open(root.path()).unwrap();
        let available = available(&mut state, &config, offer);
        prepare(&mut state, &config, available, bytes);
        assert_play_refused(&mut state, root.path());
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
    assert!(!state.launcher_update_blocks_play().unwrap());
}
