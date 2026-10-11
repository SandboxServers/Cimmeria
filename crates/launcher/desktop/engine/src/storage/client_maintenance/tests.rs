use super::*;

/// A temp profile with the `My Games\Firesky` tree and a sibling that must
/// survive every reset.
struct Profile {
    _root: tempfile::TempDir,
    firesky: PathBuf,
    sibling: PathBuf,
}

fn profile() -> Profile {
    let root = tempfile::tempdir().unwrap();
    let firesky = root.path().join("Documents/My Games/Firesky");
    let sibling = root.path().join("Documents/My Games/Other/keep.txt");
    write(&firesky.join("SGWGame/Cache.en-US/override.pak"), b"cache");
    write(
        &firesky.join("SGWGame/Cache.en-US/nested/deep.pak"),
        b"cache",
    );
    write(&firesky.join("SGWGame/Settings.ini"), b"state");
    write(&firesky.join("Logs/last.log"), b"state");
    write(&sibling, b"keep");
    Profile {
        _root: root,
        firesky,
        sibling,
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn reset_cache() -> ClientMaintenanceCommand {
    ClientMaintenanceCommand::ResetCache { schema_version: 1 }
}

fn reset_all(confirmed: bool) -> ClientMaintenanceCommand {
    ClientMaintenanceCommand::ResetAllClientState {
        schema_version: 1,
        confirmed,
    }
}

#[test]
fn reset_cache_removes_exactly_the_cache_tree() {
    let profile = profile();
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    let status = state
        .client_maintenance_in(reset_cache(), &profile.firesky)
        .unwrap();
    assert_eq!(status.removed, Some(true));
    assert!(!status.cache.exists);
    assert!(status.client_state.exists);
    assert!(!profile.firesky.join("SGWGame/Cache.en-US").exists());
    assert!(profile.firesky.join("SGWGame/Settings.ini").exists());
    assert!(profile.firesky.join("Logs/last.log").exists());
    assert!(profile.sibling.exists());
}

#[test]
fn reset_all_removes_the_firesky_tree_and_nothing_beside_it() {
    let profile = profile();
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    let status = state
        .client_maintenance_in(reset_all(true), &profile.firesky)
        .unwrap();
    assert_eq!(status.removed, Some(true));
    assert!(!status.client_state.exists);
    assert!(!profile.firesky.exists());
    assert_eq!(fs::read(&profile.sibling).unwrap(), b"keep");
}

#[test]
fn unconfirmed_reset_deletes_nothing() {
    let profile = profile();
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    assert_eq!(
        state.client_maintenance_in(reset_all(false), &profile.firesky),
        Err(ClientMaintenanceError::ConfirmationRequired)
    );
    assert!(profile
        .firesky
        .join("SGWGame/Cache.en-US/override.pak")
        .exists());
    assert!(profile.firesky.join("SGWGame/Settings.ini").exists());
    assert!(profile.firesky.join("Logs/last.log").exists());
    assert_eq!(fs::read(&profile.sibling).unwrap(), b"keep");
}

#[test]
fn a_reset_with_nothing_to_clear_says_so() {
    let root = tempfile::tempdir().unwrap();
    let firesky = root.path().join("Firesky");
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let inspect = state
        .client_maintenance_in(
            ClientMaintenanceCommand::Inspect { schema_version: 1 },
            &firesky,
        )
        .unwrap();
    assert_eq!(inspect.removed, None);
    assert!(!inspect.cache.exists && !inspect.client_state.exists);
    assert_eq!(
        state
            .client_maintenance_in(reset_cache(), &firesky)
            .unwrap()
            .removed,
        Some(false)
    );
    assert_eq!(
        state
            .client_maintenance_in(reset_all(true), &firesky)
            .unwrap()
            .removed,
        Some(false)
    );
}

#[test]
fn a_link_inside_the_tree_is_removed_as_a_link_and_its_target_survives() {
    let profile = profile();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("target.txt"), b"outside");
    if !link_dir(outside.path(), &profile.firesky.join("Linked")) {
        return;
    }
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    state
        .client_maintenance_in(reset_all(true), &profile.firesky)
        .unwrap();
    assert!(!profile.firesky.exists());
    assert_eq!(
        fs::read(outside.path().join("target.txt")).unwrap(),
        b"outside"
    );
}

#[test]
fn a_cache_reached_through_a_link_is_refused_and_nothing_is_deleted() {
    let profile = profile();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("Cache.en-US/keep.pak"), b"outside");
    fs::remove_dir_all(profile.firesky.join("SGWGame")).unwrap();
    if !link_dir(outside.path(), &profile.firesky.join("SGWGame")) {
        return;
    }
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    assert_eq!(
        state.client_maintenance_in(reset_cache(), &profile.firesky),
        Err(ClientMaintenanceError::UnsafePath)
    );
    assert_eq!(
        fs::read(outside.path().join("Cache.en-US/keep.pak")).unwrap(),
        b"outside"
    );
}

