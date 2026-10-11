//! In-place adoption (D-LX12) after the folder changed under an interrupted
//! adoption, and the refusals that keep a folder from being admitted at all.
use super::in_place_tests::*;
use super::*;

/// The release file the interrupted run moved, and where it was and is now.
struct Moved {
    old: PathBuf,
    game: PathBuf,
    bytes: Vec<u8>,
}
/// Crash after the first rename, restart, and copy that file back to its old
/// path, as the old launcher or a running game could.
fn duplicated() -> (Fixture, Moved) {
    let f = Fixture::new(None);
    assert!(commit_with(f.preview().unwrap(), crash_after_first_move).is_err());
    let f = f.reopen();
    let game = f.folder().join("game");
    let moved = inventory::scan(&game, &CancellationToken::new()).unwrap();
    assert_eq!(moved.len(), 1, "one file was moved before the crash");
    let path = moved.keys().next().unwrap().clone();
    let old = inventory::native(f.folder(), &path);
    let in_game = inventory::native(&game, &path);
    std::fs::copy(&in_game, &old).unwrap();
    let bytes = std::fs::read(&in_game).unwrap();
    (
        f,
        Moved {
            old,
            game: in_game,
            bytes,
        },
    )
}
fn recover_now(f: &Fixture) -> Result<(), Error> {
    let (id, _) = f.operation();
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    recover(&mut state, id, revision)
}
fn owner(f: &Fixture) -> Uuid {
    let marker = std::fs::read(f.folder().join(".cimmeria-install.json")).unwrap();
    serde_json::from_slice::<InstallIntent>(&marker)
        .unwrap()
        .operation_id
}

#[test]
fn a_release_file_recreated_at_its_old_path_is_left_there_and_recovery_succeeds() {
    let (f, moved) = duplicated();
    recover_now(&f).unwrap();
    assert_eq!(f.operation().1, OperationState::Succeeded);
    assert_eq!(std::fs::read(&moved.game).unwrap(), moved.bytes);
    // Never deleted: it is now a file the release does not list.
    assert_eq!(std::fs::read(&moved.old).unwrap(), moved.bytes);
    assert!(f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_some());
    assert_player_files_untouched(&f);
}

#[test]
fn when_only_the_old_path_copy_verifies_it_replaces_the_copy_in_game_and_the_other_is_kept() {
    let (f, moved) = duplicated();
    std::fs::write(&moved.game, b"damaged after the move").unwrap();
    let owner = owner(&f);
    recover_now(&f).unwrap();
    assert_eq!(f.operation().1, OperationState::Succeeded);
    assert_eq!(std::fs::read(&moved.game).unwrap(), moved.bytes);
    assert!(!moved.old.exists());
    let mut aside = moved.old.clone().into_os_string();
    aside.push(format!(".adopt-conflict-{owner}"));
    assert_eq!(std::fs::read(aside).unwrap(), b"damaged after the move");
}

#[test]
fn when_neither_copy_verifies_recovery_stops_and_changes_nothing() {
    let (f, moved) = duplicated();
    std::fs::write(&moved.game, b"one changed copy").unwrap();
    std::fs::write(&moved.old, b"another changed copy").unwrap();
    assert_eq!(recover_now(&f), Err(Error::DuplicateReleaseFile));
    assert_eq!(std::fs::read(&moved.game).unwrap(), b"one changed copy");
    assert_eq!(std::fs::read(&moved.old).unwrap(), b"another changed copy");
    assert_eq!(f.operation().1, OperationState::ReconciliationRequired);
    assert!(f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_none());
}

#[test]
fn abandoning_with_a_file_in_both_places_keeps_the_folders_copy_and_leaves_the_other_in_game() {
    let (f, moved) = duplicated();
    {
        let (id, _) = f.operation();
        let mut state = f.state.lock().unwrap();
        let revision = state.operations().snapshot().revision;
        abandon(&mut state, id, revision).unwrap();
    }
    assert_eq!(f.operation().1, OperationState::Cancelled);
    assert_eq!(std::fs::read(&moved.old).unwrap(), moved.bytes);
    assert_eq!(std::fs::read(&moved.game).unwrap(), moved.bytes);
    assert!(!f.folder().join(".cimmeria-install.json").exists());
    assert_player_files_untouched(&f);
    // The non-empty game folder is not a harmless leftover.
    assert_eq!(f.preview().err(), Some(Error::LeftoverLauncherFiles));
}

