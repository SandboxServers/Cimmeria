//! Windows zip swap recovery: a process that died mid-swap, a handoff that
//! was never recorded, and a retry past a stage an earlier attempt left.
use super::windows_zip::{stages, tree, windows_fixture, Fixture};
use super::*;

/// The fixture swap has seven renames: worker aside and in, logo aside and
/// in, banner in, exe aside and in.
const RENAMES: u32 = 7;

/// The record exactly as `apply` saves it before the first rename, then the
/// first `renames` renames done, as if the process died there.
fn interrupted(root: &Path, renames: u32) -> Fixture {
    let f = windows_fixture(root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    let Target::Windows { executable } = &f.target.0 else {
        unreachable!("the fixture is a Windows install")
    };
    let owner = Uuid::new_v4();
    let name = zip_swap::stage_name("1.1.0", owner);
    let stage = f.install.join(&name);
    let staged = zip_swap::extract(&f.bytes, &stage, owner).unwrap();
    let plan = zip_swap::plan(&f.install, executable, owner, staged).unwrap();
    let new_exe = zip_swap::executable_source(&plan).unwrap();
    let mut record = state.update_record().unwrap();
    record.phase = Phase::Installing;
    record.owner = Some(owner);
    record.apply = Some(Attempt {
        target: f.target.0.clone(),
        original: bundle::fingerprint(executable).unwrap(),
        replacement: Some(bundle::fingerprint(&stage.join(new_exe)).unwrap()),
        staged_bundle: Some(name),
        handoff: false,
        swap: Some(plan.clone()),
    });
    state.save_update(record).unwrap();
    zip_swap::FAIL_MOVE.with(|slot| slot.set(Some(renames + 1)));
    let swapped = zip_swap::swap_in(&f.install, &stage, &plan);
    zip_swap::FAIL_MOVE.with(|slot| slot.set(None));
    assert_eq!(swapped.is_ok(), renames == RENAMES, "{renames} renames");
    f
}

#[test]
fn reopen_after_a_crash_at_any_rename_restores_the_exact_original_tree() {
    // Six renames is the window with no launcher exe at its name at all.
    for renames in 0..=RENAMES {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let before = tree(&windows_fixture(&root.join("pristine")).install);
        let f = interrupted(&root, renames);
        let mut state = DesktopState::open(&root.join("state")).unwrap();
        state.reconcile_launcher_update(&f.target, "1.0.0").unwrap();
        assert_eq!(tree(&f.install), before, "{renames} renames: byte for byte");
        assert!(!f.install.join("graphics/new-art").exists(), "{renames}");
        assert!(stages(&f.install).is_empty(), "{renames}: stage removed");
        let snapshot = state.launcher_update_snapshot(Some(&f.config)).unwrap();
        assert_eq!(snapshot.phase, Phase::Failed, "{renames}");
        assert_eq!(snapshot.failure, Some(Error::Interrupted), "{renames}");
        assert!(state.ensure_updater_idle().is_ok(), "{renames}");
    }
}

#[test]
fn new_version_reopening_after_an_unrecorded_handoff_is_installed_and_cleans_up() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    // Every file moved, then the process died before saving the handoff.
    let f = interrupted(&root, RENAMES);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&f.target, "1.1.0").unwrap();
    let snapshot = state.launcher_update_snapshot(Some(&f.config)).unwrap();
    assert_eq!(snapshot.phase, Phase::Installed);
    assert!(state.ensure_updater_idle().is_ok());
    let installed = tree(&f.install);
    let names: Vec<&str> = installed.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "StargateWorlds-Launcher.exe",
            "graphics/logo.png",
            "graphics/new-art/banner.png",
            "player-notes.txt",
            "windows/cimmeria-archive-worker.exe",
        ],
        "no `.old` copies and no stage remain"
    );
    assert_eq!(installed["StargateWorlds-Launcher.exe"], b"new exe");
    assert!(stages(&f.install).is_empty());
}

#[test]
fn retrying_a_version_after_a_failed_attempt_left_its_stage_proceeds() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let f = windows_fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready_once = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    zip_swap::FAIL_MOVE.with(|slot| slot.set(Some(3)));
    let first = state.apply_update_with(
        Some(&f.config),
        &f.target,
        ready_once.offer.as_ref().unwrap().id,
        ready_once.revision,
        0,
        |_, _| panic!("relaunch after a failed swap"),
    );
    assert_eq!(first.unwrap_err(), Error::Replace);
    // The rollback's stage delete failed (Defender holding a new `.exe`), so
    // the failed attempt's stage, marker and all, is still on disk.
    let attempt = state.update_record().unwrap().apply.unwrap();
    let leftover = f.install.join(attempt.staged_bundle.unwrap());
    fs::create_dir_all(leftover.join("windows")).unwrap();
    fs::write(
        leftover.join(".cimmeria-owner"),
        zip_swap::plan_owner(attempt.swap.as_ref().unwrap()).to_string(),
    )
    .unwrap();
    fs::write(
        leftover.join("windows/cimmeria-archive-worker.exe"),
        b"held",
    )
    .unwrap();
    // An unmarked folder with a stage-like name was never the updater's.
    let foreign = f.install.join(".update-player-folder");
    fs::create_dir(&foreign).unwrap();
    fs::write(foreign.join("notes.txt"), b"mine").unwrap();
    // Still held through the retry, so the sweep cannot delete it either.
    #[cfg(windows)]
    let held = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(1) // FILE_SHARE_READ: no delete while open
            .open(leftover.join("windows/cimmeria-archive-worker.exe"))
            .unwrap()
    };

    let retry = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    let status = state
        .apply_update_with(
            Some(&f.config),
            &f.target,
            retry.offer.as_ref().unwrap().id,
            retry.revision,
            0,
            |_, _| Ok(()),
        )
        .unwrap();
    assert_eq!(status.phase, Phase::RestartRequired);
    assert_eq!(
        fs::read(f.install.join("StargateWorlds-Launcher.exe")).unwrap(),
        b"new exe"
    );
    #[cfg(windows)]
    {
        // The sweep failed part-way but kept the marker, so the next start
        // still recognizes the stage and removes it once it is released.
        assert!(leftover.join(".cimmeria-owner").is_file());
        drop(held);
        drop(state);
        let mut state = DesktopState::open(&root.join("state")).unwrap();
        state.reconcile_launcher_update(&f.target, "1.0.0").unwrap();
        let attempt = state.update_record().unwrap().apply.unwrap();
        assert_eq!(
            stages(&f.install),
            [
                attempt.staged_bundle.unwrap(),
                ".update-player-folder".into()
            ],
            "the in-flight attempt's stage is kept"
        );
    }
    assert!(!leftover.exists(), "the earlier attempt's stage was swept");
    assert_eq!(fs::read(foreign.join("notes.txt")).unwrap(), b"mine");
}
