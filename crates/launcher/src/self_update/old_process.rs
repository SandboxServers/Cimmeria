//! Find and end the launcher that started this one, for the relaunch
//! fallback in [`super::handoff::acquire_startup_lock`].
//!
//! The target is the pid in [`super::handoff::RELAUNCH_PID_ENV`], or, for a
//! relaunch by a launcher too old to set it, this process's parent (the
//! old launcher spawned us). Either way the process is ended only when its
//! image is this exe or `<exe>.old`: a pid can be reused, and the fallback
//! must never end anything but our own previous launcher.

use std::path::Path;

use super::handoff::{KillOutcome, RelaunchInfo};
use super::swap::old_path_for;

/// True when `image` (a process's full image path) is `exe` or
/// `<exe>.old`. Windows paths compare case-insensitively; a `\\?\` prefix
/// is ignored. The old launcher's image can read either way: its file was
/// renamed to `.old` while it ran.
pub fn is_our_exe(image: &Path, exe: &Path) -> bool {
    fn norm(p: &Path) -> String {
        let s = p.to_string_lossy();
        let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
        s.replace('/', "\\").to_lowercase()
    }
    let image = norm(image);
    image == norm(exe) || image == norm(&old_path_for(exe))
}

/// End the old launcher named by `info` (or this process's parent) if it
/// is running `exe` or `<exe>.old`, and wait briefly for it to go.
pub fn kill_old_launcher(info: &RelaunchInfo, exe: &Path) -> KillOutcome {
    let pid = match info.old_pid.or_else(parent_pid) {
        Some(pid) => pid,
        None => {
            return KillOutcome::NotKilled {
                reason: "no_old_pid",
            }
        }
    };
    if pid == std::process::id() {
        return KillOutcome::NotKilled { reason: "own_pid" };
    }
    platform::kill_if_ours(pid, exe)
}

#[cfg(windows)]
fn parent_pid() -> Option<u32> {
    platform::parent_pid()
}

#[cfg(not(windows))]
fn parent_pid() -> Option<u32> {
    None
}

