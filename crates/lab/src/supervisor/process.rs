//! Native process lifecycle for the supervised client: suspended
//! launch + inject (via the shared `cimmeria-client-launch` crate),
//! liveness/exit checks, terminate, and top-level-window resolution by
//! PID (for `lab_screenshot`).
//!
//! **The launch goes through the i686 `sgw-start32` helper** (#985). The
//! supervisor is 64-bit and `SGW.exe` is 32-bit, and an injection across
//! bitness cannot work: the remote thread would be handed the supervisor's
//! own `LoadLibraryW`, and a suspended WOW64 target has no 32-bit kernel32
//! mapped yet anyway. `inject_dll` refuses it with `BitnessMismatch`. The
//! helper does the suspended launch and the injection at the target's
//! bitness, exactly as `sgw-launcher` does since #984; the contract is in
//! `crates/client-launch/README.md`.
//!
//! Everything is keyed on the **PID** rather than a retained kernel
//! handle: a PID is `Copy`/`Send`, so the supervisor state and the
//! background watchdog can be `Send`/`Sync` without wrapping a raw
//! `HANDLE`. Each operation opens a short-lived handle and closes it.
//!
//! The Win32 calls are integration-only and need live validation. The
//! window-selection *policy* ([`pick_window`]) is pure and unit-tested;
//! the `EnumWindows` walk only gathers candidates and hands them to it.

use std::path::{Path, PathBuf};

use cimmeria_client_launch::start32::{Request, Target, HELPER_EXE_NAME};

/// Where the `sgw-start32` helper is: `override_path` (the
/// `CIMMERIA_LAB_START32` environment variable) when set, else
/// `sgw-start32.exe` beside the supervisor's own executable, the one
/// stable place the client-launch contract asks a separately shipped caller
/// to keep it. `None` only when neither is known.
pub fn resolve_helper(override_path: Option<PathBuf>, exe_dir: Option<&Path>) -> Option<PathBuf> {
    override_path.or_else(|| exe_dir.map(|d| d.join(HELPER_EXE_NAME)))
}

/// The helper request for one supervised launch: start `exe` suspended in
/// `install_dir`, inject the bridge DLL (the only DLL), resume. Pure so the
/// command line the helper receives is testable without Windows.
pub fn launch_request(
    install_dir: &Path,
    exe: &Path,
    dll_path: &Path,
    patches_dll: Option<&Path>,
) -> Request {
    // Same order as the launcher: client-patches first, then telemetry, so
    // a lab client runs the patched code paths a player's client runs.
    let mut dlls: Vec<PathBuf> = patches_dll.map(Path::to_path_buf).into_iter().collect();
    dlls.push(dll_path.to_path_buf());
    Request {
        target: Target::Spawn {
            exe: exe.to_path_buf(),
            cwd: Some(install_dir.to_path_buf()),
            args: Vec::new(),
        },
        dlls,
    }
}

