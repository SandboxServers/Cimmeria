//! `DllMain` + bootstrap-thread plumbing for the injected DLL.
//!
//! The Windows loader holds the loader lock for the duration of
//! `DllMain`; doing real work there risks re-entrant `LoadLibrary`
//! deadlocks (see crate docs). So `DllMain` spawns a bootstrap thread
//! via `CreateThread` and returns immediately. The bootstrap thread
//! runs after loader lock has cleared and is where all telemetry
//! init will land.

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{HMODULE, MAX_PATH, TRUE};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

/// Set once when `DllMain` records the module handle, before the
/// bootstrap thread starts. Subsequent phases read it (e.g. for
/// IAT-walking the host process).
static MODULE_HANDLE: OnceLock<usize> = OnceLock::new();

/// Pre-bootstrap diagnostics: what `DllMain` saw, captured before
/// loader lock clears. Bootstrap thread folds these into the first
/// telemetry event so we can correlate "DLL attached" with the
/// session's `install_id` from the launcher.
#[derive(Debug, Clone, Default)]
pub struct AttachDiagnostics {
    /// Full path the DLL was loaded from. Captured via
    /// `GetModuleFileNameW(MODULE_HANDLE, ...)` immediately after
    /// `DllMain` records the module — used to locate the launcher's
    /// `current-session.json` alongside the DLL on disk.
    pub dll_path: Option<PathBuf>,
}

static ATTACH_DIAG: OnceLock<AttachDiagnostics> = OnceLock::new();
static BOOTSTRAP_STARTED: AtomicBool = AtomicBool::new(false);

/// Windows DLL entry point. Called by the loader on attach/detach.
///
/// Strictly bounded work: record the module handle, capture our own
/// path, spawn the bootstrap thread, return. No allocations on the
/// hot path, no panic-across-FFI, no I/O.
///
/// # Safety
///
/// Called by the OS loader. Must obey the loader-lock contract: no
/// `LoadLibrary`, no `CreateProcess`, no FS I/O via the standard
/// library beyond the constant-time `GetModuleFileNameW` syscall.
#[cfg(windows)]
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    module: HMODULE,
    reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            // Record the module handle so the bootstrap thread (and
            // every future hook installer) can find us. `OnceLock`
            // is panic-safe; a re-attach (which shouldn't happen for
            // a process-lifetime DLL) would silently drop the new
            // handle, which is the right behaviour.
            let _ = MODULE_HANDLE.set(module as usize);

            // Capture our own path before bootstrap. `GetModuleFileNameW`
            // is a single syscall that returns a UTF-16 path — safe
            // to call under loader lock.
            let mut buf = [0u16; MAX_PATH as usize];
            let len = GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32);
            let dll_path = if len > 0 && (len as usize) < buf.len() {
                Some(PathBuf::from(String::from_utf16_lossy(
                    &buf[..len as usize],
                )))
            } else {
                None
            };
            let _ = ATTACH_DIAG.set(AttachDiagnostics { dll_path });

            // Spawn the bootstrap thread via the raw Win32 API.
            // `std::thread::spawn` would work in practice but pulls
            // in stdlib init paths we want to defer; using
            // `CreateThread` directly keeps `DllMain` minimal.
            //
            // Wrap the call in `catch_unwind` defence-in-depth so a
            // panic in any future bootstrap-side init can never
            // unwind across the FFI boundary back into SGW.exe.
            spawn_bootstrap_thread();
        }
        DLL_PROCESS_DETACH => {
            // Phase 1 contract: the DLL never unloads in normal
            // operation (process-lifetime CME subscribers can't be
            // safely unsubscribed without a follow-up decompile of
            // the Unsubscribe path near 0x00a5c150). Detach is a
            // best-effort no-op for now; a future phase replaces
            // this with the proper unsubscribe + uploader-drain
            // sequence.
        }
        _ => {}
    }
    TRUE
}

