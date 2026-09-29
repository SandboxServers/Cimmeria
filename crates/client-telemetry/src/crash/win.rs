//! The native half of crash capture: the IAT detours, our top-level
//! filter, reading the exception record and writing the minidump. The
//! decisions (what to record, once-only, chaining) live in the portable
//! modules next to this one.
//!
//! Every detour stays inside `catch_unwind` around our own code, then
//! calls the original with the game's arguments unchanged. The Win32
//! detours use `stdcall-unwind` and `exit` uses `C-unwind`, per the hook
//! ABI rule in `docs/architecture/client-telemetry.md`.

use std::ffi::c_void;
use std::os::windows::io::AsRawHandle;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, HMODULE, MAX_PATH};
use windows_sys::Win32::System::Diagnostics::Debug::{
    SetUnhandledExceptionFilter, EXCEPTION_POINTERS, MINIDUMP_EXCEPTION_INFORMATION,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GetModuleHandleW, GetProcAddress,
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId,
};

use super::chain::{filter_result, FilterChain};
use super::report::{module_basename, DumpOutcome, ExitPath, FaultModule};
use super::{CrashSource, Fault, STATE};
use crate::queue::Producer;

/// `MiniDumpNormal | MiniDumpWithUnloadedModules | MiniDumpWithThreadInfo`:
/// every thread's stack and registers, the module list, and which DLLs
/// came and went. A few MB. The client ships dbghelp 6.3, which knows
/// all three flags; if it refuses, [`FALLBACK_DUMP_TYPE`] is retried.
pub(super) const DUMP_TYPE: u32 = 0x0000_1021;
/// `MiniDumpNormal`.
pub(super) const FALLBACK_DUMP_TYPE: u32 = 0;

/// One import slot in SGW.exe's IAT (`.idata`, QA build).
#[derive(Debug, Clone, Copy)]
pub(super) struct Slot {
    pub(super) hook: &'static str,
    pub(super) slot: usize,
    pub(super) module: &'static str,
    pub(super) symbol: &'static core::ffi::CStr,
}

pub(super) const MINIDUMP_WRITE_DUMP: Slot = Slot {
    hook: "minidump_write_dump",
    slot: 0x017F_0058,
    module: "dbghelp.dll",
    symbol: c"MiniDumpWriteDump",
};
pub(super) const SET_UNHANDLED_EXCEPTION_FILTER: Slot = Slot {
    hook: "set_unhandled_exception_filter",
    slot: 0x017E_F108,
    module: "kernel32.dll",
    symbol: c"SetUnhandledExceptionFilter",
};
pub(super) const CRT_EXIT: Slot = Slot {
    hook: "crt_exit",
    slot: 0x017E_F9A8,
    module: "msvcr80.dll",
    symbol: c"exit",
};
pub(super) const EXIT_PROCESS: Slot = Slot {
    hook: "exit_process",
    slot: 0x017E_F238,
    module: "kernel32.dll",
    symbol: c"ExitProcess",
};

impl Slot {
    /// The address the loader bound this import to.
    fn resolved(&self) -> Option<usize> {
        let wide: Vec<u16> = self.module.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated strings that outlive the calls.
        unsafe {
            let module = GetModuleHandleW(wide.as_ptr());
            if module.is_null() {
                return None;
            }
            GetProcAddress(module, self.symbol.as_ptr().cast()).map(|f| f as usize)
        }
    }

    /// What the slot holds now.
    fn current(&self) -> Option<usize> {
        let bytes = cimmeria_client_hookgate::os::read_bytes(self.slot, 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?) as usize)
    }
}

static ORIG_MINIDUMP_WRITE_DUMP: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET_UEF: AtomicUsize = AtomicUsize::new(0);
static ORIG_EXIT: AtomicUsize = AtomicUsize::new(0);
static ORIG_EXIT_PROCESS: AtomicUsize = AtomicUsize::new(0);

static CHAIN: FilterChain = FilterChain::new();

