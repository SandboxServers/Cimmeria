//! Same suspended/inject/resume primitives as start32, retaining the original
//! process handle so an immediate exit cannot race a later OpenProcess. With
//! `place_on`, a detached thread moves the game window to that monitor (PD-33).
use super::*;
use cimmeria_client_launch::{
    inject::{inject_dll, InjectError},
    launch::strip_verbatim,
    process::create_process_suspended_with_args,
};
use std::ffi::OsString;
use std::io::{Read, Write};

/// The game's exe and working directory, without the verbatim prefix.
///
/// A native launcher sends canonical `\\?\C:\...` paths. Windows does not
/// resolve `..` under a verbatim working directory, and SGW.exe finds its
/// config as `..\SGWGame\Config\`, so it would quit with "Failed to find
/// default engine .ini file". A Wine guest path is already plain.
fn game_paths(request: &Request) -> (PathBuf, PathBuf) {
    (
        strip_verbatim(request.exe.clone()),
        strip_verbatim(request.directory.clone()),
    )
}

/// Writes one observation line. The stdout lock keeps the placement thread's
/// line and the main thread's lines whole.
fn send(operation_id: Uuid, observation: Event) -> Result<(), &'static str> {
    let message = Message {
        schema_version: 1,
        operation_id,
        observation,
    };
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, &message).map_err(|_| "output")?;
    out.write_all(b"\n")
        .and_then(|_| out.flush())
        .map_err(|_| "output")
}

/// The Win32 code of an injector failure, when it has one. Never the DLL path.
fn win32_of(error: &InjectError) -> Option<u32> {
    match error {
        InjectError::Win32 { code, .. } => Some(*code),
        _ => None,
    }
}