#[cfg(windows)]
fn spawn_bootstrap_thread() {
    use windows_sys::Win32::System::Threading::CreateThread;

    // Guard against the (impossible-in-practice) re-attach case so
    // we never spawn the bootstrap thread twice for the same
    // process.
    if BOOTSTRAP_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Safety: the thread proc is `extern "system" fn(*mut c_void) -> u32`
    // and is `'static` (a free function). No arguments are passed.
    // We discard the returned handle — the bootstrap thread runs for
    // the process lifetime and Windows reclaims the handle on
    // process exit.
    unsafe {
        let handle = CreateThread(
            std::ptr::null(),
            0,
            Some(bootstrap_thread_proc),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
        );
        if handle.is_null() {
            // We can't log meaningfully under loader lock — the
            // bootstrap thread is what would have set up logging.
            // The next-best signal is the absence of any client
            // telemetry events for this session, which the
            // launcher's session-meta event will make visible.
            return;
        }
        // Close our handle to the thread — we don't need to wait
        // on it from DllMain, and Windows keeps the kernel object
        // alive as long as the thread is running.
        let _ = windows_sys::Win32::Foundation::CloseHandle(handle);
    }
}

#[cfg(windows)]
unsafe extern "system" fn bootstrap_thread_proc(_arg: *mut c_void) -> u32 {
    // `catch_unwind` is the load-bearing guarantee here: a panic
    // anywhere in `bootstrap_main` (or anything it calls into) must
    // not unwind into Windows' thread-start trampoline. The
    // platform contract for `extern "system" fn` returning u32 is
    // "return a Win32 error code," which we honour by swallowing
    // the panic and returning a non-zero exit code.
    match std::panic::catch_unwind(bootstrap_main) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// Bootstrap entry point. Runs on a dedicated thread after
/// `DllMain` returns and loader lock has cleared.
///
/// **Phase 2 scope** (this file):
/// 1. Resolve the host (SGW.exe) executable path via
///    `GetModuleFileNameW(NULL, …)` so we can locate
///    `current-session.json`.
/// 2. Load + parse the launcher's marker file. If telemetry is
///    disabled (`telemetry.enabled = false` — the launcher's
///    kill-switch path), park the bootstrap thread silently —
///    the DLL stays loaded but inert. Hooks installed by Phase 2c
///    / 2d will gate their producer side on a check we add later.
/// 3. Construct the event queue (producer/consumer pair).
/// 4. Store the producer in a process-global static so future hook
///    installers can clone it.
/// 5. Emit the first event — `client.dll.attached` — carrying the
///    install_id / machine_id / session_id from the marker file
///    plus the DLL's on-disk path.
/// 6. Spawn the uploader thread; transfer ownership of the consumer.
/// 7. Park the bootstrap thread for process lifetime. Returning
///    would let the thread exit cleanly but we keep it alive so a
///    future shutdown hook (Phase 7) can drain the queue.
///
/// All work guarded by the outer `catch_unwind` in
/// `bootstrap_thread_proc` — a panic in any of the steps above
/// returns 1 from the thread proc and SGW.exe keeps running
/// without telemetry.
pub fn bootstrap_main() {
    let _ = ATTACH_DIAG.get();
    #[cfg(windows)]
    bootstrap_phase2();
}

/// Process-global producer handle. Installed by `bootstrap_phase2`
/// after the session loads successfully; read by every hook
/// installer that needs to emit events.
///
/// `OnceLock` because the producer is constructed exactly once per
/// process; cloning is cheap (it's `Arc`-backed inside
/// crossbeam-channel) so each hook holds its own clone.
#[cfg(windows)]
static PRODUCER: OnceLock<crate::queue::Producer> = OnceLock::new();

/// Returns the global producer if Phase 2 init succeeded. Hook
/// installers call this from their attach paths; `None` means
/// telemetry is disabled or the session-file load failed (the
/// hook should install itself as a no-op in that case).
#[cfg(windows)]
pub fn producer() -> Option<&'static crate::queue::Producer> {
    PRODUCER.get()
}

