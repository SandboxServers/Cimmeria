//! Outer-tier crash capture for the injected DLL (issue #685 scope 4,
//! shared with #686).
//!
//! This is the **last-resort, process-wide** tier of the two-tier crash
//! scheme the #686 spike settled on. It does exactly three things and
//! cannot do more: on an unhandled fault it writes a minidump, flushes
//! the crash marker (so the supervisor knows a command was in flight —
//! ADR §6 quarantine), and terminates fast. It never tries to *resume*;
//! that is the inner per-dispatch `microseh` tier, which is #686's job
//! and is deliberately **not** added here.
//!
//! # Chain vs. replace of UE3's own filter — decision
//!
//! UE3 installs its own top-level filter (ADR §10 open question 2). We
//! **replace** it for the lab: our filter is installed last (so it runs
//! first), writes our evidence, and calls `TerminateProcess` — it never
//! returns to UE3's filter or the CRT default. Rationale: UE3's handler
//! pops a crash dialog and runs its own reporter, which would block the
//! supervisor's fast relaunch (the whole point of ADR §6 is that a crash
//! costs the agent a minute, not the session). We capture the previous
//! filter pointer for diagnostics but do not chain to it. WER's dialog
//! is suppressed separately by the supervisor (`SetErrorMode` on the
//! child) so a fault never sits on a modal the watchdog would have to
//! time out.
//!
//! # Stack overflow
//!
//! Per the spike, `STATUS_STACK_OVERFLOW` cannot be caught reliably by a
//! top-level filter (the handler needs stack that no longer exists) and
//! is **not recoverable**. We install a vectored handler (runs
//! first-chance, before the stack is fully unwound) plus
//! `SetThreadStackGuarantee` to reserve enough stack for the handler to
//! run, take a minidump, and terminate. The bridge runs on the game's
//! pre-existing main thread, which never went through Rust startup, so
//! Rust's own stack-overflow net was never installed there — this is
//! why we install our own.
//!
//! # Signal-safety
//!
//! The filter reads only [`journal::is_in_flight`] (a lock-free
//! `AtomicBool`) and a `OnceLock` dump directory. It does not take any
//! lock a faulting thread might hold (the #686 rule). File writes are
//! best-effort; a fault deep in a corrupt heap may leave no dump, and
//! that is acceptable — the supervisor still detects the exit.

use std::path::PathBuf;
use std::sync::OnceLock;

use super::journal;

/// Directory the minidump + marker are written to (the session dir:
/// `<install>/Binaries/sessions/`). Set once by [`install`].
static DUMP_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Minidump filename for a crash at `ts_ms`. Pure — pinned by tests so
/// the supervisor and the DLL agree on where to look.
pub fn minidump_filename(ts_ms: i64) -> String {
    format!("lab-minidump-{ts_ms}.dmp")
}

/// The fixed crash-marker filename. The supervisor reads this in
/// `lab_crash_report`.
pub fn marker_filename() -> &'static str {
    "lab-crash-marker.json"
}

