//! Stops one game client by pid. Windows only: the other platforms get a stub
//! that says so (the lab runs the Windows client; Linux CI only compiles this).
//!
//! The steps are `Stop-LabClient` of the PowerShell command: act only on a
//! process whose image is `SGW.exe`; ask its windows to close; wait 8 s;
//! force-stop it; wait 2 s more. The process handle is opened first and held,
//! so a reused pid cannot be swapped in between the image check and the stop.

use std::time::Duration;

/// How long the windows get to close before the process is force-stopped.
pub const GRACE: Duration = Duration::from_millis(8000);

/// How long a force-stopped process gets to be gone before the stop reports `Failed`.
pub const KILL_WAIT: Duration = Duration::from_millis(2000);

/// What a stop did.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome {
    /// No process with that pid, or it is not `SGW.exe`: nothing was touched.
    NotSgw,
    /// The process was `SGW.exe` and is gone.
    Stopped,
    /// The process was `SGW.exe` and is still alive (for example an elevated
    /// client this shell may not stop).
    Failed,
}

/// Stops the client with this pid, giving its windows [`GRACE`] to close.
pub fn stop_client(pid: u32) -> Result<StopOutcome, String> {
    stop_client_with(pid, GRACE, KILL_WAIT)
}

/// The same with a chosen grace period and a chosen wait after the force-stop
/// (tests use a short grace and a long wait).
#[cfg(not(windows))]
pub fn stop_client_with(
    _pid: u32,
    _grace: Duration,
    _kill_wait: Duration,
) -> Result<StopOutcome, String> {
    Err("Windows only".to_string())
}

/// The same with a chosen grace period and a chosen wait after the force-stop
/// (tests use a short grace and a long wait).
#[cfg(windows)]
pub fn stop_client_with(
    pid: u32,
    grace: Duration,
    kill_wait: Duration,
) -> Result<StopOutcome, String> {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, FALSE, HANDLE, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    use crate::processes::is_sgw_image;

    /// Waits for the process to exit; true when it has.
    fn exited(handle: HANDLE, wait: Duration) -> bool {
        let ms = u32::try_from(wait.as_millis()).unwrap_or(u32::MAX);
        // SAFETY: `handle` is a live process handle with SYNCHRONIZE access.
        unsafe { WaitForSingleObject(handle, ms) == WAIT_OBJECT_0 }
    }

    // SAFETY: plain Win32 calls; the handle is closed on every path below.
    let handle = unsafe {
        let full = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            FALSE,
            pid,
        );
        if full.is_null() {
            // Without the right to terminate (an elevated client): still look,
            // so the outcome can be `Failed` rather than a lie.
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                FALSE,
                pid,
            )
        } else {
            full
        }
    };
    if handle.is_null() {
        // SAFETY: reads this thread's last error.
        let code = unsafe { GetLastError() };
        // No such process: the PowerShell "no client" case.
        return if code == ERROR_INVALID_PARAMETER {
            Ok(StopOutcome::NotSgw)
        } else {
            Err(format!("cannot open pid {pid} (Windows error {code})"))
        };
    }

    let mut buffer = [0u16; 1024];
    let mut length = buffer.len() as u32;
    // SAFETY: `buffer` holds `length` u16s; flags 0 asks for the Win32 path.
    let queried =
        unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) } != 0;
    if !queried {
        let gone = exited(handle, Duration::ZERO);
        // SAFETY: closes the handle opened above, once.
        unsafe { CloseHandle(handle) };
        return if gone {
            Ok(StopOutcome::NotSgw)
        } else {
            Err(format!("cannot read the image name of pid {pid}"))
        };
    }
    let image = String::from_utf16_lossy(&buffer[..length as usize]);
    if !is_sgw_image(&image) {
        // SAFETY: closes the handle opened above, once.
        unsafe { CloseHandle(handle) };
        return Ok(StopOutcome::NotSgw);
    }

    post_close_to_windows_of(pid);
    let mut done = exited(handle, grace);
    if !done {
        // SAFETY: `handle` was opened with PROCESS_TERMINATE when the OS allowed it;
        // a refusal just leaves the process alive and is caught by the wait below.
        unsafe { TerminateProcess(handle, 1) };
        done = exited(handle, kill_wait);
    }
    // SAFETY: closes the handle opened above, once.
    unsafe { CloseHandle(handle) };
    Ok(if done {
        StopOutcome::Stopped
    } else {
        StopOutcome::Failed
    })
}

