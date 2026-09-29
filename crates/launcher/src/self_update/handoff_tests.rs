use std::fs::File;
use std::path::PathBuf;

use fs4::FileExt;

use super::*;

/// A held `launcher.lock` plus a second handle on the same file, which
/// plays the new launcher: it can lock only once the holder lets go.
fn held_lock() -> (tempfile::TempDir, File, File) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("launcher.lock");
    let holder = File::create(&path).unwrap();
    FileExt::try_lock(&holder).unwrap();
    let contender = File::create(&path).unwrap();
    (dir, holder, contender)
}

fn can_lock(f: &File) -> bool {
    let ok = FileExt::try_lock(f).is_ok();
    if ok {
        FileExt::unlock(f).unwrap();
    }
    ok
}

/// Records the order of the handoff's process operations against a real
/// file lock.
struct FakeProcess {
    holder: Option<File>,
    contender: File,
    spawn_result: Option<std::io::Error>,
    calls: Vec<&'static str>,
    lock_free_at_spawn: Option<bool>,
    lock_free_at_exit: Option<bool>,
    spawned_with: Option<(PathBuf, String, u32)>,
}

impl FakeProcess {
    fn new(holder: File, contender: File) -> Self {
        Self {
            holder: Some(holder),
            contender,
            spawn_result: None,
            calls: Vec::new(),
            lock_free_at_spawn: None,
            lock_free_at_exit: None,
            spawned_with: None,
        }
    }
}

impl HandoffHooks for FakeProcess {
    fn spawn(&mut self, exe: &Path, from_tag: &str, old_pid: u32) -> std::io::Result<u32> {
        self.calls.push("spawn");
        self.lock_free_at_spawn = Some(can_lock(&self.contender));
        self.spawned_with = Some((exe.to_path_buf(), from_tag.to_string(), old_pid));
        match self.spawn_result.take() {
            Some(e) => Err(e),
            None => Ok(4242),
        }
    }

    fn release_lock(&mut self) -> bool {
        self.calls.push("release_lock");
        match self.holder.take() {
            Some(f) => {
                FileExt::unlock(&f).unwrap();
                true
            }
            None => false,
        }
    }

    fn exit(&mut self) {
        self.calls.push("exit");
        self.lock_free_at_exit = Some(can_lock(&self.contender));
    }
}

// Bug shape (2026-09-29, 676f314 -> 4fcae33): after starting the new
// launcher the old one closed through the UI, which waited for an egui
// frame that never came, so it kept launcher.lock and the new launcher
// gave up. The handoff itself must release the lock and exit.
#[test]
fn the_handoff_releases_the_lock_and_exits_without_the_ui() {
    let (_dir, holder, contender) = held_lock();
    let mut p = FakeProcess::new(holder, contender);
    let exe = Path::new("sgw-launcher.exe");
    hand_off(
        exe,
        "launcher-20260929-aaaaaaa",
        "launcher-20260930-bbbbbbb",
        &mut p,
    )
    .unwrap();
    assert_eq!(p.calls, ["spawn", "release_lock", "exit"]);
    assert_eq!(
        p.lock_free_at_spawn,
        Some(false),
        "the lock is kept until the new launcher has started"
    );
    assert_eq!(
        p.lock_free_at_exit,
        Some(true),
        "the new launcher can take the lock before this process exits"
    );
    let (spawned_exe, from, pid) = p.spawned_with.unwrap();
    assert_eq!(spawned_exe, exe);
    assert_eq!(from, "launcher-20260929-aaaaaaa");
    assert_eq!(pid, std::process::id(), "the new launcher learns our pid");
}

#[test]
fn a_failed_start_rolls_back_and_keeps_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("sgw-launcher.exe");
    let new = dir.path().join("new.part");
    std::fs::write(&exe, b"old launcher").unwrap();
    std::fs::write(&new, b"new launcher").unwrap();
    swap::swap_in(&exe, &new).unwrap();

    let (_lock_dir, holder, contender) = held_lock();
    let mut p = FakeProcess::new(holder, contender);
    p.spawn_result = Some(std::io::Error::other("not a valid Win32 application"));
    let err = hand_off(&exe, "a", "b", &mut p).unwrap_err();
    assert_eq!(err.reason(), "relaunch_failed");
    assert_eq!(p.calls, ["spawn"], "no release, no exit");
    assert!(!can_lock(&p.contender), "this launcher keeps its lock");
    assert_eq!(std::fs::read(&exe).unwrap(), b"old launcher");
}

fn relaunch_info(pid: Option<u32>) -> RelaunchInfo {
    RelaunchInfo {
        from_tag: "launcher-20260929-676f314".into(),
        old_pid: pid,
    }
}

