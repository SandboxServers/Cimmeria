use super::*;
use crate::{OperationKind, OperationState};
use std::{
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[test]
fn preferences_survive_reopen_and_default_consent_is_off() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    assert!(!state.preferences().launcher_summary_consent);
    let install = dir.path().join("Game");
    let saved = state.save_preferences(Some(install), true, 0).unwrap();
    drop(state);
    let mut reopened = DesktopState::open(dir.path()).unwrap();
    assert_eq!(reopened.preferences(), &saved);
    assert_eq!(
        reopened.save_preferences(None, false, 0),
        Err(StorageError::StaleRevision)
    );
    assert_eq!(reopened.preferences(), &saved);
    let opted_out = reopened
        .save_preferences(saved.install_directory, false, 1)
        .unwrap();
    drop(reopened);
    assert_eq!(
        DesktopState::open(dir.path()).unwrap().preferences(),
        &opted_out
    );
}

#[test]
fn interrupted_journal_reopens_for_reconciliation_without_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    let id = Uuid::new_v4();
    state
        .operations_mut()
        .unwrap()
        .begin(id, OperationKind::Install, [1; 32], 0)
        .unwrap();
    drop(state);
    let mut reopened = DesktopState::open(dir.path()).unwrap();
    assert_eq!(reopened.operations().snapshot().revision, 2);
    assert_eq!(
        reopened
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert_eq!(
        reopened.operations_mut().unwrap().begin(
            Uuid::new_v4(),
            OperationKind::Install,
            [1; 32],
            2
        ),
        Err(ContractError::Busy)
    );
    reopened
        .operations_mut()
        .unwrap()
        .reconcile(id, OperationState::Succeeded)
        .unwrap();
    let terminal = reopened.operations().snapshot().clone();
    drop(reopened);
    assert_eq!(
        DesktopState::open(dir.path())
            .unwrap()
            .operations()
            .snapshot(),
        &terminal
    );
}

#[test]
fn install_directory_is_locked_during_operation_but_consent_is_not() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    let install = dir.path().join("game");
    state
        .save_preferences(Some(install.clone()), false, 0)
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .begin(Uuid::new_v4(), OperationKind::Install, [1; 32], 0)
        .unwrap();
    assert_eq!(
        state.save_preferences(Some(dir.path().join("other")), false, 1),
        Err(StorageError::Busy)
    );
    assert!(
        state
            .save_preferences(Some(install), true, 1)
            .unwrap()
            .launcher_summary_consent
    );
}

