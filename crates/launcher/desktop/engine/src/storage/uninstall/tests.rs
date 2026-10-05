use super::*;
use crate::catalog::verify_release;
use ed25519_dalek::{Signer, SigningKey};

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
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Succeeded)
        .unwrap();
    std::fs::write(intent.destination.join("game/content"), b"game fixture").unwrap();
    (root, state, intent)
}

fn remove(state: &mut DesktopState, intent: &InstallIntent) -> Result<(), IntentError> {
    state.uninstall(
        Uuid::new_v4(),
        state.operations().snapshot().revision,
        intent.operation_id,
        true,
    )
}

#[test]
fn confirmed_removal_uses_saved_identity_and_preserves_preferences_and_caches() {
    let (root, mut state, intent) = fixture();
    let other = root.path().join("other");
    std::fs::create_dir(&other).unwrap();
    std::fs::write(other.join("keep"), b"unrelated").unwrap();
    state
        .save_preferences(Some(other.clone()), true, 1)
        .unwrap();
    std::fs::create_dir(root.path().join("state/runtimes")).unwrap();
    remove(&mut state, &intent).unwrap();
    assert!(!intent.destination.exists());
    assert!(other.join("keep").is_file());
    assert!(state.preferences().launcher_summary_consent);
    assert!(root.path().join("state/runtimes").is_dir());
    assert!(state.installed_content().unwrap().is_none());
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
    drop(state);
    assert!(DesktopState::open(&root.path().join("state"))
        .unwrap()
        .installed_content()
        .unwrap()
        .is_none());
}

#[test]
fn missing_confirmation_stale_revision_and_wrong_installation_do_not_mutate() {
    let (_root, mut state, intent) = fixture();
    let rev = state.operations().snapshot().revision;
    assert!(state
        .uninstall(Uuid::new_v4(), rev, intent.operation_id, false)
        .is_err());
    assert!(state
        .uninstall(Uuid::new_v4(), rev - 1, intent.operation_id, true)
        .is_err());
    assert!(state
        .uninstall(Uuid::new_v4(), rev, Uuid::new_v4(), true)
        .is_err());
    assert!(intent.destination.join("game/content").is_file());
    assert_eq!(state.operations().snapshot().revision, rev);
}

#[test]
fn foreign_top_level_file_vetoes_before_admission() {
    let (_root, mut state, intent) = fixture();
    std::fs::write(intent.destination.join("keep"), b"foreign").unwrap();
    let rev = state.operations().snapshot().revision;
    assert!(remove(&mut state, &intent).is_err());
    assert_eq!(state.operations().snapshot().revision, rev);
    assert!(intent.destination.join("game/content").exists());
}

#[test]
fn live_owner_and_foreign_receipt_veto_deletion() {
    let (_root, mut state, intent) = fixture();
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(intent.destination.join(".cimmeria-install.json"))
        .unwrap();
    marker.try_lock().unwrap();
    assert!(remove(&mut state, &intent).is_err());
    drop(marker);
    let mut foreign = intent.clone();
    foreign.operation_id = Uuid::new_v4();
    atomic::write(&intent.destination, "content-ready.json", &foreign).unwrap();
    assert!(remove(&mut state, &intent).is_err());
    assert!(intent.destination.join("game/content").exists());
}

fn interrupted(state: &mut DesktopState, intent: &InstallIntent) -> Plan {
    let plan = Plan {
        current_release: None,
        schema_version: 1,
        id: Uuid::new_v4(),
        installation: intent.clone(),
    };
    atomic::write(&state.directory.root, &name(plan.id), &plan).unwrap();
    let rev = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(
            plan.id,
            OperationKind::Uninstall,
            plan.digest().unwrap(),
            rev,
        )
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(plan.id, OperationState::Running)
        .unwrap();
    plan
}

#[test]
fn recovery_handles_crash_after_rename_before_checkpoint_and_lost_reply() {
    let (root, mut state, intent) = fixture();
    let plan = interrupted(&mut state, &intent);
    std::fs::rename(&intent.destination, plan.detached().unwrap()).unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let rev = state.operations().snapshot().revision;
    state
        .uninstall(plan.id, rev, intent.operation_id, true)
        .unwrap();
    assert!(!plan.detached().unwrap().exists());
    let rev = state.operations().snapshot().revision;
    state
        .uninstall(plan.id, rev, intent.operation_id, true)
        .unwrap();
    assert!(state.installed_content().unwrap().is_none());
}

#[test]
fn recovery_finishes_partial_deletion_and_leaves_replacement_folder_alone() {
    let (root, mut state, intent) = fixture();
    let plan = interrupted(&mut state, &intent);
    let detached = plan.detached().unwrap();
    std::fs::rename(&intent.destination, &detached).unwrap();
    atomic::write(&state.directory.root, &detached_name(plan.id), &plan).unwrap();
    std::fs::remove_dir_all(detached.join("game")).unwrap();
    std::fs::create_dir(&intent.destination).unwrap();
    std::fs::write(intent.destination.join("replacement"), b"keep").unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let rev = state.operations().snapshot().revision;
    state
        .uninstall(plan.id, rev, intent.operation_id, true)
        .unwrap();
    assert!(intent.destination.join("replacement").is_file());
    assert!(!detached.exists());
}

#[test]
fn missing_source_without_detachment_checkpoint_stays_in_recovery() {
    let (root, mut state, intent) = fixture();
    let plan = interrupted(&mut state, &intent);
    std::fs::remove_dir_all(&intent.destination).unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let rev = state.operations().snapshot().revision;
    assert!(state
        .uninstall(plan.id, rev, intent.operation_id, true)
        .is_err());
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
}

#[cfg(unix)]
#[test]
fn symlink_inside_game_cannot_redirect_deletion() {
    let (root, mut state, intent) = fixture();
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep"), b"foreign").unwrap();
    std::os::unix::fs::symlink(&outside, intent.destination.join("game/link")).unwrap();
    assert!(remove(&mut state, &intent).is_err());
    assert!(outside.join("keep").exists());
    assert!(intent.destination.join("game/content").exists());
}

#[test]
fn recovery_after_content_and_marker_deletion_requires_durable_detachment() {
    for tombstone_exists in [true, false] {
        let (root, mut state, intent) = fixture();
        let plan = interrupted(&mut state, &intent);
        std::fs::rename(&intent.destination, plan.detached().unwrap()).unwrap();
        atomic::write(&state.directory.root, &detached_name(plan.id), &plan).unwrap();
        std::fs::remove_dir_all(plan.detached().unwrap()).unwrap();
        if tombstone_exists {
            std::fs::create_dir(plan.detached().unwrap()).unwrap();
        } else {
            state.forget_installed_content(&intent).unwrap();
        }
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        let rev = state.operations().snapshot().revision;
        state
            .uninstall(plan.id, rev, intent.operation_id, true)
            .unwrap();
        assert!(!plan.detached().unwrap().exists());
        assert!(state.installed_content().unwrap().is_none());
    }
}

#[test]
fn plan_write_failure_leaves_owned_content_and_journal_unchanged() {
    let (_root, mut state, intent) = fixture();
    let id = Uuid::new_v4();
    std::fs::create_dir(state.directory.root.join(name(id))).unwrap();
    let rev = state.operations().snapshot().revision;
    assert!(state.uninstall(id, rev, intent.operation_id, true).is_err());
    assert_eq!(state.operations().snapshot().revision, rev);
    assert!(intent.destination.join("game/content").exists());
}
