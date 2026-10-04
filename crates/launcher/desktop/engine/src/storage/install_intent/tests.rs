use super::*;
use crate::{catalog::verify_release, OperationState};
use ed25519_dalek::{Signer, SigningKey};

fn release(label: &str) -> VerifiedRelease {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema":1,"seed":{"blob":label,"size":1,"sha256":"a".repeat(64)},"patches":[]
    }))
    .unwrap();
    let signature = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&bytes);
    let hex: String = signature
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    verify_release(&bytes, hex.as_bytes()).unwrap()
}
fn setup() -> (tempfile::TempDir, DesktopState) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("game")), false, 0)
        .unwrap();
    (root, state)
}

#[test]
fn admission_persists_exact_signed_identity_without_mutating_destination() {
    let (root, mut state) = setup();
    let id = Uuid::new_v4();
    let release = release("seed.zip");
    let admitted = state.admit_install(id, 0, 1, &release, vec![]).unwrap();
    assert!(admitted.dispatch);
    assert!(!root.path().join("game").exists());
    assert_eq!(
        state.install_intent().unwrap(),
        Some(admitted.intent.clone())
    );
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
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
    let retry = state.admit_install(id, 0, 1, &release, vec![]).unwrap();
    assert!(!retry.dispatch);
    assert_eq!(retry.intent, admitted.intent);
    assert!(!root.path().join("game").exists());
}

#[test]
fn retry_retains_intent_after_consent_change_but_changed_release_is_rejected() {
    let (_root, mut state) = setup();
    let id = Uuid::new_v4();
    state
        .admit_install(id, 0, 1, &release("a.zip"), vec![])
        .unwrap();
    state
        .save_preferences(state.preferences.install_directory.clone(), true, 1)
        .unwrap();
    assert!(
        !state
            .admit_install(id, 0, 1, &release("a.zip"), vec![])
            .unwrap()
            .dispatch
    );
    assert_eq!(
        state
            .admit_install(id, 0, 1, &release("b.zip"), vec![])
            .unwrap_err(),
        IntentError::Operation(ContractError::IdentityConflict)
    );
}