type MiniDumpWriteDumpFn = unsafe extern "stdcall-unwind" fn(
    HANDLE,
    u32,
    HANDLE,
    i32,
    *const MINIDUMP_EXCEPTION_INFORMATION,
    *const c_void,
    *const c_void,
) -> BOOL;
type TopFilterFn = unsafe extern "system" fn(*const EXCEPTION_POINTERS) -> i32;

pub(super) fn install(producer: &Producer) {
    // Our filter first, over whatever is current (the CRT's C++
    // filter once the CRT has started).
    // SAFETY: a valid filter pointer; the call only swaps a global.
    let displaced = unsafe { SetUnhandledExceptionFilter(Some(top_filter)) };
    CHAIN.installed_over(displaced.map_or(0, |f| f as usize));

    let mut swapped = 0u32;
    for (slot, detour, orig) in [
        (
            MINIDUMP_WRITE_DUMP,
            minidump_write_dump_detour as *const c_void as usize,
            &ORIG_MINIDUMP_WRITE_DUMP,
        ),
        (
            SET_UNHANDLED_EXCEPTION_FILTER,
            set_uef_detour as *const c_void as usize,
            &ORIG_SET_UEF,
        ),
        (CRT_EXIT, exit_detour as *const c_void as usize, &ORIG_EXIT),
        (
            EXIT_PROCESS,
            exit_process_detour as *const c_void as usize,
            &ORIG_EXIT_PROCESS,
        ),
    ] {
        // SAFETY: each detour's ABI matches its import (see the fns).
        if unsafe { swap_slot(producer, slot, detour, orig) } {
            swapped += 1;
        }
    }

    // A game call that raced the IAT swap went to the real API and
    // replaced ours; put ours back on top of it.
    // SAFETY: as above.
    let current = unsafe { SetUnhandledExceptionFilter(Some(top_filter)) };
    let ours = top_filter as TopFilterFn as usize;
    let current = current.map_or(0, |f| f as usize);
    if current != ours {
        CHAIN.installed_over(current);
    }

    crate::hooks::emit_info(
        producer,
        "client.crash.installed",
        [
            ("iat_hooks", serde_json::json!(swapped)),
            (
                "chained_filter",
                serde_json::Value::String(format!("0x{:08x}", CHAIN.next())),
            ),
        ],
    );
}

/// Swap one slot after checking it holds exactly its resolved import.
/// Reports under the same targets as the other IAT hooks.
///
/// # Safety
///
/// `detour` must have the import's exact calling convention.
unsafe fn swap_slot(producer: &Producer, slot: Slot, detour: usize, orig: &AtomicUsize) -> bool {
    let address = || serde_json::Value::String(format!("0x{:08x}", slot.slot));
    let (expected, current) = (slot.resolved(), slot.current());
    let Some(original) = expected.filter(|e| Some(*e) == current) else {
        let show = |v: Option<usize>| {
            serde_json::Value::String(v.map_or("none".into(), |v| format!("0x{v:08x}")))
        };
        crate::hooks::emit_warn(
            producer,
            "client.hooks.iat.slot_mismatch",
            [
                ("hook", serde_json::json!(slot.hook)),
                ("address", address()),
                ("expected", show(expected)),
                ("actual", show(current)),
            ],
        );
        return false;
    };
    // Publish the original first: a call can reach the detour the
    // moment the slot is written.
    orig.store(original, Ordering::Release);
    // SAFETY: the slot was just read as holding the resolved import, so
    // it is a live IAT slot of this process.
    match unsafe { crate::hooks::replace_iat_slot(slot.slot, detour) } {
        Ok(displaced) => {
            orig.store(displaced, Ordering::Release);
            crate::hooks::emit_info(
                producer,
                "client.hooks.iat.installed",
                [
                    ("hook", serde_json::json!(slot.hook)),
                    ("address", address()),
                    (
                        "original",
                        serde_json::Value::String(format!("0x{displaced:08x}")),
                    ),
                ],
            );
            true
        }
        Err(_) => {
            orig.store(0, Ordering::Release);
            crate::hooks::emit_warn(
                producer,
                "client.hooks.iat.protect_failed",
                [
                    ("hook", serde_json::json!(slot.hook)),
                    ("address", address()),
                ],
            );
            false
        }
    }
}