#[test]
fn a_firesky_that_is_a_link_is_removed_as_the_link_and_its_target_survives() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(
        &outside.path().join("SGWGame/Cache.en-US/keep.pak"),
        b"outside",
    );
    let firesky = root.path().join("Firesky");
    if !link_dir(outside.path(), &firesky) {
        return;
    }
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    // The cache reset would delete through the link, so it is refused.
    assert_eq!(
        state.client_maintenance_in(reset_cache(), &firesky),
        Err(ClientMaintenanceError::UnsafePath)
    );
    let status = state
        .client_maintenance_in(reset_all(true), &firesky)
        .unwrap();
    assert_eq!(status.removed, Some(true));
    assert!(fs::symlink_metadata(&firesky).is_err());
    assert_eq!(
        fs::read(outside.path().join("SGWGame/Cache.en-US/keep.pak")).unwrap(),
        b"outside"
    );
}

#[test]
fn a_read_only_file_does_not_stop_a_reset() {
    let profile = profile();
    let pak = profile.firesky.join("SGWGame/Cache.en-US/override.pak");
    let mut permissions = fs::metadata(&pak).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&pak, permissions).unwrap();
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    state
        .client_maintenance_in(reset_cache(), &profile.firesky)
        .unwrap();
    assert!(!profile.firesky.join("SGWGame/Cache.en-US").exists());
}

/// Creates a directory link, or returns `false` where the host does not allow
/// one. On Windows a symlink needs developer mode or elevation, so a
/// junction (which needs neither) is the fallback.
fn link_dir(target: &Path, link: &Path) -> bool {
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(target, link).is_ok();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(target, link).is_ok()
        || std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|out| out.status.success());
    if !made {
        eprintln!("skipped: this host cannot create directory links");
    }
    made
}

#[test]
fn errors_serialize_as_the_flat_codes_the_bridge_accepts() {
    for (error, code) in [
        (
            ClientMaintenanceError::ConfirmationRequired,
            "confirmation_required",
        ),
        (
            ClientMaintenanceError::ProfileUnavailable,
            "profile_unavailable",
        ),
        (ClientMaintenanceError::UnsafePath, "unsafe_path"),
        (ClientMaintenanceError::Storage(StorageError::Busy), "busy"),
        (
            ClientMaintenanceError::Storage(StorageError::UnsupportedSchema),
            "unsupported_schema",
        ),
    ] {
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!(code)
        );
    }
}

#[test]
fn unknown_schema_and_unknown_fields_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        state.client_maintenance_in(
            ClientMaintenanceCommand::ResetCache { schema_version: 2 },
            &root.path().join("Firesky"),
        ),
        Err(ClientMaintenanceError::Storage(
            StorageError::UnsupportedSchema
        ))
    );
    assert!(serde_json::from_str::<ClientMaintenanceCommand>(
        r#"{"command":"reset_all_client_state","schema_version":1,"confirmed":true,"path":"/other"}"#
    )
    .is_err());
    // A reset with no confirmation field is not a confirmed reset.
    assert!(serde_json::from_str::<ClientMaintenanceCommand>(
        r#"{"command":"reset_all_client_state","schema_version":1}"#
    )
    .is_err());
}