#[test]
fn busy_and_stale_admission_cannot_overwrite_active_recovery_evidence() {
    let (root, mut state) = setup();
    let id = Uuid::new_v4();
    state
        .admit_install(id, 0, 1, &release("a.zip"), vec![])
        .unwrap();
    let path = root.path().join("state").join(intent_name(id));
    let before = std::fs::read(&path).unwrap();
    assert_eq!(
        state
            .admit_install(Uuid::new_v4(), 1, 1, &release("b.zip"), vec![])
            .unwrap_err(),
        IntentError::Operation(ContractError::Busy)
    );
    assert_eq!(
        state
            .admit_install(Uuid::new_v4(), 0, 1, &release("b.zip"), vec![])
            .unwrap_err(),
        IntentError::Operation(ContractError::StaleRevision)
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn corrupt_or_missing_intent_never_becomes_a_new_dispatch() {
    let (root, mut state) = setup();
    let id = Uuid::new_v4();
    state
        .admit_install(id, 0, 1, &release("a.zip"), vec![])
        .unwrap();
    let path = root.path().join("state").join(intent_name(id));
    std::fs::write(&path, b"{}").unwrap();
    assert_eq!(
        state
            .admit_install(id, 0, 1, &release("a.zip"), vec![])
            .unwrap_err(),
        IntentError::Storage(StorageError::Corrupt)
    );
    std::fs::remove_file(path).unwrap();
    assert_eq!(state.install_intent(), Err(StorageError::Corrupt));
}

#[test]
fn intent_write_failure_prevents_operation_admission() {
    let (root, mut state) = setup();
    let id = Uuid::new_v4();
    std::fs::create_dir(root.path().join("state").join(intent_name(id))).unwrap();
    assert_eq!(
        state
            .admit_install(id, 0, 1, &release("a.zip"), vec![])
            .unwrap_err(),
        IntentError::Storage(StorageError::UnsafeFile)
    );
    assert!(state.operations().snapshot().operation.is_none());
    assert!(!root.path().join("game").exists());
}

#[test]
fn refuses_nonempty_or_state_overlapping_destinations() {
    let (root, mut state) = setup();
    std::fs::create_dir(root.path().join("game")).unwrap();
    std::fs::write(root.path().join("game/unrelated.txt"), b"keep").unwrap();
    assert_eq!(
        state
            .admit_install(Uuid::new_v4(), 0, 1, &release("a.zip"), vec![])
            .unwrap_err(),
        IntentError::Storage(StorageError::InvalidDirectory)
    );
    state
        .save_preferences(Some(root.path().join("state/game")), false, 1)
        .unwrap();
    assert_eq!(
        state
            .admit_install(Uuid::new_v4(), 0, 2, &release("a.zip"), vec![])
            .unwrap_err(),
        IntentError::Storage(StorageError::InvalidDirectory)
    );
    assert_eq!(
        std::fs::read(root.path().join("game/unrelated.txt")).unwrap(),
        b"keep"
    );
}

#[test]
fn failed_replacement_admission_preserves_previous_terminal_intent_on_restart() {
    let (root, mut state) = setup();
    let previous = Uuid::new_v4();
    let release = release("a.zip");
    let original = state
        .admit_install(previous, 0, 1, &release, vec![])
        .unwrap()
        .intent;
    state
        .operations_mut()
        .unwrap()
        .observe(previous, OperationState::Failed)
        .unwrap();
    let revision = state.operations().snapshot().revision;
    let next = Uuid::new_v4();
    let result = state.admit_install_with(
        AdmissionRequest {
            id: next,
            operation_revision: revision,
            preferences_revision: 1,
            release: &release,
            login_servers: vec![],
            backend: ExtractionBackend::Native,
        },
        |_, _, _, _| Err(ContractError::PersistenceFailed),
    );
    assert_eq!(
        result.unwrap_err(),
        IntentError::Operation(ContractError::PersistenceFailed)
    );
    assert!(root.path().join("state").join(intent_name(next)).is_file());
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(state.install_intent().unwrap(), Some(original));
    assert!(
        !state
            .admit_install(previous, 0, 1, &release, vec![])
            .unwrap()
            .dispatch
    );
    assert!(!root.path().join("game").exists());
}

#[test]
fn valid_but_modified_intent_fails_digest_check() {
    let (root, mut state) = setup();
    let id = Uuid::new_v4();
    let mut intent = state
        .admit_install(id, 0, 1, &release("a.zip"), vec![])
        .unwrap()
        .intent;
    intent.destination = root.path().join("elsewhere");
    std::fs::write(
        root.path().join("state").join(intent_name(id)),
        serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    assert_eq!(state.install_intent(), Err(StorageError::Corrupt));
}

#[test]
fn existing_empty_directory_is_accepted_without_claiming_it() {
    let (root, mut state) = setup();
    std::fs::create_dir(root.path().join("game")).unwrap();
    assert!(
        state
            .admit_install(Uuid::new_v4(), 0, 1, &release("a.zip"), vec![])
            .unwrap()
            .dispatch
    );
    assert_eq!(
        std::fs::read_dir(root.path().join("game")).unwrap().count(),
        0
    );
}

#[cfg(unix)]
#[test]
fn destination_symlink_is_rejected_even_when_target_is_empty() {
    let (root, mut state) = setup();
    std::fs::create_dir(root.path().join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(root.path().join("elsewhere"), root.path().join("game")).unwrap();
    assert_eq!(
        state
            .admit_install(Uuid::new_v4(), 0, 1, &release("a.zip"), vec![])
            .unwrap_err(),
        IntentError::Storage(StorageError::InvalidDirectory)
    );
}

#[test]
fn native_intent_retains_legacy_encoding_and_backend_changes_conflict() {
    let (_root, mut state) = setup();
    let id = Uuid::new_v4();
    let release = release("seed.zip");
    let intent = state
        .admit_install(id, 0, 1, &release, vec![])
        .unwrap()
        .intent;
    let bytes = serde_json::to_vec(&intent).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("backend"));
    let restored: InstallIntent = serde_json::from_slice(&bytes).unwrap();
    assert!(restored.backend.is_native());
    assert_eq!(restored.digest().unwrap(), intent.digest().unwrap());
    let result = state.admit_install_backend(AdmissionRequest {
        id,
        operation_revision: 0,
        preferences_revision: 1,
        release: &release,
        login_servers: vec![],
        backend: ExtractionBackend::Wine {
            runtime_sha256: [1; 32],
            helper_sha256: [2; 32],
        },
    });
    assert!(matches!(
        result,
        Err(IntentError::Operation(ContractError::IdentityConflict))
    ));
}