fn fast_wait() -> LockWait {
    LockWait {
        first: Duration::from_millis(60),
        after_kill: Duration::from_secs(5),
        poll: Duration::from_millis(5),
    }
}

// Bug shape: the old launcher (676f314) held the lock past the relaunch
// wait, and the new one showed "another instance is running". Past the
// first wait it now ends the old launcher and carries on.
#[test]
fn a_relaunch_survives_an_old_launcher_that_outlives_the_wait() {
    let (_dir, holder, contender) = held_lock();
    let mut holder = Some(holder);
    let mut killed = Vec::new();
    let out = acquire_startup_lock(
        Some(&relaunch_info(Some(77))),
        || FileExt::try_lock(&contender).is_ok(),
        |info| {
            killed.push(info.old_pid);
            // Ending the process drops its lock.
            drop(holder.take());
            KillOutcome::Killed { pid: 77 }
        },
        &fast_wait(),
    );
    assert!(
        matches!(
            out,
            StartupLock::Acquired {
                killed_pid: Some(77),
                ..
            }
        ),
        "{out:?}"
    );
    assert_eq!(killed, [Some(77)]);
}

// The lock frees late on its own (a slow old launcher) but inside the
// first wait: no kill.
#[test]
fn a_relaunch_that_gets_the_lock_in_time_ends_nothing() {
    let (_dir, holder, contender) = held_lock();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        drop(holder);
    });
    let wait = LockWait {
        first: Duration::from_secs(10),
        ..fast_wait()
    };
    let out = acquire_startup_lock(
        Some(&relaunch_info(Some(77))),
        || FileExt::try_lock(&contender).is_ok(),
        |_| panic!("the old launcher exited by itself"),
        &wait,
    );
    release.join().unwrap();
    assert!(
        matches!(out, StartupLock::Acquired { killed_pid: None, waited } if waited >= Duration::from_millis(50)),
        "{out:?}"
    );
}

#[test]
fn a_relaunch_that_cannot_end_the_old_launcher_says_so() {
    let (_dir, _holder, contender) = held_lock();
    let out = acquire_startup_lock(
        Some(&relaunch_info(None)),
        || FileExt::try_lock(&contender).is_ok(),
        |_| KillOutcome::NotKilled {
            reason: "not_our_exe",
        },
        &fast_wait(),
    );
    assert_eq!(
        out,
        StartupLock::UpdateHandoffStuck {
            reason: "not_our_exe"
        }
    );
}

// The double-launch protection is unchanged: no wait, no kill.
#[test]
fn a_normal_start_still_refuses_a_second_instance_at_once() {
    let (_dir, _holder, contender) = held_lock();
    let wait = LockWait {
        first: Duration::from_secs(10),
        ..fast_wait()
    };
    let start = Instant::now();
    let out = acquire_startup_lock(
        None,
        || FileExt::try_lock(&contender).is_ok(),
        |_| panic!("a normal start never ends another launcher"),
        &wait,
    );
    assert_eq!(out, StartupLock::HeldByAnotherInstance);
    assert!(start.elapsed() < Duration::from_secs(2), "no relaunch wait");
}

#[test]
fn relaunch_info_reads_the_tag_and_an_optional_pid() {
    let v = |s: &str| Some(OsString::from(s));
    assert_eq!(
        RelaunchInfo::from_values(v("launcher-a"), v("1234")),
        Some(RelaunchInfo {
            from_tag: "launcher-a".into(),
            old_pid: Some(1234)
        })
    );
    // Started by a launcher that predates the pid variable.
    assert_eq!(
        RelaunchInfo::from_values(v("launcher-a"), None)
            .unwrap()
            .old_pid,
        None
    );
    assert_eq!(
        RelaunchInfo::from_values(v("launcher-a"), v("not a pid"))
            .unwrap()
            .old_pid,
        None
    );
    assert_eq!(RelaunchInfo::from_values(None, v("1234")), None);
}

#[test]
fn the_relaunch_command_carries_the_tag_and_the_old_pid() {
    let cmd = swap::relaunch_command(Path::new("sgw-launcher.exe"), "launcher-a", 4321);
    let envs: Vec<_> = cmd.get_envs().collect();
    assert!(envs.contains(&(
        std::ffi::OsStr::new(RELAUNCH_ENV),
        Some(std::ffi::OsStr::new("launcher-a"))
    )));
    assert!(envs.contains(&(
        std::ffi::OsStr::new(RELAUNCH_PID_ENV),
        Some(std::ffi::OsStr::new("4321"))
    )));
}