#[cfg(windows)]
mod platform {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE,
    };

    use super::{is_our_exe, KillOutcome};

    /// Closes the handle on drop.
    struct Handle(HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful Win32 open and is
            // closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn parent_pid() -> Option<u32> {
        let me = std::process::id();
        // SAFETY: plain Win32 calls; the entry is sized as the API asks
        // and the snapshot handle is closed by `Handle`.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE || snap.is_null() {
                return None;
            }
            let snap = Handle(snap);
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snap.0, &mut entry);
            while ok != 0 {
                if entry.th32ProcessID == me {
                    return Some(entry.th32ParentProcessID);
                }
                ok = Process32NextW(snap.0, &mut entry);
            }
        }
        None
    }

    fn image_path(process: &Handle) -> Option<PathBuf> {
        let mut buf = vec![0u16; 32 * 1024];
        let mut len = buf.len() as u32;
        // SAFETY: `buf` holds `len` u16s and the API writes at most that.
        let ok = unsafe {
            QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
        };
        (ok != 0).then(|| PathBuf::from(OsString::from_wide(&buf[..len as usize])))
    }

    pub(super) fn kill_if_ours(pid: u32, exe: &Path) -> KillOutcome {
        // SAFETY: OpenProcess with a pid; a null result is handled.
        let raw = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
                0,
                pid,
            )
        };
        if raw.is_null() {
            // Gone already (ERROR_INVALID_PARAMETER) or not ours to open.
            return KillOutcome::NotKilled {
                reason: "open_failed",
            };
        }
        let process = Handle(raw);
        match image_path(&process) {
            Some(image) if is_our_exe(&image, exe) => {}
            Some(_) => {
                return KillOutcome::NotKilled {
                    reason: "not_our_exe",
                }
            }
            None => {
                return KillOutcome::NotKilled {
                    reason: "image_unknown",
                }
            }
        }
        // SAFETY: a valid handle opened with PROCESS_TERMINATE.
        if unsafe { TerminateProcess(process.0, 1) } == 0 {
            return KillOutcome::NotKilled {
                reason: "terminate_failed",
            };
        }
        // SAFETY: a valid handle opened with PROCESS_SYNCHRONIZE. The
        // lock wait that follows covers a timeout here.
        unsafe { WaitForSingleObject(process.0, 5_000) };
        KillOutcome::Killed { pid }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    use super::KillOutcome;

    pub(super) fn kill_if_ours(_pid: u32, _exe: &Path) -> KillOutcome {
        KillOutcome::NotKilled {
            reason: "unsupported_platform",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_exe_matches_the_exe_and_its_old_name_case_insensitively() {
        let exe = Path::new(r"C:\Games\SGW Launcher.exe");
        assert!(is_our_exe(Path::new(r"c:\games\sgw launcher.EXE"), exe));
        assert!(is_our_exe(
            Path::new(r"\\?\C:\Games\SGW Launcher.exe.old"),
            exe
        ));
        assert!(!is_our_exe(Path::new(r"C:\Games\SGW.exe"), exe));
        assert!(!is_our_exe(Path::new(r"C:\Other\SGW Launcher.exe"), exe));
    }

    /// Set on a copy of this test binary that plays the stuck old
    /// launcher: [`sleeper_child`] then sleeps instead of returning.
    #[cfg(windows)]
    const SLEEPER_ENV: &str = "SGW_LAUNCHER_TEST_SLEEPER";

    #[cfg(windows)]
    #[test]
    fn sleeper_child() {
        if std::env::var_os(SLEEPER_ENV).is_some() {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    /// Start this test binary running only [`sleeper_child`], asleep.
    #[cfg(windows)]
    fn spawn_sleeper() -> std::process::Child {
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "self_update::old_process::tests::sleeper_child",
                "--test-threads=1",
            ])
            .env(SLEEPER_ENV, "1")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    }

    // The fallback ends a process running our exe: here the test binary
    // stands in for the launcher that would not let go of the lock.
    #[cfg(windows)]
    #[test]
    fn the_fallback_ends_an_old_launcher_running_our_exe() {
        let mut child = spawn_sleeper();
        let exe = std::env::current_exe().unwrap();
        let info = RelaunchInfo {
            from_tag: "launcher-20260929-aaaaaaa".into(),
            old_pid: Some(child.id()),
        };
        let out = kill_old_launcher(&info, &exe);
        assert_eq!(out, KillOutcome::Killed { pid: child.id() });
        let status = child.wait().unwrap();
        assert!(!status.success(), "ended, not a normal exit: {status:?}");
    }

    // A reused pid that belongs to some other program is left alone.
    #[cfg(windows)]
    #[test]
    fn the_fallback_leaves_a_process_that_is_not_our_exe() {
        let mut child = spawn_sleeper();
        let info = RelaunchInfo {
            from_tag: "launcher-20260929-aaaaaaa".into(),
            old_pid: Some(child.id()),
        };
        let out = kill_old_launcher(&info, Path::new(r"C:\Games\sgw-launcher.exe"));
        assert_eq!(
            out,
            KillOutcome::NotKilled {
                reason: "not_our_exe"
            }
        );
        assert!(child.try_wait().unwrap().is_none(), "still running");
        child.kill().unwrap();
        let _ = child.wait();
    }

    // Launchers before the pid variable: the old launcher is our parent.
    #[cfg(windows)]
    #[test]
    fn the_parent_pid_is_found() {
        let parent = parent_pid().expect("a test process has a parent");
        assert_ne!(parent, 0);
        assert_ne!(parent, std::process::id());
    }

    #[test]
    fn a_relaunch_never_ends_itself() {
        let info = RelaunchInfo {
            from_tag: "launcher-20260929-aaaaaaa".into(),
            old_pid: Some(std::process::id()),
        };
        assert_eq!(
            kill_old_launcher(&info, Path::new("x.exe")),
            KillOutcome::NotKilled { reason: "own_pid" }
        );
    }
}