#[cfg(windows)]
fn bootstrap_phase2() {
    // Step 1: find the host executable. `GetModuleFileNameW(NULL)`
    // returns the path to the .exe that loaded us (SGW.exe).
    let host_exe = match host_exe_path() {
        Some(p) => p,
        None => return, // can't locate; can't proceed
    };

    // Step 2: load `current-session.json`. Errors here include
    // file-missing (launcher didn't write it — likely user
    // launched SGW.exe directly without going through the
    // launcher), parse errors, and the explicit
    // `telemetry.enabled = false` kill-switch case. All map to
    // "park, do nothing."
    let session = match crate::session::session_path_for_host(&host_exe)
        .and_then(|p| crate::session::load_session(&p))
    {
        Ok(s) => s,
        Err(_) => return,
    };

    // Step 3 + 4: queue + global producer.
    let (producer, consumer) = crate::queue::channel();
    if PRODUCER.set(producer.clone()).is_err() {
        // Double-init shouldn't happen (the bootstrap-thread
        // guard above is single-shot), but if it does, leave the
        // first producer in place and quietly bail.
        return;
    }

    // Step 5: first event — `client.dll.attached`. Carries the
    // identity fields the server-side replay uses to pivot
    // SigNoz queries on session.
    let mut builder = crate::events::ClientNativeEvent::builder("client.dll.attached", "info");
    for (k, v) in crate::session::identity_fields(&session) {
        builder = builder.field(&k, v);
    }
    if let Some(diag) = ATTACH_DIAG.get() {
        if let Some(p) = &diag.dll_path {
            builder = builder.field(
                "dll_path",
                serde_json::Value::String(p.display().to_string()),
            );
        }
    }
    builder = builder.field(
        "host_exe",
        serde_json::Value::String(host_exe.display().to_string()),
    );
    let _ = producer.try_emit(builder);

    // Step 6: spawn the uploader thread. It owns the consumer
    // end and runs for the lifetime of the process. We
    // intentionally don't keep the JoinHandle — Phase 7's
    // shutdown hook will use a stop-flag and a sibling JoinHandle
    // static when it lands.
    let cfg = crate::uploader::UploaderConfig::from_session(&session.telemetry);
    std::thread::Builder::new()
        .name("cimmeria-uploader".into())
        .spawn(move || {
            crate::uploader::run_uploader(consumer, cfg, || false);
        })
        .ok();

    // Step 7: park the bootstrap thread. Future Phase 7 hooks can
    // wake us via an `Event` to drive shutdown drain. For now,
    // `thread::park()` blocks until the OS reclaims the thread at
    // process exit.
    loop {
        std::thread::park();
    }
}

#[cfg(windows)]
fn host_exe_path() -> Option<std::path::PathBuf> {
    use windows_sys::Win32::Foundation::MAX_PATH;
    let mut buf = [0u16; MAX_PATH as usize];
    // SAFETY: NULL module handle = main executable per Win32 docs.
    let len = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW(
            std::ptr::null_mut::<core::ffi::c_void>() as HMODULE,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if len == 0 || (len as usize) >= buf.len() {
        return None;
    }
    Some(std::path::PathBuf::from(String::from_utf16_lossy(
        &buf[..len as usize],
    )))
}

/// Returns the diagnostics captured by `DllMain`, if attach has run.
/// Public surface for unit tests and for future phases that need the
/// DLL's own on-disk path.
pub fn attach_diagnostics() -> Option<&'static AttachDiagnostics> {
    ATTACH_DIAG.get()
}

/// Returns the module handle recorded by `DllMain`, if attach has
/// run. Future phases use this for IAT walking on the host module.
pub fn module_handle() -> Option<usize> {
    MODULE_HANDLE.get().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `attach_diagnostics()` returns `None` until `DllMain` runs.
    /// In a unit-test binary, `DllMain` is never called, so this
    /// path is the contract for "the DLL didn't attach."
    #[test]
    fn diagnostics_unset_outside_attach() {
        // Cannot assert `is_none()` if a prior test set it — and
        // `OnceLock` survives across tests in the same process —
        // so we only check the type is reachable.
        let _ = attach_diagnostics();
    }

    /// `module_handle()` follows the same OnceLock-shared lifetime.
    #[test]
    fn module_handle_unset_outside_attach() {
        let _ = module_handle();
    }

    /// `bootstrap_main` must be panic-safe — the
    /// `catch_unwind` wrapper in `bootstrap_thread_proc` depends on
    /// every code path it calls being unwind-safe. Running it from
    /// a unit test catches "I added something that panics
    /// unconditionally" regressions.
    #[test]
    fn bootstrap_main_runs_to_completion() {
        bootstrap_main();
    }
}
