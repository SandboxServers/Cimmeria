use super::*;
use crate::{install_recovery, OperationState};
use ed25519_dalek::{Signer, SigningKey};

fn release(padding: usize) -> VerifiedRelease {
    let mut body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    body.extend(std::iter::repeat_n(b' ', padding));
    let sig = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let sig: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    verify_release(&body, sig.as_bytes()).unwrap()
}
fn setup(release: &VerifiedRelease) -> (tempfile::TempDir, DesktopState, Uuid) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    state.admit_install(id, 0, 1, release, vec![]).unwrap();
    (root, state, id)
}
#[test]
fn exact_original_signed_bytes_survive_restart_for_offline_recovery() {
    // Deliberate whitespace exceeds ordinary state-file limits and must not be
    // normalized away: the signature is over original bytes, not parsed JSON.
    let original = release(70_000);
    let (root, state, _id) = setup(&original);
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let cached = state.cached_install_release().unwrap();
    assert_eq!(cached.evidence(), original.evidence());
    assert_eq!(cached.digest(), original.digest());
    assert_eq!(
        install_recovery::reconcile(&mut state, &cached).unwrap(),
        install_recovery::Recovery::NoOutput
    );
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
fn tampered_and_other_valid_release_evidence_are_rejected() {
    let original = release(0);
    let (root, mut state, id) = setup(&original);
    let path = root.path().join("state").join(name(id));
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[5] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    assert!(matches!(
        state.cached_install_release(),
        Err(EvidenceError::Verification(_))
    ));
    state.save_release_evidence(id, &release(1)).unwrap();
    assert_eq!(
        state.cached_install_release().unwrap_err(),
        EvidenceError::IdentityMismatch
    );
}
#[test]
fn cache_write_failure_prevents_admission_and_preserves_game_directory() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    std::fs::create_dir(root.path().join("state").join(name(id))).unwrap();
    assert!(state.admit_install(id, 0, 1, &release(0), vec![]).is_err());
    assert!(state.operations().snapshot().operation.is_none());
    assert!(!root.path().join("install").exists());
}
#[test]
fn malformed_oversized_and_missing_cache_never_fall_back_to_network() {
    let (root, state, id) = setup(&release(0));
    let path = root.path().join("state").join(name(id));
    std::fs::write(&path, u32::MAX.to_le_bytes()).unwrap();
    assert_eq!(
        state.cached_install_release().unwrap_err(),
        EvidenceError::Storage(StorageError::Corrupt)
    );
    std::fs::write(&path, vec![0; MAX_EVIDENCE + 1]).unwrap();
    assert_eq!(
        state.cached_install_release().unwrap_err(),
        EvidenceError::Storage(StorageError::TooLarge)
    );
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        state.cached_install_release().unwrap_err(),
        EvidenceError::Storage(StorageError::Io)
    );
}

#[test]
fn two_signed_releases_survive_reopen_without_rewriting_install_ownership() {
    let original = release(0);
    let next = release(1);
    let (root, mut state, installation_id) = setup(&original);
    let intent = state.install_intent().unwrap().unwrap();
    let intent_path = root
        .path()
        .join("state")
        .join(format!("install-intent-{installation_id}.json"));
    let owner_bytes = std::fs::read(&intent_path).unwrap();
    let original_bytes =
        std::fs::read(root.path().join("state").join(name(installation_id))).unwrap();
    let update_id = Uuid::new_v4();
    state.save_release_evidence(update_id, &next).unwrap();
    let next_identity = ReleaseIdentity {
        evidence_id: update_id,
        manifest_digest: next.digest(),
    };
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state
            .verify_release_identity(intent.release_identity())
            .unwrap()
            .evidence(),
        original.evidence()
    );
    assert_eq!(
        state
            .verify_release_identity(next_identity)
            .unwrap()
            .evidence(),
        next.evidence()
    );
    assert_eq!(std::fs::read(intent_path).unwrap(), owner_bytes);
    assert_eq!(
        std::fs::read(root.path().join("state").join(name(installation_id))).unwrap(),
        original_bytes
    );
    assert_eq!(state.install_intent().unwrap().unwrap(), intent);
    assert!(!root
        .path()
        .join("state")
        .join(format!("install-intent-{update_id}.json"))
        .exists());
    assert_eq!(
        state
            .verify_release_identity(ReleaseIdentity {
                evidence_id: update_id,
                manifest_digest: original.digest(),
            })
            .unwrap_err(),
        EvidenceError::IdentityMismatch
    );
    assert_eq!(
        state
            .verify_release_identity(ReleaseIdentity {
                evidence_id: installation_id,
                manifest_digest: next.digest(),
            })
            .unwrap_err(),
        EvidenceError::IdentityMismatch
    );
}