/// Setup rewrites release bytes, so once it may have run only recovery is
/// offered: putting the files back would not put the folder back as reviewed.
#[test]
fn after_setup_an_interrupted_adoption_is_recovered_and_never_abandoned() {
    let f = Fixture::new(None);
    let crash_after_setup = |point| {
        if point == publication::Point::SetUp {
            Err(StorageError::Io.into())
        } else {
            Ok(())
        }
    };
    assert!(commit_with(f.preview().unwrap(), crash_after_setup).is_err());
    let f = f.reopen();
    let (id, phase) = f.operation();
    assert_eq!(phase, OperationState::ReconciliationRequired);
    let mut state = f.state.lock().unwrap();
    let interrupted = interrupted(&state, id).unwrap();
    assert!(interrupted.recoverable);
    assert!(!interrupted.abandonable);
    let revision = state.operations().snapshot().revision;
    assert_eq!(
        abandon(&mut state, id, revision),
        Err(ContractError::InvalidTransition.into())
    );
    let revision = state.operations().snapshot().revision;
    recover(&mut state, id, revision).unwrap();
    assert!(state.installed_content().unwrap().is_some());
}

#[cfg(windows)]
#[test]
fn a_running_game_is_refused_before_anything_is_admitted() {
    let f = Fixture::new(None);
    let exe = inventory::native(f.folder(), "Working/Binaries/SGW.exe");
    // A running game holds its executable open; any open handle stands in.
    let held = std::fs::File::open(&exe).unwrap();
    assert_eq!(commit(f.preview().unwrap()), Err(Error::GameRunning));
    for name in ["game", ".cimmeria-install.json", "content-ready.json"] {
        assert!(!f.folder().join(name).exists(), "{name} was created");
    }
    assert!(exe.is_file());
    assert!(f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_none());
    drop(held);
    commit(f.preview().unwrap()).unwrap();
}

#[test]
fn a_folder_outside_the_full_layout_is_refused_once_before_any_download() {
    let f = Fixture::new(None);
    // The old launcher's install path pointed straight at the binaries.
    std::fs::rename(
        inventory::native(f.folder(), "Working/Binaries/SGW.exe"),
        f.folder().join("SGW.exe"),
    )
    .unwrap();
    assert_eq!(f.preview().err(), Some(Error::UnsupportedLayout));
    let state = f.state.lock().unwrap();
    assert!(list_preparations(&state).unwrap().is_empty());
}

/// What an admission leaves if the launcher dies after the marker and before
/// its operation begins.
fn plant_unbegun_admission(f: &Fixture) {
    let intent = InstallIntent {
        schema_version: 1,
        operation_id: Uuid::new_v4(),
        preferences_revision: 0,
        destination: f.folder().to_path_buf(),
        manifest_digest: f.release.digest(),
        login_servers: vec![],
        backend: ExtractionBackend::Native,
    };
    std::fs::write(
        f.folder().join(".cimmeria-install.json"),
        serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    std::fs::create_dir(f.folder().join("game")).unwrap();
}

#[test]
fn a_marker_and_empty_game_from_an_admission_that_never_began_are_taken_back() {
    let f = Fixture::new(None);
    plant_unbegun_admission(&f);
    commit(f.preview().unwrap()).unwrap();
    assert!(f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_some());
    assert!(inventory::native(f.folder(), "game/Working/Binaries/SGW.exe").is_file());
}

#[test]
fn launcher_owned_names_holding_anything_are_refused_and_left_alone() {
    let f = Fixture::new(None);
    plant_unbegun_admission(&f);
    std::fs::write(
        f.folder().join("game").join("kept.txt"),
        b"not ours to remove",
    )
    .unwrap();
    assert_eq!(f.preview().err(), Some(Error::LeftoverLauncherFiles));
    assert!(f.folder().join(".cimmeria-install.json").is_file());
    assert_eq!(
        std::fs::read(f.folder().join("game").join("kept.txt")).unwrap(),
        b"not ours to remove"
    );
}
