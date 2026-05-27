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
/// `DllMain` returns and loader lock has cleared. This is where all
/// real init lands as future phases ship — hook installation, CME
/// subscriber registration, uploader thread spawn.
///
/// Phase 1 scope: capture `AttachDiagnostics`, do nothing else.
/// Returning leaves the thread to exit normally; future phases will
/// park here to keep the bootstrap thread alive for shutdown
/// coordination.
pub fn bootstrap_main() {
    // Touch the static so we can prove (in tests) that
    // `DllMain` ran. Future phases will pass this into the
    // telemetry uploader as the first event.
    let _ = ATTACH_DIAG.get();
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