/// Wall-clock milliseconds since the Unix epoch. Best-effort; returns 0
/// if the clock is somehow before the epoch.
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Install the outer-tier crash handlers, writing evidence to
/// `dump_dir`. Idempotent-ish: a second call keeps the first dir. Safe
/// to call from the bootstrap thread after the bridge starts.
///
/// On non-i686-Windows this only records the dump dir (the native
/// handlers don't exist off-target), so the pure path stays testable.
pub fn install(dump_dir: PathBuf) {
    let _ = DUMP_DIR.set(dump_dir);
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    win::install_native();
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod win {
    use super::{journal, marker_filename, minidump_filename, now_ms, DUMP_DIR};
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Diagnostics::Debug::{
        AddVectoredExceptionHandler, MiniDumpWriteDump, SetErrorMode, SetUnhandledExceptionFilter,
        EXCEPTION_POINTERS, MINIDUMP_EXCEPTION_INFORMATION,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId, SetThreadStackGuarantee,
        TerminateProcess,
    };

    /// `STATUS_STACK_OVERFLOW`. Defined locally so we don't depend on a
    /// windows-sys module path that has moved between versions.
    const EXCEPTION_STACK_OVERFLOW: u32 = 0xC000_00FD;
    /// Let the next handler run (used by the vectored handler for every
    /// non-stack-overflow fault — the top-level filter deals with those).
    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
    /// `MiniDumpNormal` — smallest, fastest dump. Enough for a stack +
    /// module list; we are not trying to snapshot the whole heap from a
    /// possibly-corrupt process.
    const MINIDUMP_NORMAL: i32 = 0;
    /// Reserve this much stack for the stack-overflow handler to run in.
    const STACK_GUARANTEE_BYTES: u32 = 16 * 1024;

    /// `SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX` — suppress the
    /// WER "SGW.exe has stopped working" dialog so a fault never sits on
    /// a modal the supervisor's watchdog would have to time out.
    const SEM_SUPPRESS: u32 = 0x0001 | 0x0002;

    pub(super) fn install_native() {
        // Suppress the WER crash dialog for this process (issue #685
        // scope 3). Called from the DLL (the child), which is the right
        // place per the spike.
        // SAFETY: SetErrorMode only adjusts this process's error mode.
        unsafe {
            SetErrorMode(SEM_SUPPRESS);
        }

        // Reserve handler stack on the current (game main) thread.
        let mut bytes = STACK_GUARANTEE_BYTES;
        // SAFETY: `bytes` is a valid in/out u32; the call only adjusts
        // this thread's guard reservation.
        unsafe {
            let _ = SetThreadStackGuarantee(&mut bytes);
        }

        // Vectored handler first: it is the only reliable catch for a
        // stack overflow. `first = 1` puts us at the front of the chain.
        // SAFETY: a valid extern "system" callback pointer.
        unsafe {
            AddVectoredExceptionHandler(1, Some(vectored_handler));
        }

        // Replace UE3's top-level filter. We keep the previous pointer
        // for diagnostics but never chain to it (see module docs).
        // SAFETY: a valid extern "system" filter pointer.
        let _previous = unsafe { SetUnhandledExceptionFilter(Some(unhandled_filter)) };
    }

    /// Top-level unhandled-exception filter. Writes evidence and
    /// terminates — never returns (the `!` coerces to the ABI's i32).
    unsafe extern "system" fn unhandled_filter(info: *const EXCEPTION_POINTERS) -> i32 {
        capture_and_die(info)
    }

    /// Vectored handler. Only acts on stack overflow (which the
    /// top-level filter can't catch); everything else falls through to
    /// the normal chain, where our top-level filter handles it.
    ///
    /// `PVECTORED_EXCEPTION_HANDLER` takes `*mut EXCEPTION_POINTERS` (vs
    /// the top-level filter's `*const`); we only read through it.
    unsafe extern "system" fn vectored_handler(info: *mut EXCEPTION_POINTERS) -> i32 {
        let info = info as *const EXCEPTION_POINTERS;
        if let Some(code) = exception_code(info) {
            if code == EXCEPTION_STACK_OVERFLOW {
                capture_and_die(info);
            }
        }
        EXCEPTION_CONTINUE_SEARCH
    }

    /// Read the STATUS_* code out of the exception pointers, if present.
    unsafe fn exception_code(info: *const EXCEPTION_POINTERS) -> Option<u32> {
        let rec = (*info).ExceptionRecord;
        if rec.is_null() {
            return None;
        }
        Some((*rec).ExceptionCode as u32)
    }

    /// Read the faulting instruction address, if present.
    unsafe fn exception_address(info: *const EXCEPTION_POINTERS) -> u64 {
        let rec = (*info).ExceptionRecord;
        if rec.is_null() {
            return 0;
        }
        (*rec).ExceptionAddress as usize as u64
    }

    /// Write the minidump + marker, then terminate the process. Never
    /// returns.
    unsafe fn capture_and_die(info: *const EXCEPTION_POINTERS) -> ! {
        let ts = now_ms();
        let code = exception_code(info).unwrap_or(0);
        let addr = exception_address(info);
        let tid = GetCurrentThreadId();
        let in_flight = journal::is_in_flight();

        let dump_name = write_minidump(info, ts);
        write_marker(code, addr, tid, ts, in_flight, dump_name);

        // Fast, deterministic terminate — no UE3 dialog, no CRT atexit.
        // Exit code 0xDEAD is arbitrary but distinguishable in logs.
        TerminateProcess(GetCurrentProcess(), 0xDEAD);
        // TerminateProcess does not return for the current process, but
        // the type system needs a diverging tail.
        loop {
            core::hint::spin_loop();
        }
    }

    /// Write a minidump to `<dump_dir>/lab-minidump-<ts>.dmp`. Returns
    /// the filename on success. Uses a `std::fs::File` handle rather
    /// than `CreateFileW` to keep the native surface small.
    unsafe fn write_minidump(info: *const EXCEPTION_POINTERS, ts: i64) -> Option<String> {
        let dir = DUMP_DIR.get()?;
        let name = minidump_filename(ts);
        let path = dir.join(&name);
        let file = std::fs::File::create(&path).ok()?;
        let hfile = file.as_raw_handle() as HANDLE;

        let mut exc = MINIDUMP_EXCEPTION_INFORMATION {
            ThreadId: GetCurrentThreadId(),
            ExceptionPointers: info as *mut EXCEPTION_POINTERS,
            ClientPointers: 0, // pointers are in *our* address space
        };

        // SAFETY: valid process + file handles; `exc` outlives the call.
        let ok = MiniDumpWriteDump(
            GetCurrentProcess(),
            GetCurrentProcessId(),
            hfile,
            MINIDUMP_NORMAL,
            &mut exc as *mut _,
            core::ptr::null(),
            core::ptr::null(),
        );
        if ok != 0 {
            Some(name)
        } else {
            None
        }
    }

    /// Write the crash marker JSON. Best-effort.
    fn write_marker(
        code: u32,
        addr: u64,
        tid: u32,
        ts: i64,
        in_flight: bool,
        dump_name: Option<String>,
    ) {
        let Some(dir) = DUMP_DIR.get() else {
            return;
        };
        let marker = journal::CrashMarker::new(code, addr, tid, ts, in_flight, dump_name);
        let _ = std::fs::write(dir.join(marker_filename()), marker.to_json_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minidump_filename_is_stable() {
        assert_eq!(
            minidump_filename(1_700_000_000_000),
            "lab-minidump-1700000000000.dmp"
        );
    }

    #[test]
    fn marker_filename_is_stable() {
        assert_eq!(marker_filename(), "lab-crash-marker.json");
    }

    /// `install` records the dump dir even off-target so the pure path
    /// is exercised; the native handlers are a no-op here.
    #[test]
    fn install_records_dump_dir_offtarget() {
        // OnceLock means only the first install in the process wins; use
        // whatever value is present, just assert install doesn't panic.
        install(std::path::PathBuf::from("."));
    }
}
