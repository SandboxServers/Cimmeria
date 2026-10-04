use super::*;
use crate::catalog::verify_release;
use ed25519_dalek::{Signer, SigningKey};
use uuid::Uuid;

fn release() -> VerifiedRelease {
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    let signature = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let signature: String = signature
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    verify_release(&body, signature.as_bytes()).unwrap()
}
fn interrupted() -> (
    tempfile::TempDir,
    DesktopState,
    InstallIntent,
    VerifiedRelease,
) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let release = release();
    let id = Uuid::new_v4();
    let intent = state
        .admit_install(id, 0, 1, &release, vec![])
        .unwrap()
        .intent;
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    (root, state, intent, release)
}
fn owned(intent: &InstallIntent) {
    std::fs::create_dir(&intent.destination).unwrap();
    atomic::write(&intent.destination, ".cimmeria-install.json", intent).unwrap();
}
fn prepared(intent: &InstallIntent, release: &VerifiedRelease) {
    owned(intent);
    let content = intent.destination.join("game");
    std::fs::create_dir_all(content.join("Working/Binaries")).unwrap();
    std::fs::create_dir_all(content.join("Working/SGWGame")).unwrap();
    std::fs::write(content.join("Working/Binaries/SGW.exe"), b"fixture").unwrap();
    crate::state::InstalledState {
        seed_sha256: Some(release.manifest().seed.sha256.clone()),
        applied_patches: vec![],
        seed_adopted: false,
    }
    .save(&content)
    .unwrap();
    atomic::write(&intent.destination, "content-ready.json", intent).unwrap();
}

#[test]
fn absent_destination_reconciles_without_creating_files() {
    let (_root, mut state, intent, release) = interrupted();
    assert_eq!(reconcile(&mut state, &release).unwrap(), Recovery::NoOutput);
    assert!(!intent.destination.exists());
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Failed
    );
}

#[test]
fn owned_partial_content_stays_gated_and_untouched() {
    let (_root, mut state, intent, release) = interrupted();
    owned(&intent);
    std::fs::write(intent.destination.join("partial"), b"keep").unwrap();
    assert_eq!(reconcile(&mut state, &release).unwrap(), Recovery::Partial);
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert_eq!(
        std::fs::read(intent.destination.join("partial")).unwrap(),
        b"keep"
    );
}

#[test]
fn receipt_before_terminal_journal_recovers_prepared_content() {
    let (root, mut state, intent, release) = interrupted();
    prepared(&intent, &release);
    assert_eq!(
        reconcile(&mut state, &release).unwrap(),
        Recovery::ContentPrepared
    );
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Succeeded
    );
}

#[test]
fn receipt_alone_cannot_hide_missing_content() {
    let (_root, mut state, intent, release) = interrupted();
    prepared(&intent, &release);
    std::fs::remove_file(intent.destination.join("game/Working/Binaries/SGW.exe")).unwrap();
    assert_eq!(reconcile(&mut state, &release).unwrap(), Recovery::Partial);
}

#[test]
fn live_owner_lock_prevents_reconciliation() {
    let (_root, mut state, intent, release) = interrupted();
    prepared(&intent, &release);
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(intent.destination.join(".cimmeria-install.json"))
        .unwrap();
    lock.lock().unwrap();
    assert_eq!(
        reconcile(&mut state, &release),
        Err(IntentError::Storage(StorageError::InUse))
    );
    lock.unlock().unwrap();
    assert_eq!(
        reconcile(&mut state, &release).unwrap(),
        Recovery::ContentPrepared
    );
}

#[test]
fn foreign_marker_does_not_grant_ownership() {
    let (_root, mut state, intent, release) = interrupted();
    prepared(&intent, &release);
    let mut foreign = intent.clone();
    foreign.operation_id = Uuid::new_v4();
    atomic::write(&intent.destination, ".cimmeria-install.json", &foreign).unwrap();
    assert_eq!(
        reconcile(&mut state, &release),
        Err(IntentError::Storage(StorageError::Corrupt))
    );
    assert!(intent.destination.join("game").is_dir());
}

#[test]
fn crash_before_marker_with_empty_directory_can_be_reconciled() {
    let (_root, mut state, intent, release) = interrupted();
    std::fs::create_dir(&intent.destination).unwrap();
    assert_eq!(reconcile(&mut state, &release).unwrap(), Recovery::NoOutput);
    assert_eq!(std::fs::read_dir(&intent.destination).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn redirected_content_subdirectory_does_not_prove_preparation() {
    let (root, mut state, intent, release) = interrupted();
    prepared(&intent, &release);
    let game = intent.destination.join("game/Working/SGWGame");
    std::fs::remove_dir(&game).unwrap();
    let outside = root.path().join("elsewhere");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, game).unwrap();
    assert_eq!(reconcile(&mut state, &release).unwrap(), Recovery::Partial);
}
