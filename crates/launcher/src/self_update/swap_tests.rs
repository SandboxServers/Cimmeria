use std::time::Duration;

use fs4::FileExt;

use super::*;

const NAME: &str = "sgw-launcher-launcher-20260929-f518b57.exe";

fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join(NAME);
    let new = dir
        .path()
        .join(".sgw-launcher-update-launcher-20261002-bbbbbbb.exe.part");
    std::fs::write(&exe, b"old launcher").unwrap();
    std::fs::write(&new, b"new launcher").unwrap();
    (dir, exe, new)
}

#[test]
fn old_path_keeps_the_players_file_name() {
    assert_eq!(
        old_path_for(Path::new(r"C:\Games\my launcher.exe")),
        Path::new(r"C:\Games\my launcher.exe.old")
    );
}

#[test]
fn swap_puts_the_new_exe_in_place_and_keeps_the_old_one() {
    let (_dir, exe, new) = setup();
    swap_in(&exe, &new).unwrap();
    assert_eq!(std::fs::read(&exe).unwrap(), b"new launcher");
    assert_eq!(std::fs::read(old_path_for(&exe)).unwrap(), b"old launcher");
    assert!(!new.exists());
}

#[test]
fn swap_replaces_a_leftover_old_file() {
    let (_dir, exe, new) = setup();
    std::fs::write(old_path_for(&exe), b"two updates ago").unwrap();
    swap_in(&exe, &new).unwrap();
    assert_eq!(std::fs::read(old_path_for(&exe)).unwrap(), b"old launcher");
}

// Bug shape: the running exe was moved aside, the new one never landed,
// and the player was left with no launcher at its path.
#[test]
fn a_failed_install_rolls_the_old_exe_back() {
    let (_dir, exe, new) = setup();
    std::fs::remove_file(&new).unwrap();
    let err = swap_in(&exe, &new).unwrap_err();
    assert_eq!(err.reason(), "install_failed_rolled_back", "{err}");
    assert_eq!(std::fs::read(&exe).unwrap(), b"old launcher");
    assert!(!old_path_for(&exe).exists());
}

#[test]
fn a_failed_move_aside_changes_nothing() {
    let (dir, _exe, new) = setup();
    let missing = dir.path().join("not-there.exe");
    let err = swap_in(&missing, &new).unwrap_err();
    assert_eq!(err.reason(), "move_aside_failed");
    assert!(new.exists(), "the verified download is left for the caller");
}

#[test]
fn roll_back_restores_the_old_exe_after_a_failed_start() {
    let (_dir, exe, new) = setup();
    swap_in(&exe, &new).unwrap();
    roll_back(&exe).unwrap();
    assert_eq!(std::fs::read(&exe).unwrap(), b"old launcher");
    assert!(!old_path_for(&exe).exists());
}

#[test]
fn startup_cleanup_removes_the_old_exe() {
    let (_dir, exe, _new) = setup();
    assert_eq!(
        remove_old_exe(&exe, 3, Duration::ZERO),
        OldCleanup::NoneFound
    );
    std::fs::write(old_path_for(&exe), b"old").unwrap();
    assert_eq!(
        remove_old_exe(&exe, 3, Duration::ZERO),
        OldCleanup::Removed { attempts: 1 }
    );
    assert!(!old_path_for(&exe).exists());
    assert!(exe.exists(), "only the .old file goes");
}

// The old process may still hold its image open when the new one starts.
#[cfg(windows)]
#[test]
fn startup_cleanup_reports_a_locked_old_exe_and_retries_later() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_dir, exe, _new) = setup();
    let old = old_path_for(&exe);
    std::fs::write(&old, b"old").unwrap();
    // No sharing at all: delete fails with a sharing violation.
    let handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&old)
        .unwrap();
    assert_eq!(
        remove_old_exe(&exe, 2, Duration::from_millis(1)),
        OldCleanup::StillLocked
    );
    drop(handle);
    assert_eq!(
        remove_old_exe(&exe, 2, Duration::from_millis(1)),
        OldCleanup::Removed { attempts: 1 }
    );
}

// Bug shape: the relaunched launcher started while the old one still held
// launcher.lock and quit with "another instance is running".
#[test]
fn a_relaunch_waits_for_the_old_process_to_release_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("launcher.lock");
    let holder = std::fs::File::create(&lock_path).unwrap();
    FileExt::try_lock(&holder).unwrap();

    let contender = std::fs::File::create(&lock_path).unwrap();
    // Without waiting it fails at once, as a normal second instance must.
    assert!(!acquire_lock(
        || FileExt::try_lock(&contender).is_ok(),
        Duration::ZERO,
        Duration::from_millis(10)
    ));

    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        drop(holder);
    });
    assert!(acquire_lock(
        || FileExt::try_lock(&contender).is_ok(),
        Duration::from_secs(10),
        Duration::from_millis(10)
    ));
    release.join().unwrap();
}
