use super::*;
use crate::catalog::verify_release;
use ed25519_dalek::{Signer, SigningKey};
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, DesktopState, InstallIntent) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("game")), false, 0)
        .unwrap();
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    let sig = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let sig: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let release = verify_release(&body, sig.as_bytes()).unwrap();
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
    std::fs::create_dir_all(intent.destination.join("game")).unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        atomic::write(&intent.destination, name, &intent).unwrap();
    }
    (root, state, intent)
}

#[test]
fn identity_survives_new_operation_reopen_and_changed_preferences() {
    let (root, mut state, intent) = fixture();
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(intent.operation_id, OperationState::Succeeded)
        .unwrap();
    state
        .save_preferences(Some(root.path().join("another")), false, 1)
        .unwrap();
    let rev = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(Uuid::new_v4(), OperationKind::Repair, [7; 32], rev)
        .unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert!(state.install_intent().unwrap().is_none());
    let found = state.installed_content().unwrap().unwrap();
    assert_eq!(found.intent, intent);
    assert_eq!(found.release.digest(), intent.manifest_digest);
    // Identity does not claim readiness: this fixture deliberately lacks SGW.exe.
    assert!(!intent
        .destination
        .join("game/Working/Binaries/SGW.exe")
        .exists());
}

#[test]
fn uncommitted_reference_stays_gated_after_restart() {
    let (root, mut state, _) = fixture();
    state.remember_prepared_content().unwrap();
    assert!(matches!(state.installed_content(), Err(StorageError::Busy)));
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert!(matches!(state.installed_content(), Err(StorageError::Busy)));
}

#[test]
fn legacy_success_migrates_only_from_matching_owned_receipt() {
    let (root, mut state, intent) = fixture();
    assert!(state.installed_content().unwrap().is_none());
    state
        .operations_mut()
        .unwrap()
        .observe(intent.operation_id, OperationState::Succeeded)
        .unwrap();
    assert_eq!(state.installed_content().unwrap().unwrap().intent, intent);
    assert!(root.path().join("state/installed-content.json").is_file());
    std::fs::remove_file(intent.destination.join("content-ready.json")).unwrap();
    assert!(state.installed_content().is_err());
}

#[test]
fn mismatched_owner_signature_and_future_schema_are_not_adopted() {
    let (root, mut state, intent) = fixture();
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(intent.operation_id, OperationState::Succeeded)
        .unwrap();
    let mut foreign = intent.clone();
    foreign.operation_id = Uuid::new_v4();
    atomic::write(&intent.destination, ".cimmeria-install.json", &foreign).unwrap();
    assert!(matches!(
        state.installed_content(),
        Err(StorageError::Corrupt)
    ));
    atomic::write(&intent.destination, ".cimmeria-install.json", &intent).unwrap();
    let evidence = root.path().join(format!(
        "state/release-evidence-{}.bin",
        intent.operation_id
    ));
    let bytes = std::fs::read(&evidence).unwrap();
    std::fs::write(&evidence, b"corrupt").unwrap();
    assert!(state.installed_content().is_err());
    std::fs::write(&evidence, bytes).unwrap();
    atomic::write(
        &root.path().join("state"),
        NAME,
        &Record {
            adoption: None,
            schema_version: 4,
            current_release: None,
            intent,
        },
    )
    .unwrap();
    assert!(matches!(
        state.installed_content(),
        Err(StorageError::UnsupportedSchema)
    ));
}

#[test]
fn missing_game_tree_retains_repair_identity_but_redirected_tree_is_refused() {
    let (_root, mut state, intent) = fixture();
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(intent.operation_id, OperationState::Succeeded)
        .unwrap();
    std::fs::remove_dir(intent.destination.join("game")).unwrap();
    assert_eq!(state.installed_content().unwrap().unwrap().intent, intent);
    std::fs::write(intent.destination.join("game"), b"foreign").unwrap();
    assert!(matches!(
        state.installed_content(),
        Err(StorageError::UnsafeFile)
    ));
}