/// Posts `WM_CLOSE` to every top-level window that belongs to `pid`.
#[cfg(windows)]
fn post_close_to_windows_of(pid: u32) {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, TRUE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
    };

    struct Target {
        pid: u32,
    }

    unsafe extern "system" fn close_if_owned(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `&Target` passed to EnumWindows below, alive
        // for the whole enumeration.
        let target = unsafe { &*(lparam as *const Target) };
        let mut owner: u32 = 0;
        // SAFETY: `hwnd` comes from EnumWindows; `owner` is a valid out pointer.
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        if owner == target.pid {
            // SAFETY: posting to a window handle is safe even if it just died.
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        }
        TRUE
    }

    let target = Target { pid };
    // SAFETY: the callback only reads `target`, which outlives the call.
    unsafe { EnumWindows(Some(close_if_owned), &target as *const Target as LPARAM) };
}

#[cfg(all(test, windows))]
mod tests {
    use std::process::Command;

    use super::*;

    // ports: 'a pid that is not SGW is not stopped' (this process is left alone)
    #[test]
    fn a_process_that_is_not_sgw_is_not_touched() {
        let outcome = stop_client_with(std::process::id(), Duration::from_millis(200), KILL_WAIT);
        assert_eq!(outcome, Ok(StopOutcome::NotSgw));
    }

    // new: a process whose image is SGW.exe really is stopped
    #[test]
    fn a_process_named_sgw_exe_is_stopped() {
        let dir = tempfile::tempdir().expect("temp dir");
        let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let fake = dir.path().join("SGW.exe");
        std::fs::copy(format!(r"{system}\System32\ping.exe"), &fake).expect("copy ping.exe");
        let mut child = Command::new(&fake)
            .args(["-n", "60", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("start the stand-in SGW.exe");
        // The stand-in has no window, so the grace runs out and the force-stop
        // does the work; 30 s is a ceiling, not a delay (the wait returns when
        // the process is gone).
        let outcome = stop_client_with(
            child.id(),
            Duration::from_millis(200),
            Duration::from_secs(30),
        );
        let still_running = child.try_wait().expect("try_wait").is_none();
        if still_running {
            let _ = child.kill();
        }
        assert_eq!(outcome, Ok(StopOutcome::Stopped));
        assert!(!still_running, "the stand-in SGW.exe should be gone");
    }

    // F-LX9: stopping several stand-ins in a row is stable (the old fixed 2 s
    // wait after the force-stop was the flake under load).
    #[test]
    fn five_stand_ins_in_a_row_are_all_stopped() {
        let dir = tempfile::tempdir().expect("temp dir");
        let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let fake = dir.path().join("SGW.exe");
        std::fs::copy(format!(r"{system}\System32\ping.exe"), &fake).expect("copy ping.exe");
        for round in 0..5 {
            let mut child = Command::new(&fake)
                .args(["-n", "60", "127.0.0.1"])
                .stdout(std::process::Stdio::null())
                .spawn()
                .expect("start the stand-in SGW.exe");
            let outcome = stop_client_with(
                child.id(),
                Duration::from_millis(100),
                Duration::from_secs(30),
            );
            let gone = child.try_wait().expect("try_wait").is_some();
            if !gone {
                let _ = child.kill();
            }
            assert_eq!(outcome, Ok(StopOutcome::Stopped), "round {round}");
            assert!(gone, "round {round}");
        }
    }
}