/// Whether a process image path names the game client (`SGW.exe`, any
/// directory, any case). Pure so the match is testable off Windows.
pub fn is_sgw_image(path: &str) -> bool {
    path.rsplit(['\\', '/'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("SGW.exe"))
}

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
    use super::{launch_request, pick_window, WindowCandidate, HELPER_EXE_NAME};
    use std::path::Path;

    use cimmeria_client_launch::inject::RunningProcess;
    use cimmeria_client_launch::launch::checked_sgw_exe;
    use cimmeria_client_launch::start32;

    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
        PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW,
    };

    const STILL_ACTIVE: u32 = 259;

    /// Launch SGW.exe with the (lab-bridge) telemetry DLL injected, through
    /// the i686 `helper`, and return its PID. The session file must already
    /// be written (the DLL reads it during `DllMain`).
    ///
    /// A failed injection comes back as the helper's `error kind=...` line;
    /// the helper has already terminated the suspended process by then.
    pub fn launch(
        install_dir: &Path,
        dll_path: &Path,
        helper: &Path,
        patches_dll: Option<&Path>,
        envs: &[(String, String)],
    ) -> Result<u32, String> {
        let (dir, exe) = checked_sgw_exe(install_dir).map_err(|e| format!("launch: {e}"))?;
        if !dll_path.is_file() {
            return Err(format!(
                "launch: bridge DLL not found at {}",
                dll_path.display()
            ));
        }
        if let Some(p) = patches_dll.filter(|p| !p.is_file()) {
            return Err(format!(
                "launch: client-patches DLL not found at {}",
                p.display()
            ));
        }
        if !helper.is_file() {
            return Err(format!(
                "launch: {HELPER_EXE_NAME} not found at {}; build it with `cargo build -p \
                 cimmeria-start32 --target i686-pc-windows-msvc` and put it beside \
                 cimmeria-lab.exe, or set CIMMERIA_LAB_START32",
                helper.display()
            ));
        }
        let pid = start32::run_with_env(
            helper,
            &launch_request(&dir, &exe, dll_path, patches_dll),
            envs,
        )
        .map_err(|e| format!("launch+inject via {HELPER_EXE_NAME}: {e}"))?;
        // The helper exits as soon as the game is resumed. Opening the pid
        // confirms it names a process this supervisor can follow before the
        // pid is recorded; liveness after that is `is_alive`.
        RunningProcess::open(pid)
            .map_err(|e| format!("started SGW.exe (pid {pid}) but could not open it: {e}"))?;
        Ok(pid)
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

    /// Post a window message to the game (no focus needed; delivered on
    /// the game's own message loop).
    pub fn post_message(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> Result<(), String> {
        // SAFETY: PostMessageW only queues; a stale hwnd fails cleanly.
        let ok = unsafe { PostMessageW(hwnd as HWND, msg, wparam, lparam) };
        if ok == 0 {
            Err(format!("PostMessageW(0x{msg:04x}) failed"))
        } else {
            Ok(())
        }
    }

    /// PIDs of every running `SGW.exe` that owns a top-level window,
    /// whoever started it. Two clients on one machine misbehave, so the
    /// supervisor refuses to launch while this is non-empty.
    pub fn running_sgw_pids() -> Vec<u32> {
        running_pids_where(super::is_sgw_image)
    }

    /// Pids of processes with a top-level window whose image path passes
    /// `keep` (e.g. a running screensaver, `*.scr`).
    pub fn running_pids_where(keep: impl Fn(&str) -> bool) -> Vec<u32> {
        let mut ctx = EnumCtx {
            candidates: Vec::new(),
        };
        // SAFETY: enum_cb is a valid callback; ctx outlives the call.
        unsafe {
            EnumWindows(Some(enum_cb), &mut ctx as *mut EnumCtx as LPARAM);
        }
        let mut pids: Vec<u32> = ctx.candidates.iter().map(|c| c.pid).collect();
        pids.sort_unstable();
        pids.dedup();
        pids.retain(|&pid| image_path(pid).is_some_and(|p| keep(&p)));
        pids
    }

    fn image_path(pid: u32) -> Option<String> {
        // SAFETY: OpenProcess/QueryFullProcessImageNameW/CloseHandle with a
        // correctly sized buffer; the handle is closed on every path.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(h);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
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
pub use win::{
    find_main_window, is_alive, launch, post_message, running_pids_where, running_sgw_pids,
    terminate,
};

// Non-Windows stubs so the crate compiles on Linux dev hosts / coverage
// and the portable supervisor tests still run. The lab only runs on the
// owner's Windows box.
#[cfg(not(windows))]
pub fn launch(
    _install_dir: &Path,
    _dll_path: &Path,
    _helper: &Path,
    _patches_dll: Option<&Path>,
    _envs: &[(String, String)],
) -> Result<u32, String> {
    Err("process launch is Windows-only".to_string())
}
#[cfg(not(windows))]
pub fn running_sgw_pids() -> Vec<u32> {
    Vec::new()
}
#[cfg(not(windows))]
pub fn running_pids_where(_keep: impl Fn(&str) -> bool) -> Vec<u32> {
    Vec::new()
}
#[cfg(not(windows))]
pub fn post_message(_hwnd: isize, _msg: u32, _wparam: usize, _lparam: isize) -> Result<(), String> {
    Err("window messages are Windows-only".to_string())
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

    /// #985: the supervised launch asks the helper to spawn SGW.exe in the
    /// install dir with exactly the bridge DLL. The command line pins the
    /// contract (`spawn <exe> --cwd <dir> --dll <path>`) the i686 helper
    /// parses, so a drift in how the lab builds the request shows here.
    #[test]
    fn the_launch_request_spawns_sgw_in_the_install_dir_with_the_bridge_dll() {
        let install = Path::new("C:/SGW");
        let exe = Path::new("C:/SGW/SGW.exe");
        let dll = Path::new("C:/lab/cimmeria-client-telemetry.dll");
        let req = launch_request(install, exe, dll, None);
        assert_eq!(req.dlls, vec![dll.to_path_buf()]);
        let args: Vec<String> = req
            .to_args()
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "spawn",
                "C:/SGW/SGW.exe",
                "--cwd",
                "C:/SGW",
                "--dll",
                "C:/lab/cimmeria-client-telemetry.dll"
            ]
        );
    }

    /// With a client-patches DLL configured, it goes in first, as the
    /// launcher injects it, so the lab client runs the patched paths.
    #[test]
    fn the_client_patches_dll_is_injected_before_the_bridge_dll() {
        let exe = Path::new("C:/SGW/SGW.exe");
        let dll = Path::new("C:/lab/cimmeria-client-telemetry.dll");
        let patches = Path::new("C:/lab/cimmeria-client-patches.dll");
        let req = launch_request(Path::new("C:/SGW"), exe, dll, Some(patches));
        assert_eq!(req.dlls, vec![patches.to_path_buf(), dll.to_path_buf()]);
    }

    #[test]
    fn the_helper_is_the_override_else_beside_the_supervisor() {
        let dir = Path::new("C:/tools");
        assert_eq!(
            resolve_helper(None, Some(dir)),
            Some(dir.join("sgw-start32.exe"))
        );
        let custom = PathBuf::from("D:/build/sgw-start32.exe");
        assert_eq!(
            resolve_helper(Some(custom.clone()), Some(dir)),
            Some(custom)
        );
        assert_eq!(resolve_helper(None, None), None);
    }

    /// A missing helper is refused with a message that says how to get one,
    /// before anything is started (Windows only: elsewhere launch is a stub).
    #[cfg(windows)]
    #[test]
    fn a_missing_helper_is_refused_before_anything_starts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"MZ").unwrap();
        let dll = dir.path().join("bridge.dll");
        std::fs::write(&dll, b"MZ").unwrap();
        let err = launch(
            dir.path(),
            &dll,
            &dir.path().join("sgw-start32.exe"),
            None,
            &[],
        )
        .unwrap_err();
        assert!(err.contains("sgw-start32.exe not found"), "{err}");
        assert!(err.contains("CIMMERIA_LAB_START32"), "{err}");
    }

    /// End to end, the #985 bug shape: from this 64-bit build, `launch`
    /// starts a real 32-bit program (a copy of `SysWOW64\winver.exe` named
    /// SGW.exe) with a real 32-bit DLL (`SysWOW64\version.dll`) injected.
    /// `Ok(pid)` is the proof: the helper answers `ok` only after the
    /// remote `LoadLibraryW` returned a module and the process was resumed,
    /// and `launch` then opened the pid. The old same-bitness injector
    /// refuses this pair (`BitnessMismatch`).
    ///
    /// Needs the built i686 helper in `CIMMERIA_TEST_START32` (see
    /// `crates/client-launch/README.md`); skips without it. The lab is not
    /// in CI, so this is a manual check.
    #[cfg(windows)]
    #[test]
    fn launch_injects_into_a_real_32_bit_process_through_the_helper() {
        let Some(helper) = std::env::var_os("CIMMERIA_TEST_START32").map(PathBuf::from) else {
            eprintln!("CIMMERIA_TEST_START32 unset; skipping");
            return;
        };
        let Some(wow) = std::env::var_os("SystemRoot").map(|r| PathBuf::from(r).join("SysWOW64"))
        else {
            return;
        };
        let (exe, dll) = (wow.join("winver.exe"), wow.join("version.dll"));
        if !exe.is_file() || !dll.is_file() {
            eprintln!("no SysWOW64 winver.exe/version.dll; skipping");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(&exe, dir.path().join("SGW.exe")).unwrap();
        // Without its language resource the copy exits at once, before
        // `launch` can open the pid. With it, it shows the About dialog
        // until terminated below.
        let mui = wow.join("en-US").join("winver.exe.mui");
        if !mui.is_file() {
            eprintln!("no SysWOW64 en-US winver.exe.mui; skipping");
            return;
        }
        std::fs::create_dir(dir.path().join("en-US")).unwrap();
        std::fs::copy(&mui, dir.path().join("en-US").join("SGW.exe.mui")).unwrap();

        let pid = launch(dir.path(), &dll, &helper, None, &[]).expect("launch through the helper");
        let alive = is_alive(pid);
        terminate(pid);
        assert!(alive, "the injected program is still running after launch");
    }

    #[test]
    fn is_sgw_image_matches_the_client_exe_only() {
        assert!(super::is_sgw_image(r"C:\SGW\Working\Binaries\SGW.exe"));
        assert!(super::is_sgw_image("C:/x/sgw.EXE"));
        assert!(!super::is_sgw_image(r"C:\SGW\sgw-launcher.exe"));
        assert!(!super::is_sgw_image(r"C:\SGW.exe\other.exe"));
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