// ─── Detours ────────────────────────────────────────────────────

/// `BOOL MiniDumpWriteDump(HANDLE, DWORD, HANDLE, MINIDUMP_TYPE,
/// PMINIDUMP_EXCEPTION_INFORMATION, PMINIDUMP_USER_STREAM_INFORMATION,
/// PMINIDUMP_CALLBACK_INFORMATION)` — `__stdcall`. The game only calls
/// it from its crash handler, inside an exception filter, so the
/// exception pointers are live.
unsafe extern "stdcall-unwind" fn minidump_write_dump_detour(
    process: HANDLE,
    pid: u32,
    file: HANDLE,
    dump_type: i32,
    exception: *const MINIDUMP_EXCEPTION_INFORMATION,
    user_streams: *const c_void,
    callback: *const c_void,
) -> BOOL {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: GetCurrentProcessId has no preconditions.
        if exception.is_null() || pid != unsafe { GetCurrentProcessId() } {
            return;
        }
        // SAFETY: the game passes a pointer to its own local; the struct
        // is packed on x86, hence the unaligned read.
        let info = unsafe { exception.read_unaligned() };
        if info.ExceptionPointers.is_null() {
            return;
        }
        // SAFETY: live exception pointers from the game's filter.
        unsafe {
            capture(
                CrashSource::GameMinidump,
                info.ExceptionPointers,
                Some(dump_type as u32),
            )
        };
    }));

    let original = original_minidump_write_dump();
    match original {
        // SAFETY: the loader's own MiniDumpWriteDump, same signature.
        Some(f) => unsafe {
            f(
                process,
                pid,
                file,
                dump_type,
                exception,
                user_streams,
                callback,
            )
        },
        None => 0,
    }
}

/// `LPTOP_LEVEL_EXCEPTION_FILTER SetUnhandledExceptionFilter(
/// LPTOP_LEVEL_EXCEPTION_FILTER)` — `__stdcall`. Game calls land here:
/// the game's filter goes under ours, and the game gets back the
/// filter it would have got from the real API.
unsafe extern "stdcall-unwind" fn set_uef_detour(filter: usize) -> usize {
    CHAIN.game_sets(filter)
}

/// `void exit(int)` from `MSVCR80` — `__cdecl`. `WinMain` returning ends
/// here; so do the game's fatal-error `exit(1)` calls.
unsafe extern "C-unwind" fn exit_detour(code: i32) {
    let _ = catch_unwind(|| {
        // SAFETY: no preconditions.
        let tid = unsafe { GetCurrentThreadId() };
        STATE.on_exit(ExitPath::CrtExit, code as u32, tid);
    });
    let orig = ORIG_EXIT.load(Ordering::Acquire);
    if orig != 0 {
        // SAFETY: the loader's own `exit`, same signature.
        let original: unsafe extern "C-unwind" fn(i32) = unsafe { std::mem::transmute(orig) };
        unsafe { original(code) };
    }
    // `exit` does not return; if the original somehow did, leave the
    // way it would have.
    // SAFETY: terminates this process.
    unsafe { windows_sys::Win32::System::Threading::ExitProcess(code as u32) }
}

/// `void ExitProcess(UINT)` — `__stdcall`.
unsafe extern "stdcall-unwind" fn exit_process_detour(code: u32) {
    let _ = catch_unwind(|| {
        // SAFETY: no preconditions.
        let tid = unsafe { GetCurrentThreadId() };
        STATE.on_exit(ExitPath::ExitProcess, code, tid);
    });
    let orig = ORIG_EXIT_PROCESS.load(Ordering::Acquire);
    if orig != 0 {
        // SAFETY: the loader's own ExitProcess, same signature.
        let original: unsafe extern "stdcall-unwind" fn(u32) = unsafe { std::mem::transmute(orig) };
        unsafe { original(code) };
    }
    // SAFETY: terminates this process.
    unsafe { windows_sys::Win32::System::Threading::ExitProcess(code) }
}

