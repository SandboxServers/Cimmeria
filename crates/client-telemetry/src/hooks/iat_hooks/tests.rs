//! Unit tests for the IAT hooks.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use super::*;

/// IAT slot pin — bumps tests if any address shifts.
#[test]
fn iat_slots_match_manifest() {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        // From the QA SGW.exe import directory (2026-09-27). IAT
        // slots are 4-byte aligned; the old hint/name addresses
        // were not.
        assert_eq!(IAT_LUA_PCALL.slot, 0x017F0228);
        assert_eq!(IAT_LUA_CALL.slot, 0x017F0244);
        assert_eq!(IAT_LUA_NEWSTATE.slot, 0x017F0288);
        assert_eq!(IAT_CREATE_THREAD.slot, 0x017EF290);
        assert_eq!(IAT_LOAD_LIBRARY_W.slot, 0x017EF26C);
        assert_eq!(IAT_LOAD_LIBRARY_A.slot, 0x017EF268);
        assert_eq!(IAT_GET_FOREGROUND_WINDOW.slot, 0x017EFDF8);
        assert_eq!(IAT_RECVFROM.slot, 0x017EFF60);
        for import in [
            IAT_LUA_PCALL,
            IAT_LUA_CALL,
            IAT_LUA_NEWSTATE,
            IAT_CREATE_THREAD,
            IAT_LOAD_LIBRARY_W,
            IAT_LOAD_LIBRARY_A,
            IAT_GET_FOREGROUND_WINDOW,
            IAT_RECVFROM,
        ] {
            assert_eq!(import.slot % 4, 0, "{:?}", import.symbol);
        }
    }
}

/// Sampler rates pin.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn iat_sampler_rates() {
    let pcall_emits: usize = (0..10).filter(|_| LUA_PCALL_SAMPLER.should_emit()).count();
    assert_eq!(pcall_emits, 1, "lua_pcall sampler should emit 1/10");

    let call_emits: usize = (0..10).filter(|_| LUA_CALL_SAMPLER.should_emit()).count();
    assert_eq!(call_emits, 1, "lua_call sampler should emit 1/10");

    let focus_emits: usize = (0..1000)
        .filter(|_| GET_FOREGROUND_WINDOW_SAMPLER.should_emit())
        .count();
    assert_eq!(focus_emits, 1, "focus sampler should emit 1/1000");
}

/// ASCII bounded reader stops at NUL.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn ascii_bounded_stops_at_nul() {
    let s = b"kernel32.dll\0extra";
    assert_eq!(read_ascii_bounded(s.as_ptr(), 256), "kernel32.dll");
}

/// ASCII bounded reader caps at max_chars.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn ascii_bounded_caps_at_max() {
    let s = vec![b'A'; 500];
    assert_eq!(read_ascii_bounded(s.as_ptr(), 16).len(), 16);
}

/// Null pointer → sentinel string, no UB.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn ascii_bounded_null() {
    assert_eq!(read_ascii_bounded(std::ptr::null(), 256), "<null>");
}

// #915: `lua51.dll` raises Lua errors as C++ exceptions. An error in
// a `lua_call` made under an outer `lua_pcall` unwinds through
// `lua_call_detour` to that pcall. A Rust panic stands in for the C++
// exception (on MSVC both are SEH unwinds). With a plain `extern "C"`
// detour the unwind aborts the process at the detour's frame, so
// these tests die instead of passing.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn a_lua_error_unwinds_through_lua_call_detour() {
    unsafe extern "C-unwind" fn throwing_lua_call(_l: *mut c_void, _nargs: i32, _nres: i32) {
        panic!("lua error");
    }
    ORIG_LUA_CALL.store(throwing_lua_call as *const () as usize, Ordering::Release);
    let caught = std::panic::catch_unwind(|| unsafe {
        lua_call_detour(core::ptr::null_mut(), 0, 0);
    });
    assert!(caught.is_err());
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn an_error_from_the_original_unwinds_through_lua_pcall_detour() {
    unsafe extern "C-unwind" fn throwing_lua_pcall(
        _l: *mut c_void,
        _nargs: i32,
        _nres: i32,
        _errfunc: i32,
    ) -> i32 {
        panic!("error handler raised");
    }
    let _serial = PCALL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ORIG_LUA_PCALL.store(throwing_lua_pcall as *const () as usize, Ordering::Release);
    let caught =
        std::panic::catch_unwind(|| unsafe { lua_pcall_detour(core::ptr::null_mut(), 0, 0, 0) });
    assert!(caught.is_err());
}

/// The two tests that swap `ORIG_LUA_PCALL` must not overlap.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static PCALL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A failed `lua_pcall` returns the original's status unchanged and reports
/// the error without touching the stack. `lua51.dll` is not loaded in the
/// test process, so the message reader finds no exports and the event goes
/// out message-less: the point is that nothing faults and nothing changes.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn a_failed_lua_pcall_returns_its_status_and_reports_safely() {
    unsafe extern "C-unwind" fn failing_lua_pcall(
        l: *mut c_void,
        nargs: i32,
        nres: i32,
        errfunc: i32,
    ) -> i32 {
        assert_eq!((l as usize, nargs, nres, errfunc), (0x1234, 2, 3, 4));
        2
    }
    let _serial = PCALL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ORIG_LUA_PCALL.store(failing_lua_pcall as *const () as usize, Ordering::Release);
    let status = unsafe { lua_pcall_detour(0x1234 as *mut c_void, 2, 3, 4) };
    assert_eq!(status, 2);
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn an_error_from_the_original_unwinds_through_lua_newstate_detour() {
    unsafe extern "C-unwind" fn throwing_newstate(
        _f: *mut c_void,
        _ud: *mut c_void,
    ) -> *mut c_void {
        panic!("allocation failed");
    }
    ORIG_LUA_NEWSTATE.store(throwing_newstate as *const () as usize, Ordering::Release);
    let caught = std::panic::catch_unwind(|| unsafe {
        lua_newstate_detour(core::ptr::null_mut(), core::ptr::null_mut())
    });
    assert!(caught.is_err());
}

/// With no original published the detour answers on its own instead
/// of calling address 0.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[test]
fn get_foreground_window_detour_without_an_original_returns_null() {
    assert_eq!(ORIG_GET_FOREGROUND_WINDOW.load(Ordering::Acquire), 0);
    assert!(unsafe { get_foreground_window_detour() }.is_null());
}