#[cfg(unix)]
#[test]
fn linked_content_is_not_owned_even_when_receipts_match() {
    let (root, mut state, intent) = fixture();
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(intent.operation_id, OperationState::Succeeded)
        .unwrap();
    std::fs::remove_dir(intent.destination.join("game")).unwrap();
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(outside, intent.destination.join("game")).unwrap();
    assert!(matches!(
        state.installed_content(),
        Err(StorageError::UnsafeFile)
    ));
}

// Construct the durable post-publication state directly. This exercises consumers
// of two releases, not an Update worker or proof that replacement was completed.
fn changed_release() -> (
    tempfile::TempDir,
    DesktopState,
    InstallIntent,
    ReleaseIdentity,
) {
    let (root, mut state, intent) = fixture();
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(intent.operation_id, OperationState::Succeeded)
        .unwrap();
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"next.zip","size":2,"sha256":"b".repeat(64)},"patches":[]})).unwrap();
    let sig = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let sig: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let release = verify_release(&body, sig.as_bytes()).unwrap();
    let current = ReleaseIdentity {
        evidence_id: Uuid::new_v4(),
        manifest_digest: release.digest(),
    };
    state
        .save_release_evidence(current.evidence_id, &release)
        .unwrap();
    write_ready(&intent.destination, &intent, current).unwrap();
    atomic::write(
        &state.directory.root,
        NAME,
        &Record {
            schema_version: 3,
            intent: intent.clone(),
            adoption: None,
            current_release: Some(current),
        },
    )
    .unwrap();
    (root, state, intent, current)
}

#[test]
fn repair_reopen_uses_current_signed_release_and_preserves_original_owner() {
    let (root, mut state, owner, current) = changed_release();
    let owner_bytes = std::fs::read(owner.destination.join(".cimmeria-install.json")).unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    assert_eq!(installed.intent, owner);
    assert_eq!(installed.current_release, current);
    assert_eq!(installed.release.digest(), current.manifest_digest);
    let id = Uuid::new_v4();
    let rev = state.operations().snapshot().revision;
    let plan = state
        .admit_repair(id, rev, owner.operation_id, true)
        .unwrap()
        .plan;
    assert_eq!(plan.installation, owner);
    assert_eq!(plan.release_identity(), current);
    assert_eq!(
        state.cached_extraction_release(id).unwrap().digest(),
        current.manifest_digest
    );
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(state.repair_plan().unwrap().unwrap(), plan);
    assert_eq!(
        state.cached_extraction_release(id).unwrap().digest(),
        current.manifest_digest
    );
    assert_eq!(
        state
            .verify_release_identity(owner.release_identity())
            .unwrap()
            .digest(),
        owner.manifest_digest
    );
    assert_eq!(
        std::fs::read(owner.destination.join(".cimmeria-install.json")).unwrap(),
        owner_bytes
    );
}

#[test]
fn uninstall_after_release_change_keeps_owner_identity_and_retained_evidence() {
    let (root, mut state, owner, current) = changed_release();
    let target = state.uninstall_target().unwrap().unwrap();
    assert_eq!(target.installation_id, owner.operation_id);
    let revision = state.operations().snapshot().revision;
    state
        .uninstall(Uuid::new_v4(), revision, owner.operation_id, true)
        .unwrap();
    assert!(!owner.destination.exists());
    assert!(state.installed_content().unwrap().is_none());
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state.verify_release_identity(current).unwrap().digest(),
        current.manifest_digest
    );
    assert_eq!(
        state
            .verify_release_identity(owner.release_identity())
            .unwrap()
            .digest(),
        owner.manifest_digest
    );
}

#[test]
fn current_receipt_cannot_substitute_another_owner_or_another_release() {
    let (_root, mut state, owner, current) = changed_release();
    atomic::write(
        &owner.destination,
        "content-ready.json",
        &ContentReceipt {
            schema_version: 2,
            installation_id: Uuid::new_v4(),
            release: current,
        },
    )
    .unwrap();
    assert!(matches!(
        state.installed_content(),
        Err(StorageError::Corrupt)
    ));
    write_ready(&owner.destination, &owner, owner.release_identity()).unwrap();
    assert!(matches!(
        state.installed_content(),
        Err(StorageError::Corrupt)
    ));
    write_ready(&owner.destination, &owner, current).unwrap();
    assert_eq!(
        state.installed_content().unwrap().unwrap().current_release,
        current
    );
}
