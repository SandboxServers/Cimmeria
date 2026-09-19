//! Native process lifecycle for the supervised client: suspended
//! launch + inject (via the shared `cimmeria-client-launch` crate),
//! liveness/exit checks, terminate, and top-level-window resolution by
//! PID (for `lab_screenshot`).
//!
//! Everything is keyed on the **PID** rather than a retained kernel
//! handle: a PID is `Copy`/`Send`, so the supervisor state and the
//! background watchdog can be `Send`/`Sync` without wrapping a raw
//! `HANDLE`. Each operation opens a short-lived handle and closes it.
//!
//! The Win32 calls are integration-only and need live validation. The
//! window-selection *policy* ([`pick_window`]) is pure and unit-tested;
//! the `EnumWindows` walk only gathers candidates and hands them to it.

/// A top-level window candidate gathered during the `EnumWindows` walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowCandidate {
    pub hwnd: isize,
    pub pid: u32,
    pub visible: bool,
    /// Title length in chars; the main game window has a title, tool /
    /// message windows usually don't.
    pub title_len: i32,
}

/// Choose the best window for a PID: visible, matching PID, preferring
/// the one with the longest title (the main window). Returns its HWND.
/// Pure so the selection heuristic is testable without a desktop.
pub fn pick_window(candidates: &[WindowCandidate], target_pid: u32) -> Option<isize> {
    candidates
        .iter()
        .filter(|c| c.pid == target_pid && c.visible)
        .max_by_key(|c| c.title_len)
        .map(|c| c.hwnd)
}

#[cfg(windows)]
mod win {
    use super::{pick_window, WindowCandidate};
    use std::path::Path;

    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, TerminateProcess, PROCESS_QUERY_INFORMATION,
        PROCESS_TERMINATE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible,
    };

    const STILL_ACTIVE: u32 = 259;

    /// Launch SGW.exe with the (lab-bridge) telemetry DLL injected and
    /// return its PID. The session file must already be written (the DLL
    /// reads it during `DllMain`).
    pub fn launch(install_dir: &Path, dll_path: &Path) -> Result<u32, String> {
        cimmeria_client_launch::launch::launch_sgw_with_telemetry(install_dir, dll_path)
            .map_err(|e| format!("launch+inject: {e}"))
    }

    /// Whether a PID is still running.
    pub fn is_alive(pid: u32) -> bool {
        // SAFETY: standard OpenProcess call.
        let h = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid) };
        if h.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        // SAFETY: valid handle + out-param; handle closed below.
        let ok = unsafe { GetExitCodeProcess(h, &mut code) };
        unsafe { CloseHandle(h) };
        ok != 0 && code == STILL_ACTIVE
    }

    /// Terminate a PID (watchdog fired, or explicit stop). Best-effort.
    pub fn terminate(pid: u32) {
        // SAFETY: standard OpenProcess/TerminateProcess/CloseHandle.
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
            if !h.is_null() {
                TerminateProcess(h, 0xDEAD);
                CloseHandle(h);
            }
        }
    }

    struct EnumCtx {
        candidates: Vec<WindowCandidate>,
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
        let ctx = &mut *(lparam as *mut EnumCtx);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        ctx.candidates.push(WindowCandidate {
            hwnd: hwnd as isize,
            pid,
            visible: IsWindowVisible(hwnd) != 0,
            title_len: GetWindowTextLengthW(hwnd),
        });
        1 // continue enumeration (TRUE)
    }

    /// Resolve the main top-level window for a PID.
    pub fn find_main_window(pid: u32) -> Option<isize> {
        let mut ctx = EnumCtx {
            candidates: Vec::new(),
        };
        // SAFETY: enum_cb is a valid callback; ctx outlives the call.
        unsafe {
            EnumWindows(Some(enum_cb), &mut ctx as *mut EnumCtx as LPARAM);
        }
        pick_window(&ctx.candidates, pid)
    }
}

#[cfg(windows)]
pub use win::{find_main_window, is_alive, launch, terminate};

// Non-Windows stubs so the crate compiles on Linux dev hosts / coverage
// and the portable supervisor tests still run. The lab only runs on the
// owner's Windows box.
#[cfg(not(windows))]
pub fn launch(_install_dir: &std::path::Path, _dll_path: &std::path::Path) -> Result<u32, String> {
    Err("process launch is Windows-only".to_string())
}
#[cfg(not(windows))]
pub fn is_alive(_pid: u32) -> bool {
    false
}
#[cfg(not(windows))]
pub fn terminate(_pid: u32) {}
#[cfg(not(windows))]
pub fn find_main_window(_pid: u32) -> Option<isize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hwnd: isize, pid: u32, visible: bool, title_len: i32) -> WindowCandidate {
        WindowCandidate {
            hwnd,
            pid,
            visible,
            title_len,
        }
    }

    #[test]
    fn pick_window_prefers_visible_titled_match() {
        let cands = [
            c(1, 100, true, 0),   // matching pid, visible, no title
            c(2, 100, true, 12),  // matching pid, visible, titled ← main
            c(3, 100, false, 30), // matching pid but hidden
            c(4, 200, true, 40),  // other process
        ];
        assert_eq!(pick_window(&cands, 100), Some(2));
    }

    #[test]
    fn pick_window_none_when_no_visible_match() {
        let cands = [c(1, 100, false, 5), c(2, 200, true, 5)];
        assert_eq!(pick_window(&cands, 100), None);
        assert_eq!(pick_window(&[], 100), None);
    }

    #[test]
    fn pick_window_falls_back_to_titleless_visible() {
        let cands = [c(9, 100, true, 0)];
        assert_eq!(pick_window(&cands, 100), Some(9));
    }
}
