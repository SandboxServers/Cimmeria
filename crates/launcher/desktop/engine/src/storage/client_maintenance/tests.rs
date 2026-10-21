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
fn a_firesky_that_is_a_link_is_refused_by_both_resets_and_nothing_is_deleted() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(
        &outside.path().join("SGWGame/Cache.en-US/keep.pak"),
        b"outside",
    );
    let firesky = root.path().join("Documents/My Games/Firesky");
    if !link_dir(outside.path(), &firesky) {
        return;
    }
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    for request in [reset_cache(), reset_all(true)] {
        assert_eq!(
            state.client_maintenance_in(request, &firesky),
            Err(ClientMaintenanceError::UnsafePath),
            "{request:?}"
        );
        // Neither the link nor what it points at is removed.
        assert!(fs::symlink_metadata(&firesky).is_ok(), "{request:?}");
        assert_eq!(
            fs::read(outside.path().join("SGWGame/Cache.en-US/keep.pak")).unwrap(),
            b"outside"
        );
    }
}

#[test]
fn a_linked_my_games_is_refused_by_both_resets_and_nothing_is_deleted() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let pak = outside.path().join("Firesky/SGWGame/Cache.en-US/keep.pak");
    write(&pak, b"outside");
    let my_games = root.path().join("Documents/My Games");
    if !link_dir(outside.path(), &my_games) {
        return;
    }
    let firesky = my_games.join("Firesky");
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    for request in [reset_cache(), reset_all(true)] {
        assert_eq!(
            state.client_maintenance_in(request, &firesky),
            Err(ClientMaintenanceError::UnsafePath),
            "{request:?}"
        );
        assert_eq!(fs::read(&pak).unwrap(), b"outside", "{request:?}");
    }
}

#[test]
fn a_cache_folder_that_is_a_link_is_removed_as_the_link_and_its_target_survives() {
    let profile = profile();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("keep.pak"), b"outside");
    let cache = profile.firesky.join("SGWGame/Cache.en-US");
    fs::remove_dir_all(&cache).unwrap();
    if !link_dir(outside.path(), &cache) {
        return;
    }
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    let status = state
        .client_maintenance_in(reset_cache(), &profile.firesky)
        .unwrap();
    assert_eq!(status.removed, Some(true));
    assert!(fs::symlink_metadata(&cache).is_err());
    assert_eq!(
        fs::read(outside.path().join("keep.pak")).unwrap(),
        b"outside"
    );
    assert!(profile.firesky.join("SGWGame/Settings.ini").exists());
}

#[test]
fn a_link_deep_in_the_cache_is_removed_as_a_link_and_its_target_survives() {
    let profile = profile();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("keep.pak"), b"outside");
    let link = profile.firesky.join("SGWGame/Cache.en-US/nested/linked");
    if !link_dir(outside.path(), &link) {
        return;
    }
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    state
        .client_maintenance_in(reset_cache(), &profile.firesky)
        .unwrap();
    assert!(!profile.firesky.join("SGWGame/Cache.en-US").exists());
    assert_eq!(
        fs::read(outside.path().join("keep.pak")).unwrap(),
        b"outside"
    );
}

/// std must report a junction as a link (its mount-point tag is a name
/// surrogate), now that no reparse-attribute check backs it up.
#[cfg(windows)]
#[test]
fn a_junction_counts_as_a_link() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(
        &outside.path().join("SGWGame/Cache.en-US/keep.pak"),
        b"outside",
    );
    let firesky = root.path().join("Documents/My Games/Firesky");
    fs::create_dir_all(firesky.parent().unwrap()).unwrap();
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&firesky)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(made.status.success(), "mklink /J: {made:?}");
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state.client_maintenance_in(reset_all(true), &firesky),
        Err(ClientMaintenanceError::UnsafePath)
    );
    assert!(outside.path().join("SGWGame/Cache.en-US/keep.pak").exists());
}

#[test]
fn a_reset_that_stops_part_way_returns_io_and_logs_the_error_kind_without_a_path() {
    let profile = profile();
    // Spelled as `cache_dir` joins it, so a logged path would match.
    let cache = profile.firesky.join("SGWGame").join("Cache.en-US");
    let held = cache.join("nested/deep.pak");
    let Some(_hold) = hold_undeletable(&held) else {
        return;
    };
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    let (result, logs) =
        capture_logs(|| state.client_maintenance_in(reset_cache(), &profile.firesky));
    assert_eq!(
        result,
        Err(ClientMaintenanceError::Storage(StorageError::Io))
    );
    assert!(held.exists());
    assert!(
        logs.iter().any(|line| line.starts_with("WARN")
            && line.contains("event=\"client_maintenance_removal_failed\"")
            && line.contains("error_kind=")),
        "{logs:#?}"
    );
    // R5: no field or message carries a path, so none of the profile's text appears.
    let cache_text = cache.display().to_string();
    let root_text = profile._root.path().display().to_string();
    assert!(
        !logs
            .iter()
            .any(|line| line.contains(&cache_text) || line.contains(&root_text)),
        "{logs:#?}"
    );
    assert!(
        !logs.iter().any(|line| line.contains("deleted")),
        "{logs:#?}"
    );
}

