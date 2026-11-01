//! Keep the display usable for the lab client.
//!
//! Lab input goes through the client's hooked DirectInput, which never resets
//! Windows' idle timer, so the screensaver starts about ten minutes into any
//! unattended run. While a screensaver owns the input desktop, Direct3D 9's
//! `GetDeviceCaps` fails with `D3DERR_NOTAVAILABLE`: every client launched
//! then dies on a "GetDeviceCaps failed" message box followed by an R6025
//! box (each one a Windows error sound), and the watchdog relaunches it until
//! its cap (2026-09-29, twice).
//!
//! - [`spawn_keep_awake`]: a thread holds `ES_DISPLAY_REQUIRED` while any
//!   `SGW.exe` is running and releases it when none is.
//! - [`ensure_display_for_launch`]: before a launch, dismiss a running
//!   screensaver that is not password-protected, then probe Direct3D; refuse
//!   the launch with the reason instead of starting a client that can only
//!   show an error box.

/// `D3DERR_NOTAVAILABLE`.
pub const D3DERR_NOTAVAILABLE: u32 = 0x8876_086a;

/// Why a launch cannot get a display, as `lab_client_start` reports it.
pub fn describe_d3d_failure(hr: u32, screensaver_running: bool, secure: bool) -> String {
    let what = if hr == D3DERR_NOTAVAILABLE {
        "Direct3D reports no available adapter (D3DERR_NOTAVAILABLE)".to_string()
    } else {
        format!("Direct3D GetDeviceCaps failed with {hr:#010x}")
    };
    if screensaver_running && secure {
        format!("{what}: a password-protected screensaver is running; unlock the workstation")
    } else if screensaver_running {
        format!("{what}: the screensaver is still running")
    } else {
        format!("{what}: the display is off, locked or not attached")
    }
}

/// Pids of `SGW.exe` among `(pid, exe file name)` process entries. The
/// keep-awake counts every such process, window or not: a client that hangs
/// before its window shows (a starved drive) is exactly when the screensaver
/// must not start.
pub fn sgw_pids_in(entries: impl IntoIterator<Item = (u32, String)>) -> Vec<u32> {
    entries
        .into_iter()
        .filter(|(_, exe)| crate::supervisor::process::is_sgw_image(exe))
        .map(|(pid, _)| pid)
        .collect()
}

/// What a launch does about running screensavers (C18).
#[derive(Debug, PartialEq, Eq)]
pub enum SaverAction {
    /// No screensaver process runs; launch continues.
    None,
    /// Non-secure screensavers (pids) to terminate before the probe.
    Kill(Vec<u32>),
    /// A secure screensaver locks the session; the launch fails with this text.
    Refuse(String),
}

/// `.scr` image names (case-insensitive) among `(pid, exe file name)` entries.
pub fn scr_entries(entries: impl IntoIterator<Item = (u32, String)>) -> Vec<(u32, String)> {
    let is_scr = |exe: &String| exe.to_ascii_lowercase().ends_with(".scr");
    entries.into_iter().filter(|(_, exe)| is_scr(exe)).collect()
}

/// Pure. `scr` is every running `.scr` process as (pid, name); `secure` is
/// `SPI_GETSCREENSAVESECURE`.
pub fn screensaver_action(scr: &[(u32, String)], secure: bool) -> SaverAction {
    match scr.first() {
        None => SaverAction::None,
        Some((pid, name)) if secure => SaverAction::Refuse(format!(
            "screensaver {name} (pid {pid}) is running and locks the session; unlock the lab machine"
        )),
        Some(_) => SaverAction::Kill(scr.iter().map(|(pid, _)| *pid).collect()),
    }
}