#[test]
fn invalid_and_future_files_are_not_reset_or_overwritten() {
    for (name, bytes, expected) in [
        ("operation.json", b"{truncated".as_slice(), StorageError::Corrupt),
        ("operation.json", br#"{"schema_version":99,"revision":0,"operation":null}"#.as_slice(), StorageError::UnsupportedSchema),
        ("preferences.json", br#"{"schema_version":99,"revision":0,"install_directory":null,"launcher_summary_consent":false}"#.as_slice(), StorageError::UnsupportedSchema),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(DesktopState::open(dir.path()), Err(error) if error == expected));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn oversized_file_and_relative_install_path_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    assert_eq!(
        state.save_preferences(Some("relative/game".into()), true, 0),
        Err(StorageError::InvalidDirectory)
    );
    assert_eq!(state.preferences(), &Preferences::default());
    drop(state);
    std::fs::write(
        dir.path().join("operation.json"),
        vec![b' '; MAX_STATE_BYTES as usize + 1],
    )
    .unwrap();
    assert!(matches!(
        DesktopState::open(dir.path()),
        Err(StorageError::TooLarge)
    ));
}

#[test]
fn failed_replace_preserves_old_preferences_and_does_not_acknowledge() {
    let dir = tempfile::tempdir().unwrap();
    let old = Preferences::default();
    atomic::write(dir.path(), "preferences.json", &old).unwrap();
    let next = Preferences {
        revision: 1,
        launcher_summary_consent: true,
        ..old.clone()
    };
    assert_eq!(
        atomic::write_with(dir.path(), "preferences.json", &next, |stage| {
            if stage == atomic::Checkpoint::BeforeReplace {
                Err(std::io::Error::other("injected disk failure"))
            } else {
                Ok(())
            }
        }),
        Err(StorageError::Io)
    );
    assert_eq!(
        read::<Preferences>(&dir.path().join("preferences.json")).unwrap(),
        Some(old)
    );
    // tempfile cleanup must not leave a staged file on normal error unwinding.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn failed_sync_after_replace_is_uncertain_and_new_file_is_inspectable() {
    let dir = tempfile::tempdir().unwrap();
    atomic::write(dir.path(), "preferences.json", &Preferences::default()).unwrap();
    let next = Preferences {
        revision: 1,
        launcher_summary_consent: true,
        ..Preferences::default()
    };
    assert_eq!(
        atomic::write_with(dir.path(), "preferences.json", &next, |stage| {
            if stage == atomic::Checkpoint::AfterReplace {
                Err(std::io::Error::other("injected sync failure"))
            } else {
                Ok(())
            }
        }),
        Err(StorageError::PersistenceUncertain)
    );
    assert_eq!(
        read::<Preferences>(&dir.path().join("preferences.json")).unwrap(),
        Some(next)
    );
}

#[test]
fn preference_write_error_keeps_last_confirmed_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    std::fs::create_dir(dir.path().join("preferences.json")).unwrap();
    assert_eq!(
        state.save_preferences(None, true, 0),
        Err(StorageError::UnsafeFile)
    );
    assert_eq!(state.preferences(), &Preferences::default());
    assert!(!state.requires_reopen());
}

#[cfg(unix)]
#[test]
fn symlink_state_does_not_read_or_modify_another_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("outside.json");
    std::fs::write(&target, b"untouched").unwrap();
    std::os::unix::fs::symlink(&target, dir.path().join("preferences.json")).unwrap();
    assert!(matches!(
        DesktopState::open(dir.path()),
        Err(StorageError::UnsafeFile)
    ));
    assert_eq!(std::fs::read(target).unwrap(), b"untouched");
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn process_lock_survives_content_replacement_and_releases_after_process_death() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "storage::tests::child_holds_lock",
                "--ignored",
                "--nocapture",
            ])
            .env("CIMMERIA_STATE_TEST_DIRECTORY", dir.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !dir.path().join("ready").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before acquiring lock"
        );
        assert!(Instant::now() < deadline, "child did not acquire lock");
        thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(
        DesktopState::open(dir.path()),
        Err(StorageError::InUse)
    ));
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let reopened = DesktopState::open(dir.path()).unwrap();
    assert_eq!(
        reopened
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert_eq!(reopened.preferences().revision, 2);
    assert!(!reopened.preferences().launcher_summary_consent);
}

#[test]
#[ignore = "subprocess fixture invoked by process_lock test"]
fn child_holds_lock() {
    let root = std::env::var_os("CIMMERIA_STATE_TEST_DIRECTORY")
        .expect("parent supplies isolated directory");
    let mut state = DesktopState::open(Path::new(&root)).unwrap();
    state.save_preferences(None, true, 0).unwrap();
    state.save_preferences(None, false, 1).unwrap();
    state
        .operations_mut()
        .unwrap()
        .begin(Uuid::new_v4(), OperationKind::Install, [1; 32], 0)
        .unwrap();
    std::fs::write(Path::new(&root).join("ready"), b"ready").unwrap();
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

#[test]
fn uncertain_preference_save_blocks_operations_until_disk_is_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    let result = state.save_preferences_with(None, true, 0, |root, next| {
        atomic::write_with(root, "preferences.json", next, |stage| {
            if stage == atomic::Checkpoint::AfterReplace {
                Err(std::io::Error::other("injected sync failure"))
            } else {
                Ok(())
            }
        })
    });
    assert_eq!(result, Err(StorageError::PersistenceUncertain));
    assert!(state.requires_reopen());
    assert_eq!(state.preferences(), &Preferences::default());
    assert!(matches!(
        state.operations_mut(),
        Err(StorageError::PersistenceUncertain)
    ));
    assert_eq!(
        state.save_preferences(None, false, 0),
        Err(StorageError::PersistenceUncertain)
    );
    drop(state);
    let reopened = DesktopState::open(dir.path()).unwrap();
    assert!(!reopened.requires_reopen());
    assert!(reopened.preferences().launcher_summary_consent);
    assert_eq!(reopened.preferences().revision, 1);
}