pub fn run() -> Result<(), &'static str> {
    if !cfg!(target_pointer_width = "32") {
        return Err("requires_x86");
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take((MAX_MESSAGE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "input")?;
    let request = Request::decode(&bytes)?;
    let emit = |observation| send(request.operation_id, observation);
    if !request.exe.is_absolute()
        || !request.directory.is_absolute()
        || request.exe.parent() != Some(request.directory.as_path())
        || request
            .dlls
            .iter()
            .any(|p| !p.is_absolute() || !p.is_file())
        || !valid_args(&request.args)
    {
        return emit(Event::NotStarted {
            stage: Some(Stage::Request),
            win32: None,
        });
    }
    let (exe, directory) = game_paths(&request);
    // An empty list gives the same command line as before: the bare exe.
    let args: Vec<OsString> = request.args.iter().map(OsString::from).collect();
    let suspended = match create_process_suspended_with_args(&exe, &args, Some(&directory)) {
        Ok(process) => process,
        Err(e) => {
            return emit(Event::NotStarted {
                stage: Some(Stage::CreateProcess),
                win32: win32_of(&e),
            })
        }
    };
    for dll in &request.dlls {
        if let Err(e) = inject_dll(suspended.process_handle(), dll) {
            // The shared termination primitive is best effort, not proof of exit.
            suspended.terminate();
            return emit(Event::Unknown {
                stage: Some(Stage::Inject),
                win32: win32_of(&e),
            });
        }
    }
    let running = match suspended.resume_running() {
        Ok(process) => process,
        Err(e) => {
            return emit(Event::Unknown {
                stage: Some(Stage::Resume),
                win32: win32_of(&e),
            })
        }
    };
    let guest_pid = running.pid();
    // Loss of the launcher's pipe must not drop our process observation early.
    let started = emit(Event::ProcessStarted { guest_pid });
    if let Some(target) = request.place_on {
        // Detached: placement never delays the wait below, and a game that
        // exits first simply ends the helper with it.
        let operation_id = request.operation_id;
        let _ = std::thread::Builder::new()
            .name("place-window".into())
            .spawn(move || {
                let outcome = window_placement::place(guest_pid, target);
                let _ = send(operation_id, Event::WindowPlaced { outcome });
            });
    }
    let exited = running.wait();
    started?;
    match exited {
        Ok(code) => emit(Event::ProcessExited { guest_pid, code }),
        Err(e) => emit(Event::Unknown {
            stage: Some(Stage::Wait),
            win32: e.raw_os_error().and_then(|c| u32::try_from(c).ok()),
        }),
    }
}

/// Moves the game window to the remembered monitor (PD-33), by raw FFI to
/// user32 and shcore, like the engine's `display_targets` (no new `windows-sys`
/// features). The pure rules are in `placement.rs`.
mod window_placement {
    use super::super::placement::{centre, choose, game_units, scale_percent_from_dpi};
    use super::super::PlacementOutcome;
    use std::ffi::c_void;
    use std::time::{Duration, Instant};

    type Bool = i32;
    type Hwnd = *mut c_void;
    type HMonitor = *mut c_void;
    type Hdc = *mut c_void;
    type LParam = isize;
    type HResult = i32;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    /// `MONITORINFO`.
    #[repr(C)]
    #[derive(Default)]
    #[allow(dead_code)] // layout field the API fills; `dw_flags` is not read
    struct MonitorInfo {
        cb_size: u32,
        rc_monitor: Rect,
        rc_work: Rect,
        dw_flags: u32,
    }
    const _: () = assert!(std::mem::size_of::<MonitorInfo>() == 40);

    const GW_OWNER: u32 = 4;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const MDT_EFFECTIVE_DPI: i32 = 0;

    type WndEnumProc = unsafe extern "system" fn(Hwnd, LParam) -> Bool;
    type MonitorEnumProc = unsafe extern "system" fn(HMonitor, Hdc, *mut Rect, LParam) -> Bool;

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(callback: WndEnumProc, data: LParam) -> Bool;
        fn GetWindowThreadProcessId(window: Hwnd, process_id: *mut u32) -> u32;
        fn IsWindowVisible(window: Hwnd) -> Bool;
        fn GetWindow(window: Hwnd, command: u32) -> Hwnd;
        fn GetWindowRect(window: Hwnd, rect: *mut Rect) -> Bool;
        fn SetWindowPos(
            window: Hwnd,
            insert_after: Hwnd,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> Bool;
        fn MonitorFromWindow(window: Hwnd, flags: u32) -> HMonitor;
        fn EnumDisplayMonitors(
            hdc: Hdc,
            clip: *const Rect,
            callback: MonitorEnumProc,
            data: LParam,
        ) -> Bool;
        fn GetMonitorInfoW(monitor: HMonitor, info: *mut MonitorInfo) -> Bool;
    }

    #[link(name = "shcore")]
    extern "system" {
        fn GetDpiForMonitor(
            monitor: HMonitor,
            dpi_type: i32,
            dpi_x: *mut u32,
            dpi_y: *mut u32,
        ) -> HResult;
    }

    /// How long the game gets to show its window, and how often it is looked for.
    const WAIT: Duration = Duration::from_secs(60);
    const POLL: Duration = Duration::from_millis(250);
    /// When the window is checked again, in case the game moved it back.
    const RECHECK: Duration = Duration::from_secs(3);
    /// More monitors than anyone connects; bounds the list.
    const MAX_MONITORS: usize = 32;

    fn as_xywh(r: Rect) -> [i32; 4] {
        [
            r.left,
            r.top,
            r.right.saturating_sub(r.left),
            r.bottom.saturating_sub(r.top),
        ]
    }

    /// The search state `find_window` passes through `EnumWindows`.
    struct Search {
        pid: u32,
        found: Hwnd,
    }

    unsafe extern "system" fn find_window(window: Hwnd, data: LParam) -> Bool {
        // SAFETY: `data` is the `&mut Search` `game_window` passes, alive for the call.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut pid = 0u32;
        // SAFETY: `window` came from EnumWindows; `pid` is a live local.
        unsafe { GetWindowThreadProcessId(window, &mut pid) };
        // SAFETY: plain queries on a window handle EnumWindows gave us.
        let main = pid == search.pid
            && unsafe { IsWindowVisible(window) } != 0
            && unsafe { GetWindow(window, GW_OWNER) }.is_null();
        if main {
            search.found = window;
            return 0; // stop enumerating
        }
        1
    }

    /// A visible, unowned top-level window of the game's process.
    fn game_window(pid: u32) -> Option<Hwnd> {
        let mut search = Search {
            pid,
            found: std::ptr::null_mut(),
        };
        // SAFETY: the callback only writes into `search`, which outlives the call.
        // EnumWindows returns 0 when the callback stops it, so its result is unused.
        unsafe { EnumWindows(find_window, &mut search as *mut Search as LParam) };
        (!search.found.is_null()).then_some(search.found)
    }

    unsafe extern "system" fn collect(
        monitor: HMonitor,
        _: Hdc,
        _: *mut Rect,
        data: LParam,
    ) -> Bool {
        // SAFETY: `data` is the `&mut Vec<HMonitor>` `monitors` passes, alive for the call.
        let list = unsafe { &mut *(data as *mut Vec<HMonitor>) };
        list.push(monitor);
        Bool::from(list.len() < MAX_MONITORS)
    }

    /// One connected monitor: its handle, its rectangle in game units (for
    /// `choose`) and its work area in this process's units (for `centre`).
    struct Monitor {
        handle: HMonitor,
        game: [i32; 4],
        work: [i32; 4],
    }

    fn monitors() -> Vec<Monitor> {
        let mut handles: Vec<HMonitor> = Vec::new();
        // SAFETY: the callback only pushes into `handles`, which outlives the call.
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                collect,
                &mut handles as *mut Vec<HMonitor> as LParam,
            )
        };
        handles
            .into_iter()
            .filter_map(|handle| {
                let mut info = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..MonitorInfo::default()
                };
                // SAFETY: `info` is a correctly sized MONITORINFO with `cbSize` set.
                if unsafe { GetMonitorInfoW(handle, &mut info) } == 0 {
                    return None;
                }
                let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
                // SAFETY: `handle` came from EnumDisplayMonitors; both out-pointers are live locals.
                let hr =
                    unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
                let scale = if hr < 0 {
                    100
                } else {
                    scale_percent_from_dpi(dpi_x)
                };
                Some(Monitor {
                    handle,
                    game: game_units(as_xywh(info.rc_monitor), scale),
                    work: as_xywh(info.rc_work),
                })
            })
            .collect()
    }

    fn monitor_of(window: Hwnd) -> HMonitor {
        // SAFETY: a plain query; an invalid window gives the nearest monitor or null.
        unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) }
    }

    /// Centres the window in `work`, keeping its size, z-order and focus.
    fn move_to(window: Hwnd, work: [i32; 4]) {
        let mut rect = Rect::default();
        // SAFETY: `rect` is a live local; a failed call leaves it zeroed.
        unsafe { GetWindowRect(window, &mut rect) };
        let (x, y) = centre(work, as_xywh(rect));
        // SAFETY: SWP_NOSIZE and SWP_NOZORDER make the size and insert-after arguments unused.
        unsafe {
            SetWindowPos(
                window,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
    }

    /// Waits for the game's window and puts it on the monitor `target` names.
    pub(super) fn place(pid: u32, target: [i32; 4]) -> PlacementOutcome {
        let deadline = Instant::now() + WAIT;
        let window = loop {
            if let Some(window) = game_window(pid) {
                break window;
            }
            if Instant::now() >= deadline {
                return PlacementOutcome::NoWindow;
            }
            std::thread::sleep(POLL);
        };
        let monitors = monitors();
        let rects: Vec<[i32; 4]> = monitors.iter().map(|m| m.game).collect();
        let Some(chosen) = choose(&rects, target).map(|i| &monitors[i]) else {
            return PlacementOutcome::NoMonitor;
        };
        if monitor_of(window) == chosen.handle {
            return PlacementOutcome::AlreadyThere;
        }
        move_to(window, chosen.work);
        std::thread::sleep(RECHECK);
        if monitor_of(window) != chosen.handle {
            move_to(window, chosen.work);
        }
        PlacementOutcome::Placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(exe: &str, directory: &str) -> Request {
        Request {
            schema_version: 1,
            operation_id: Uuid::from_u128(1),
            exe: exe.into(),
            directory: directory.into(),
            dlls: Vec::new(),
            args: Vec::new(),
            place_on: None,
        }
    }

    // Bug shape: Play on Windows started SGW.exe under `\\?\C:\...` and the
    // game quit at once with "Failed to find default engine .ini file".
    #[test]
    fn a_verbatim_request_starts_the_game_under_plain_paths() {
        let (exe, directory) = game_paths(&request(
            r"\\?\C:\Games\SGW\Working\Binaries\SGW.exe",
            r"\\?\C:\Games\SGW\Working\Binaries",
        ));
        assert_eq!(exe, PathBuf::from(r"C:\Games\SGW\Working\Binaries\SGW.exe"));
        assert_eq!(directory, PathBuf::from(r"C:\Games\SGW\Working\Binaries"));
    }

    #[test]
    fn a_plain_guest_request_is_unchanged() {
        let plain = (
            r"C:\Games\SGW\Working\Binaries\SGW.exe",
            r"C:\Games\SGW\Working\Binaries",
        );
        let (exe, directory) = game_paths(&request(plain.0, plain.1));
        assert_eq!((exe, directory), (plain.0.into(), plain.1.into()));
    }
}
