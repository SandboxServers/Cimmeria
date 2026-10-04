use super::*;
use crate::catalog::verify_release;
use ed25519_dalek::{Signer, SigningKey};
fn fixture() -> (tempfile::TempDir, DesktopState, Uuid, u64) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("game")), false, 0)
        .unwrap();
    let body = br#"{"schema":1,"seed":{"blob":"seed.zip","size":1,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"patches":[]}"#;
    let signature = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(body);
    let hex: String = signature
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let release = verify_release(body, hex.as_bytes()).unwrap();
    let id = Uuid::new_v4();
    let intent = state
        .admit_install(id, 0, 1, &release, vec![])
        .unwrap()
        .intent;
    std::fs::create_dir(&intent.destination).unwrap();
    atomic::write(&intent.destination, ".cimmeria-install.json", &intent).unwrap();
    std::fs::create_dir(intent.destination.join(format!(".cimmeria-stage-{id}"))).unwrap();
    std::fs::write(
        intent
            .destination
            .join(format!(".cimmeria-stage-{id}/partial")),
        "bytes",
    )
    .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Failed)
        .unwrap();
    let revision = state.operations.snapshot().revision;
    (root, state, id, revision)
}
#[test]
fn cleanup_is_explicit_idempotent_and_allows_a_fresh_admission() {
    let (root, mut state, id, revision) = fixture();
    assert!(!state.can_retry_install());
    state.clean_failed_install(id, revision).unwrap();
    state.clean_failed_install(id, revision).unwrap();
    assert!(state.can_retry_install());
    let release = state.cached_install_release().unwrap();
    let next = Uuid::new_v4();
    let admission = state
        .admit_install(next, revision, 1, &release, vec![])
        .unwrap();
    assert!(admission.dispatch);
    assert_eq!(
        std::fs::read_dir(root.path().join("game")).unwrap().count(),
        0
    );
}
#[test]
fn foreign_or_promoted_files_veto_before_deleting_partial_data() {
    for name in [
        "game",
        "content-ready.json",
        "notes.txt",
        ".cimmeria-stage-other",
    ] {
        let (root, mut state, id, revision) = fixture();
        std::fs::write(root.path().join("game").join(name), "preserve").unwrap();
        assert!(state.clean_failed_install(id, revision).is_err());
        assert!(root
            .path()
            .join(format!("game/.cimmeria-stage-{id}/partial"))
            .exists());
    }
}
#[test]
fn stale_requests_and_changed_owner_cannot_delete() {
    let (root, mut state, id, revision) = fixture();
    assert!(state.clean_failed_install(id, revision + 1).is_err());
    assert!(state
        .clean_failed_install(Uuid::new_v4(), revision)
        .is_err());
    std::fs::write(root.path().join("game/.cimmeria-install.json"), "{}").unwrap();
    assert!(state.clean_failed_install(id, revision).is_err());
    assert!(root
        .path()
        .join(format!("game/.cimmeria-stage-{id}/partial"))
        .exists());
}

#[cfg(unix)]
#[test]
fn redirected_stage_and_nested_links_preserve_external_files() {
    let (root, mut state, id, revision) = fixture();
    let stage = root.path().join(format!("game/.cimmeria-stage-{id}"));
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep"), "preserve").unwrap();
    std::os::unix::fs::symlink(&outside, stage.join("link")).unwrap();
    assert!(state.clean_failed_install(id, revision).is_err());
    assert!(outside.join("keep").exists());

    let (root, mut state, id, revision) = fixture();
    let stage = root.path().join(format!("game/.cimmeria-stage-{id}"));
    std::fs::remove_dir_all(&stage).unwrap();
    std::os::unix::fs::symlink(&outside, &stage).unwrap();
    assert!(state.clean_failed_install(id, revision).is_err());
    assert!(outside.join("keep").exists());
}

#[test]
fn live_owner_lock_prevents_cleanup() {
    let (root, mut state, id, revision) = fixture();
    let owner = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join("game/.cimmeria-install.json"))
        .unwrap();
    owner.try_lock().unwrap();
    assert!(state.clean_failed_install(id, revision).is_err());
    assert!(root
        .path()
        .join(format!("game/.cimmeria-stage-{id}/partial"))
        .exists());
}
#[cfg(windows)]
#[test]
fn windows_junction_cannot_redirect_partial_cleanup() {
    let (root, mut state, id, revision) = fixture();
    let stage = root.path().join(format!("game/.cimmeria-stage-{id}"));
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep"), "preserve").unwrap();
    let junction = stage.join("junction");
    assert!(std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .status()
        .unwrap()
        .success());
    assert!(state.clean_failed_install(id, revision).is_err());
    assert!(outside.join("keep").exists());
    std::fs::remove_dir(junction).unwrap();
}
