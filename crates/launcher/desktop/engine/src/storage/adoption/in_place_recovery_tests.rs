//! Direct-root interruption and legacy relocation-journal compatibility.
use super::in_place_tests::*;
use super::*;

fn recover_now(f: &Fixture) -> Result<(), Error> {
    let (id, _) = f.operation();
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    recover(&mut state, id, revision)
}

#[test]
fn interrupted_adoption_recovery_keeps_existing_paths_and_file_ids() {
    let f = Fixture::new(None);
    let path = f.folder().join("later.txt");
    let before = inventory::identity(&path).unwrap();
    assert!(commit_with(f.preview().unwrap(), crash_after_first_move).is_err());
    let f = f.reopen();
    recover_now(&f).unwrap();
    let after = inventory::identity(&path).unwrap();
    assert_eq!((before.device, before.inode), (after.device, after.inode));
    assert!(!f.folder().join("game").exists());
    assert_player_files_untouched(&f);
}

#[test]
fn an_edit_after_interruption_is_preserved_and_refuses_full_verification() {
    let f = Fixture::new(None);
    assert!(commit_with(f.preview().unwrap(), crash_after_first_move).is_err());
    let f = f.reopen();
    std::fs::write(f.folder().join("later.txt"), b"changed after review").unwrap();
    assert_eq!(recover_now(&f), Err(StorageError::Corrupt.into()));
    assert_eq!(
        std::fs::read(f.folder().join("later.txt")).unwrap(),
        b"changed after review"
    );
    assert!(!f.folder().join("game").exists());
}

#[test]
fn extra_files_added_after_interruption_remain_at_their_paths() {
    let f = Fixture::new(None);
    assert!(commit_with(f.preview().unwrap(), crash_after_first_move).is_err());
    let f = f.reopen();
    std::fs::write(f.folder().join("new-notes.txt"), b"new notes").unwrap();
    recover_now(&f).unwrap();
    assert_eq!(
        std::fs::read(f.folder().join("new-notes.txt")).unwrap(),
        b"new notes"
    );
    assert_player_files_untouched(&f);
}

#[test]
fn legacy_interrupted_moves_are_never_resumed_and_abandon_only_unregisters() {
    use super::publication::{name, plan_name, write_large};
    let f = Fixture::new(None);
    assert!(commit_with(f.preview().unwrap(), crash_after_first_move).is_err());
    let (id, _) = f.operation();
    let state = f.state.lock().unwrap();
    let mut record = inspect(&state, id).unwrap();
    record.plan.schema_version = 1;
    record.plan.installation.schema_version = 1;
    let root = f.folder();
    let old_game = root.join("game");
    std::fs::create_dir(&old_game).unwrap();
    // Reconstruct a journal an older released launcher really could leave.
    std::fs::rename(root.join("later.txt"), old_game.join("later.txt")).unwrap();
    record.plan.stage_identity = inventory::identity(&old_game).unwrap();
    write_large(&state.directory.root, &plan_name(id), &record.plan).unwrap();
    write_large(&state.directory.root, &name(id), &record).unwrap();
    atomic::write(root, ".cimmeria-install.json", &record.plan.installation).unwrap();
    atomic::write(
        &state.directory.root,
        &format!(
            "install-intent-{}.json",
            record.plan.installation.operation_id
        ),
        &record.plan.installation,
    )
    .unwrap();
    let mut snapshot = state.operations().snapshot().clone();
    snapshot.operation.as_mut().unwrap().intent_digest = record.plan.digest().unwrap();
    atomic::write(&state.directory.root, "operation.json", &snapshot).unwrap();
    drop(state);
    let f = f.reopen();
    let before = f.scan();
    let mut state = f.state.lock().unwrap();
    let offered = interrupted(&state, id).unwrap();
    assert!(!offered.recoverable && offered.abandonable);
    let revision = state.operations().snapshot().revision;
    assert_eq!(
        recover(&mut state, id, revision),
        Err(ContractError::InvalidTransition.into())
    );
    assert_eq!(f.scan(), before);
    abandon(&mut state, id, revision).unwrap();
    drop(state);
    assert_eq!(
        std::fs::read(old_game.join("later.txt")).unwrap(),
        b"later entry"
    );
    assert!(!f.folder().join("later.txt").exists());
    assert!(!f.folder().join(".cimmeria-install.json").exists());
    assert_player_files_untouched(&f);
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
    assert!(inventory::native(f.folder(), "Working/Binaries/SGW.exe").is_file());
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