/// Our top-level filter: record, then hand the exception to the filter
/// the game (or the CRT) set, and return its verdict.
unsafe extern "system" fn top_filter(info: *const EXCEPTION_POINTERS) -> i32 {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !info.is_null() {
            // SAFETY: the OS passes live exception pointers.
            unsafe { capture(CrashSource::UnhandledFilter, info, None) };
        }
    }));
    let next = CHAIN.next();
    let ours = top_filter as TopFilterFn as usize;
    let verdict = if next == 0 || next == ours {
        None
    } else {
        // SAFETY: `next` came from SetUnhandledExceptionFilter, whose
        // argument type is exactly this.
        let next: TopFilterFn = unsafe { std::mem::transmute(next) };
        Some(unsafe { next(info) })
    };
    filter_result(verdict)
}

// ─── Capture ────────────────────────────────────────────────────

/// # Safety
///
/// `info` must be live exception pointers for this process.
unsafe fn capture(source: CrashSource, info: *const EXCEPTION_POINTERS, dump_type: Option<u32>) {
    // SAFETY: forwarded.
    let fault = unsafe { read_fault(info) };
    STATE.on_crash(source, fault, dump_type, now_ms(), |path| {
        // SAFETY: forwarded.
        unsafe { write_dump(path, info) }
    });
}

/// # Safety
///
/// `info` must be null or live exception pointers.
unsafe fn read_fault(info: *const EXCEPTION_POINTERS) -> Fault {
    // SAFETY: no preconditions.
    let thread_id = unsafe { GetCurrentThreadId() };
    let mut fault = Fault {
        code: 0,
        flags: 0,
        address: 0,
        params: [None, None],
        module: None,
        thread_id,
    };
    // SAFETY: caller contract; each pointer is checked before use.
    let rec = if info.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { (*info).ExceptionRecord }
    };
    if rec.is_null() {
        return fault;
    }
    // SAFETY: a live EXCEPTION_RECORD.
    let rec = unsafe { &*rec };
    fault.code = rec.ExceptionCode as u32;
    fault.flags = rec.ExceptionFlags;
    fault.address = rec.ExceptionAddress as usize as u32;
    let n = (rec.NumberParameters as usize).min(rec.ExceptionInformation.len());
    for (i, p) in fault.params.iter_mut().enumerate() {
        if i < n {
            *p = Some(rec.ExceptionInformation[i] as u32);
        }
    }
    fault.module = module_of(fault.address as usize);
    fault
}

/// The module whose image contains `address`: its file name and base.
pub(super) fn module_of(address: usize) -> Option<FaultModule> {
    if address == 0 {
        return None;
    }
    let mut module: HMODULE = std::ptr::null_mut();
    // SAFETY: FROM_ADDRESS makes the "name" argument an address; the
    // refcount is left alone, so nothing needs releasing.
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            address as *const u16,
            &mut module,
        )
    };
    if ok == 0 || module.is_null() {
        return None;
    }
    let mut buf = [0u16; MAX_PATH as usize];
    // SAFETY: `buf` is writable for its length.
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    let path = String::from_utf16_lossy(&buf[..len.min(buf.len())]);
    Some(FaultModule {
        name: module_basename(&path).to_string(),
        base: module as usize as u32,
    })
}

/// The real `MiniDumpWriteDump`: the one the IAT held, else the loaded
/// `dbghelp.dll`'s export (SGW.exe loads its own 6.3 copy at start-up),
/// else our own import, which makes the loader bring in a `dbghelp.dll`
/// (only the tests get that far).
fn original_minidump_write_dump() -> Option<MiniDumpWriteDumpFn> {
    let mut addr = ORIG_MINIDUMP_WRITE_DUMP.load(Ordering::Acquire);
    if addr == 0 {
        addr = MINIDUMP_WRITE_DUMP.resolved().unwrap_or(
            windows_sys::Win32::System::Diagnostics::Debug::MiniDumpWriteDump as *const () as usize,
        );
    }
    // SAFETY: a MiniDumpWriteDump entry point, same signature.
    Some(unsafe { std::mem::transmute::<usize, MiniDumpWriteDumpFn>(addr) })
}