/// Append the running `.scr` processes to a probe failure message.
pub fn with_running_scr(msg: String, scr: &[(u32, String)]) -> String {
    if scr.is_empty() {
        return msg;
    }
    let list: Vec<String> = scr.iter().map(|(p, n)| format!("{n} pid {p}")).collect();
    format!("{msg} (running: {})", list.join(", "))
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;

    use crate::supervisor::process;

    const SPI_GETSCREENSAVERRUNNING: u32 = 0x0072;
    const SPI_GETSCREENSAVESECURE: u32 = 0x0076;

    fn spi_flag(action: u32) -> bool {
        let mut v: i32 = 0;
        // SAFETY: both actions write one BOOL to the pointer.
        unsafe {
            SystemParametersInfoW(action, 0, &mut v as *mut i32 as *mut c_void, 0) != 0 && v != 0
        }
    }

    pub fn screensaver_running() -> bool {
        spi_flag(SPI_GETSCREENSAVERRUNNING)
    }

    pub fn screensaver_secure() -> bool {
        spi_flag(SPI_GETSCREENSAVESECURE)
    }

    /// `IDirect3D9::GetDeviceCaps(0, D3DDEVTYPE_HAL)`; `Ok(())` when usable.
    pub fn probe_d3d() -> Result<(), u32> {
        type Create = unsafe extern "system" fn(u32) -> *mut *const usize;
        type Release = unsafe extern "system" fn(*mut *const usize) -> u32;
        type Caps = unsafe extern "system" fn(*mut *const usize, u32, i32, *mut u8) -> i32;
        let name: Vec<u16> = "d3d9.dll\0".encode_utf16().collect();
        // SAFETY: d3d9's exported Direct3DCreate9 and the IDirect3D9 vtable
        // (slot 2 Release, slot 14 GetDeviceCaps) have these signatures; the
        // caps buffer is larger than D3DCAPS9 (304 bytes).
        unsafe {
            let lib = LoadLibraryW(name.as_ptr());
            if lib.is_null() {
                return Err(0xffff_ffff);
            }
            let Some(f) = GetProcAddress(lib, c"Direct3DCreate9".as_ptr().cast()) else {
                return Err(0xffff_ffff);
            };
            let create: Create = std::mem::transmute(f);
            let d3d = create(32);
            if d3d.is_null() {
                return Err(0xffff_ffff);
            }
            let vt = *d3d;
            let caps: Caps = std::mem::transmute(*vt.add(14));
            let release: Release = std::mem::transmute(*vt.add(2));
            let mut buf = [0u8; 1024];
            let hr = caps(d3d, 0, 1, buf.as_mut_ptr());
            release(d3d);
            if hr == 0 {
                Ok(())
            } else {
                Err(hr as u32)
            }
        }
    }

    /// Kill the given screensaver pids.
    pub fn dismiss_screensaver(pids: &[u32]) -> usize {
        pids.iter().for_each(|&pid| process::terminate(pid));
        pids.len()
    }

    /// Every running process as `(pid, exe file name)`, from a Toolhelp snapshot.
    pub fn process_entries() -> Vec<(u32, String)> {
        let mut entries = Vec::new();
        // SAFETY: a process snapshot walked with a correctly sized entry; the
        // snapshot handle is closed below.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return Vec::new();
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snap, &mut entry) != 0;
            while more {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                entries.push((
                    entry.th32ProcessID,
                    String::from_utf16_lossy(&entry.szExeFile[..len]),
                ));
                more = Process32NextW(snap, &mut entry) != 0;
            }
            CloseHandle(snap);
        }
        entries
    }

    /// Pids of every running `SGW.exe`, whether or not it has a window.
    pub fn sgw_process_pids() -> Vec<u32> {
        crate::supervisor::display::sgw_pids_in(process_entries())
    }

    /// The keep-awake thread: holds the display and system awake while any
    /// `SGW.exe` runs, checked every two seconds.
    pub fn spawn_keep_awake() {
        let _ = std::thread::Builder::new()
            .name("lab-keep-awake".into())
            .spawn(|| {
                let mut held = false;
                loop {
                    let pids = sgw_process_pids();
                    let want = !pids.is_empty();
                    if want != held {
                        let flags = if want {
                            ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED
                        } else {
                            ES_CONTINUOUS
                        };
                        // SAFETY: per-thread execution state; this thread lives
                        // as long as the supervisor.
                        unsafe { SetThreadExecutionState(flags) };
                        tracing::info!(
                            held = want,
                            sgw_pids = pids.len(),
                            "lab display keep-awake"
                        );
                        held = want;
                    }
                    std::thread::sleep(Duration::from_secs(2));
                }
            });
    }
}

/// Start the keep-awake thread; later calls do nothing.
pub fn spawn_keep_awake() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        #[cfg(windows)]
        win::spawn_keep_awake();
    });
}