#[test]
fn a_successful_reset_logs_the_outcome_without_a_path() {
    let profile = profile();
    let mut state = DesktopState::open(&profile._root.path().join("state")).unwrap();
    // Through the fixture, so the `client_maintenance_at` wrapper logs the outcome.
    let (result, logs) =
        capture_logs(|| state.client_maintenance_fixture(reset_all(true), &profile.firesky));
    assert_eq!(result.unwrap().removed, Some(true));
    assert!(
        logs.iter().any(|line| line.starts_with("INFO")
            && line.contains("event=\"client_maintenance_finished\"")
            && line.contains("action=\"reset_all_client_state\"")
            && line.contains("removed=true")
            && line.contains("outcome=\"ok\"")
            && line.contains("duration_ms=")),
        "{logs:#?}"
    );
    // R5: the removed folder's path must not reach any row.
    let firesky_text = profile.firesky.display().to_string();
    let root_text = profile._root.path().display().to_string();
    assert!(
        !logs
            .iter()
            .any(|line| line.contains(&firesky_text) || line.contains(&root_text)),
        "{logs:#?}"
    );
}

/// Off Windows the client's Documents is inside the Wine prefix, so the
/// resolved `$HOME/Documents` is never acted on.
#[cfg(not(windows))]
#[test]
fn off_windows_every_request_is_profile_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    for request in [
        ClientMaintenanceCommand::Inspect { schema_version: 1 },
        reset_cache(),
        reset_all(true),
    ] {
        assert_eq!(
            state.client_maintenance(request),
            Err(ClientMaintenanceError::ProfileUnavailable),
            "{request:?}"
        );
    }
}

/// Makes `path` undeletable until the guard drops: held open without delete
/// sharing on Windows, its folder made read-only on Unix. `None` where the
/// host deletes it anyway (root on Unix).
fn hold_undeletable(path: &Path) -> Option<Box<dyn std::any::Any>> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let file = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .unwrap();
        Some(Box::new(file))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
            }
        }
        let folder = path.parent().unwrap().to_path_buf();
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
        let restore = Restore(folder);
        let probe = path.with_extension("probe");
        if fs::write(&probe, b"").is_ok() {
            let _ = fs::remove_file(&probe);
            eprintln!("skipped: this host can write a read-only folder");
            return None;
        }
        Some(Box::new(restore))
    }
}

/// Runs `run` with a subscriber that keeps each event as `LEVEL field=value ...`.
fn capture_logs<T>(run: impl FnOnce() -> T) -> (T, Vec<String>) {
    use std::sync::{Arc, Mutex};
    use tracing::{field, span, Event, Metadata};
    struct Capture(Arc<Mutex<Vec<String>>>);
    struct Text(String);
    impl field::Visit for Text {
        fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
            self.0.push_str(&format!(" {}={value:?}", field.name()));
        }
    }
    impl tracing::Subscriber for Capture {
        // sometimes, not the default lways: another test may already have cached
        // a callsite as uninteresting while no subscriber was set.
        fn register_callsite(
            &self,
            _: &'static Metadata<'static>,
        ) -> tracing::subscriber::Interest {
            tracing::subscriber::Interest::sometimes()
        }
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }
        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, event: &Event<'_>) {
            let mut text = Text(event.metadata().level().to_string());
            event.record(&mut text);
            self.0.lock().unwrap().push(text.0);
        }
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }
    let lines = Arc::new(Mutex::new(Vec::new()));
    // With one live Dispatch, tracing-core takes a new callsite's interest from
    // whichever thread hits it first, often another test's with no subscriber.
    // A second live Dispatch keeps interest per-dispatcher (see agent memory
    // tracing-capture-single-dispatcher-trap).
    let _second = tracing::Dispatch::new(Capture(Arc::new(Mutex::new(Vec::new()))));
    let value = tracing::subscriber::with_default(Capture(lines.clone()), || {
        // Callsite interest is cached process-wide; recompute it with this capture
        // installed, or an event first hit by an earlier test stays silent.
        tracing::callsite::rebuild_interest_cache();
        run()
    });
    let lines = lines.lock().unwrap().clone();
    (value, lines)
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