/// Write our minidump to `path`. `info` may be null (a dump with no
/// exception stream; only the tests do that).
///
/// # Safety
///
/// `info` must be null or live exception pointers.
pub(super) unsafe fn write_dump(path: &Path, info: *const EXCEPTION_POINTERS) -> DumpOutcome {
    let Some(write) = original_minidump_write_dump() else {
        return DumpOutcome::Failed {
            stage: "dbghelp_missing",
            os_error: 0,
        };
    };
    let mut last_error = 0;
    for dump_type in [DUMP_TYPE, FALLBACK_DUMP_TYPE] {
        let file = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(e) => {
                return DumpOutcome::Failed {
                    stage: "create_file",
                    os_error: e.raw_os_error().unwrap_or(0) as u32,
                }
            }
        };
        let exception = MINIDUMP_EXCEPTION_INFORMATION {
            // SAFETY: no preconditions.
            ThreadId: unsafe { GetCurrentThreadId() },
            ExceptionPointers: info as *mut EXCEPTION_POINTERS,
            // The pointers are in this process's address space.
            ClientPointers: 0,
        };
        let exception_ptr = if info.is_null() {
            std::ptr::null()
        } else {
            &exception as *const MINIDUMP_EXCEPTION_INFORMATION
        };
        // SAFETY: valid process and file handles; `exception` outlives
        // the call.
        let ok = unsafe {
            write(
                GetCurrentProcess(),
                GetCurrentProcessId(),
                file.as_raw_handle() as HANDLE,
                dump_type as i32,
                exception_ptr,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if ok != 0 {
            let bytes = file.metadata().map_or(0, |m| m.len());
            return DumpOutcome::Written { bytes, dump_type };
        }
        // SAFETY: no preconditions.
        last_error = unsafe { GetLastError() };
    }
    let _ = std::fs::remove_file(path);
    DumpOutcome::Failed {
        stage: "minidump_write",
        os_error: last_error,
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slot addresses are pointer-aligned entries of SGW.exe's IAT
    /// (`.idata` runs 0x017EF000..0x017F1000 in the QA build). The
    /// pre-2026-09-27 anchors were odd hint/name RVAs; this catches a
    /// regression to that shape.
    #[test]
    fn slots_are_aligned_iat_entries() {
        for s in [
            MINIDUMP_WRITE_DUMP,
            SET_UNHANDLED_EXCEPTION_FILTER,
            CRT_EXIT,
            EXIT_PROCESS,
        ] {
            assert_eq!(s.slot % 4, 0, "{}", s.hook);
            assert!((0x017E_F000..0x017F_1000).contains(&s.slot), "{}", s.hook);
        }
    }

    #[test]
    fn module_of_names_the_test_binary() {
        let here = module_of_names_the_test_binary as *const () as usize;
        let m = module_of(here).expect("our own code is in a module");
        assert!(m.name.ends_with(".exe"), "{}", m.name);
        assert!(!m.name.contains('\\'));
        assert!(here as u32 > m.base);
        assert_eq!(module_of(0), None);
        assert_eq!(module_of(0x10), None);
    }

    /// A real dump of this (test) process through the same call the
    /// crash path makes: it is written, starts with the minidump
    /// signature, and is small.
    #[test]
    fn writes_a_small_minidump_of_this_process() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.dmp");
        // SAFETY: null exception pointers are allowed.
        let outcome = unsafe { write_dump(&path, std::ptr::null()) };
        let DumpOutcome::Written { bytes, dump_type } = outcome else {
            panic!("dump not written: {outcome:?}");
        };
        assert_eq!(dump_type, DUMP_TYPE);
        let data = std::fs::read(&path).unwrap();
        assert_eq!(data.len() as u64, bytes);
        assert_eq!(&data[..4], b"MDMP");
        assert!(bytes < 16 * 1024 * 1024, "{bytes} bytes");
    }
}