/// Make sure a launched client can create its Direct3D device.
pub fn ensure_display_for_launch() -> Result<(), String> {
    #[cfg(windows)]
    {
        // Enumerated before the probe, whatever SPI says: a saver on its own
        // desktop is invisible to the window walk.
        let scr = scr_entries(win::process_entries());
        let secure = win::screensaver_secure();
        match screensaver_action(&scr, secure) {
            SaverAction::Refuse(reason) => {
                tracing::warn!(target: "lab.client", event = "screensaver_refused", reason = %reason, "lab launch: refused, a secure screensaver is running");
                return Err(reason);
            }
            SaverAction::Kill(pids) => dismiss_and_settle(&scr, &pids),
            SaverAction::None => {}
        }
        if win::probe_d3d().is_ok() {
            tracing::info!(target: "lab.client", event = "display_probe", ok = true, "lab launch: Direct3D probe ok");
            return Ok(());
        }
        // Second check: SPI may still report a non-secure saver.
        if win::screensaver_running() && !secure {
            let fresh = scr_entries(win::process_entries());
            if let SaverAction::Kill(pids) = screensaver_action(&fresh, false) {
                dismiss_and_settle(&fresh, &pids);
            }
        }
        return match win::probe_d3d() {
            Ok(()) => Ok(()),
            Err(hr) => {
                let msg = describe_d3d_failure(hr, win::screensaver_running(), secure);
                Err(with_running_scr(msg, &scr_entries(win::process_entries())))
            }
        };
    }
    #[allow(unreachable_code)]
    Ok(())
}

/// Kill the non-secure screensavers `pids` (all of `scr`), then give the
/// desktop two seconds.
#[cfg(windows)]
fn dismiss_and_settle(scr: &[(u32, String)], pids: &[u32]) {
    let names: Vec<&str> = scr.iter().map(|(_, name)| name.as_str()).collect();
    let n = win::dismiss_screensaver(pids);
    tracing::warn!(dismissed = n, names = %names.join(", "), "lab launch: dismissed a non-secure screensaver");
    std::thread::sleep(std::time::Duration::from_secs(2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusal_names_the_cause() {
        let s = describe_d3d_failure(D3DERR_NOTAVAILABLE, true, false);
        assert!(
            s.contains("D3DERR_NOTAVAILABLE") && s.contains("screensaver is still running"),
            "{s}"
        );
        let s = describe_d3d_failure(D3DERR_NOTAVAILABLE, true, true);
        assert!(s.contains("password-protected"), "{s}");
        let s = describe_d3d_failure(0x8876_0868, false, false);
        assert!(
            s.contains("0x88760868") && s.contains("display is off"),
            "{s}"
        );
    }

    /// Regression guard (2026-10-12): the keep-awake counted only an SGW.exe
    /// with a top-level window, so a client hung before its window never
    /// held the display. Every SGW.exe process counts, whatever else runs.
    #[test]
    fn a_windowless_client_process_keeps_the_display_awake() {
        let procs = vec![
            (100, "explorer.exe".to_string()),
            (4242, "SGW.exe".to_string()),
            (4300, "sgw.EXE".to_string()),
            (4400, "NotSGW.exe".to_string()),
            (4500, "SGW.exe.bak".to_string()),
        ];
        assert_eq!(sgw_pids_in(procs), vec![4242, 4300]);
        assert!(
            sgw_pids_in(Vec::new()).is_empty(),
            "no client, nothing held"
        );
    }

    /// Fails on revert: without the suffix match, `scrcons.exe` and `foo.scr.exe`
    /// would count as screensavers.
    #[test]
    fn scr_entries_match_by_suffix() {
        let entries = vec![
            (1, "Ribbons.SCR".to_string()),
            (2, "ribbons.scr".to_string()),
            (3, "scrcons.exe".to_string()),
            (4, "foo.scr.exe".to_string()),
            (5, "SGW.exe".to_string()),
        ];
        let kept = vec![
            (1, "Ribbons.SCR".to_string()),
            (2, "ribbons.scr".to_string()),
        ];
        assert_eq!(scr_entries(entries), kept);
    }

    #[test]
    fn no_scr_no_action() {
        assert_eq!(screensaver_action(&[], false), SaverAction::None);
    }

    /// Fails on revert: a non-secure saver is killed (C18).
    #[test]
    fn non_secure_scr_is_killed() {
        let scr = vec![(4242, "Ribbons.scr".to_string())];
        assert_eq!(
            screensaver_action(&scr, false),
            SaverAction::Kill(vec![4242])
        );
    }

    /// Fails on revert: a secure saver is refused by name and pid (C18).
    #[test]
    fn secure_scr_is_refused_by_name_and_pid() {
        let scr = vec![(4242, "Ribbons.scr".to_string())];
        match screensaver_action(&scr, true) {
            SaverAction::Refuse(s) => {
                assert!(s.contains("Ribbons.scr") && s.contains("4242"), "{s}")
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// Fails on revert: the probe failure must name the running saver.
    #[test]
    fn probe_failure_names_running_scr() {
        let scr = [(7, "a.scr".to_string())];
        let s = with_running_scr("x".into(), &scr);
        assert!(s.contains("a.scr pid 7"), "{s}");
        assert_eq!(with_running_scr("x".into(), &[]), "x");
    }
}
