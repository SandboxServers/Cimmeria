//! IAT (Import Address Table) hooks.
//!
//! Patches the IAT slot for a specific imported function so the
//! address SGW.exe calls through resolves to OUR detour instead
//! of the original library entry. The detour calls the original
//! via the saved address.
//!
//! # Hook surface (this module)
//!
//! Phase 4 (Lua / scripted UI):
//! - `lua_pcall` @ IAT `0x01988A0C` — most common Lua dispatch
//! - `lua_call`  @ IAT `0x01988904` — unprotected Lua call
//! - `lua_newstate` @ IAT `0x01988656` — Lua state creation
//!
//! Phase 5 (OS correlators):
//! - `CreateThread`       @ IAT `0x0196B65A` (KERNEL32) — thread timeline
//! - `LoadLibraryW`       @ IAT `0x0196B5BC` (KERNEL32) — module timeline
//! - `LoadLibraryA`       @ IAT `0x0196B5AC` (KERNEL32) — module timeline (ANSI)
//! - `GetForegroundWindow`@ IAT `0x0196AF20` (USER32)   — focus correlation
//!
//! # Technique
//!
//! IAT slots live in the `.idata` section. They are 4-byte function
//! pointers that the loader populates when a DLL is loaded. To swap
//! one:
//!
//! 1. `VirtualProtect` the slot to `PAGE_READWRITE` (it's `PAGE_READONLY`
//!    after loader fixup).
//! 2. Atomically replace the slot value with our detour address.
//! 3. Restore the original page protection.
//! 4. Remember the original address so the detour can chain to it.
//!
//! No code-byte patching, no trampolines — IAT hooks are the
//! lightest-weight inline-hook alternative. The cost is that the
//! hook only intercepts callers that go through the IAT (not direct
//! `call <addr>` to the library entry from SGW.exe code that
//! resolved the address some other way).
//!
//! # Safety
//!
//! The detour MUST match the imported function's calling convention
//! EXACTLY or the stack will be corrupted on call/return. Windows
//! APIs are `__stdcall` (callee cleans the stack); the Lua C API is
//! `__cdecl` (caller cleans). Mismatching either way is undefined
//! behavior and crashes the game.

#![allow(clippy::missing_safety_doc)] // FFI bindings — safety docs at fn-level

use crate::queue::Producer;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::atomic::{AtomicUsize, Ordering};

// ─── IAT slot addresses (2026-06-04 manifest) ───────────────────

#[cfg(all(target_os = "windows", target_arch = "x86"))]
const IAT_LUA_PCALL: usize = 0x01988A0C;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const IAT_LUA_CALL: usize = 0x01988904;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const IAT_LUA_NEWSTATE: usize = 0x01988656;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
const IAT_CREATE_THREAD: usize = 0x0196B65A;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const IAT_LOAD_LIBRARY_W: usize = 0x0196B5BC;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const IAT_LOAD_LIBRARY_A: usize = 0x0196B5AC;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const IAT_GET_FOREGROUND_WINDOW: usize = 0x0196AF20;

// ─── Saved originals (set at install time, used in detours) ─────
//
// `AtomicUsize::new(0)` is sentinel "not yet installed". After
// install_one swaps the IAT slot, we store the original here so
// the detour can chain through. Atomic so the detour (running on
// any thread) sees the value without explicit ordering.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_LUA_PCALL: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_LUA_CALL: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_LUA_NEWSTATE: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_CREATE_THREAD: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_LOAD_LIBRARY_W: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_LOAD_LIBRARY_A: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_GET_FOREGROUND_WINDOW: AtomicUsize = AtomicUsize::new(0);

/// Install every IAT hook. Best-effort: each slot swap reports its
/// own success/failure event; one failure doesn't block the others.
pub fn install(_producer: Producer) {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        install_inner(_producer);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_inner(producer: Producer) {
    install_one(
        &producer,
        "lua_pcall",
        IAT_LUA_PCALL,
        lua_pcall_detour as *const c_void as usize,
        &ORIG_LUA_PCALL,
    );
    install_one(
        &producer,
        "lua_call",
        IAT_LUA_CALL,
        lua_call_detour as *const c_void as usize,
        &ORIG_LUA_CALL,
    );
    install_one(
        &producer,
        "lua_newstate",
        IAT_LUA_NEWSTATE,
        lua_newstate_detour as *const c_void as usize,
        &ORIG_LUA_NEWSTATE,
    );
    install_one(
        &producer,
        "create_thread",
        IAT_CREATE_THREAD,
        create_thread_detour as *const c_void as usize,
        &ORIG_CREATE_THREAD,
    );
    install_one(
        &producer,
        "load_library_w",
        IAT_LOAD_LIBRARY_W,
        load_library_w_detour as *const c_void as usize,
        &ORIG_LOAD_LIBRARY_W,
    );
    install_one(
        &producer,
        "load_library_a",
        IAT_LOAD_LIBRARY_A,
        load_library_a_detour as *const c_void as usize,
        &ORIG_LOAD_LIBRARY_A,
    );
    install_one(
        &producer,
        "get_foreground_window",
        IAT_GET_FOREGROUND_WINDOW,
        get_foreground_window_detour as *const c_void as usize,
        &ORIG_GET_FOREGROUND_WINDOW,
    );

    super::emit_info(
        &producer,
        "client.hooks.iat.install_complete",
        [("hook_count", serde_json::json!(7))],
    );
}

/// Generic IAT slot swap. Writes `detour` into the 4-byte slot at
/// `iat_slot_addr`, saves the previously-stored address in
/// `orig_slot` for the detour to chain through. Toggles
/// `VirtualProtect` to `PAGE_READWRITE` for the swap and restores
/// the original protection afterward.
///
/// # Safety
///
/// `iat_slot_addr` must point to a valid IAT slot owned by SGW.exe
/// (in the `.idata` section). The `detour` calling convention must
/// match the imported function's ABI exactly.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_one(
    producer: &Producer,
    hook_name: &'static str,
    iat_slot_addr: usize,
    detour: usize,
    orig_slot: &AtomicUsize,
) {
    use windows_sys::Win32::System::Memory::{
        VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE,
    };

    let slot_ptr = iat_slot_addr as *mut usize;
    let mut old_protect: PAGE_PROTECTION_FLAGS = 0;

    // Step 1: open the page for writing.
    let ok = VirtualProtect(
        slot_ptr as *const c_void,
        std::mem::size_of::<usize>(),
        PAGE_READWRITE,
        &mut old_protect,
    );
    if ok == 0 {
        super::emit_warn(
            producer,
            "client.hooks.iat.protect_failed",
            [
                ("hook", serde_json::Value::String(hook_name.into())),
                (
                    "address",
                    serde_json::Value::String(format!("0x{iat_slot_addr:08x}")),
                ),
            ],
        );
        return;
    }

    // Step 2: read original + write detour. The IAT slot is one
    // dword; non-atomic write is fine because the page is single-
    // threaded at this point (we're still in DLL bootstrap, no
    // other code is calling through the slot concurrently).
    let original = *slot_ptr;
    *slot_ptr = detour;
    orig_slot.store(original, Ordering::Release);

    // Step 3: restore the original page protection.
    let mut _unused: PAGE_PROTECTION_FLAGS = 0;
    let _ = VirtualProtect(
        slot_ptr as *const c_void,
        std::mem::size_of::<usize>(),
        old_protect,
        &mut _unused,
    );

    super::emit_info(
        producer,
        "client.hooks.iat.installed",
        [
            ("hook", serde_json::Value::String(hook_name.into())),
            (
                "address",
                serde_json::Value::String(format!("0x{iat_slot_addr:08x}")),
            ),
            (
                "original",
                serde_json::Value::String(format!("0x{original:08x}")),
            ),
        ],
    );
}

// ─── Sampling ──────────────────────────────────────────────────

/// `CreateThread` is rare enough (a few per session) that 1/1 emit
/// is fine. Same for `LoadLibraryW/A` and `GetForegroundWindow`
/// (well, focus checks fire often but the typical observability
/// budget is small).
///
/// Lua calls fire on every UI script — bound them.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static LUA_PCALL_SAMPLER: super::sampling::SamplingCounter =
    super::sampling::SamplingCounter::new(10);
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static LUA_CALL_SAMPLER: super::sampling::SamplingCounter =
    super::sampling::SamplingCounter::new(10);

/// `GetForegroundWindow` is polled by some game code every frame.
/// Sample heavily — we only care about transitions, not absolute
/// frequency.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static GET_FOREGROUND_WINDOW_SAMPLER: super::sampling::SamplingCounter =
    super::sampling::SamplingCounter::new(1000);

// ─── Detours ────────────────────────────────────────────────────
//
// IMPORTANT calling convention rules:
//
// - Lua C API: `__cdecl` (returns int, args via stack, caller cleans).
// - Win32 API: `__stdcall` (callee cleans the stack).
//
// We must declare each detour with the EXACT matching convention
// or the stack frame corrupts on call/return.

/// `lua_pcall(lua_State* L, int nargs, int nresults, int errfunc) -> int`
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn lua_pcall_detour(
    l: *mut c_void,
    nargs: i32,
    nresults: i32,
    errfunc: i32,
) -> i32 {
    let _ = std::panic::catch_unwind(|| {
        if LUA_PCALL_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.lua.pcall", "debug")
                        .field("nargs", serde_json::json!(nargs))
                        .field("nresults", serde_json::json!(nresults)),
                );
            }
        }
    });

    let orig_addr = ORIG_LUA_PCALL.load(Ordering::Acquire);
    if orig_addr == 0 {
        // Not yet installed — should be unreachable since the slot
        // wouldn't be pointing at us. Return error-ish value (Lua
        // errors are 1-5; -1 is invalid but safe).
        return -1;
    }
    let original: unsafe extern "C" fn(*mut c_void, i32, i32, i32) -> i32 =
        unsafe { std::mem::transmute(orig_addr) };
    original(l, nargs, nresults, errfunc)
}

/// `lua_call(lua_State* L, int nargs, int nresults) -> void`
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn lua_call_detour(l: *mut c_void, nargs: i32, nresults: i32) {
    let _ = std::panic::catch_unwind(|| {
        if LUA_CALL_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.lua.call", "debug")
                        .field("nargs", serde_json::json!(nargs))
                        .field("nresults", serde_json::json!(nresults)),
                );
            }
        }
    });

    let orig_addr = ORIG_LUA_CALL.load(Ordering::Acquire);
    if orig_addr == 0 {
        return;
    }
    let original: unsafe extern "C" fn(*mut c_void, i32, i32) =
        unsafe { std::mem::transmute(orig_addr) };
    original(l, nargs, nresults);
}

/// `lua_newstate(lua_Alloc f, void* ud) -> lua_State*`
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn lua_newstate_detour(f: *mut c_void, ud: *mut c_void) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(crate::events::ClientNativeEvent::builder(
                "client.lua.newstate",
                "info",
            ));
        }
    });

    let orig_addr = ORIG_LUA_NEWSTATE.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(f, ud)
}

/// `HANDLE CreateThread(LPSECURITY_ATTRIBUTES, SIZE_T stack_size,
///   LPTHREAD_START_ROUTINE start, LPVOID parameter, DWORD flags,
///   LPDWORD out_thread_id)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall" fn create_thread_detour(
    sec_attrs: *mut c_void,
    stack_size: usize,
    start_routine: *mut c_void,
    parameter: *mut c_void,
    flags: u32,
    out_thread_id: *mut u32,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.os.create_thread", "info")
                    .field("stack_size", serde_json::json!(stack_size))
                    .field("flags", serde_json::json!(flags)),
            );
        }
    });

    let orig_addr = ORIG_CREATE_THREAD.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall" fn(
        *mut c_void,
        usize,
        *mut c_void,
        *mut c_void,
        u32,
        *mut u32,
    ) -> *mut c_void = unsafe { std::mem::transmute(orig_addr) };
    original(
        sec_attrs,
        stack_size,
        start_routine,
        parameter,
        flags,
        out_thread_id,
    )
}

/// `HMODULE LoadLibraryW(LPCWSTR lib_filename)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall" fn load_library_w_detour(lib_filename: *const u16) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            let name = super::inline_hooks::read_utf16_bounded(lib_filename, 256);
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.os.load_library", "info")
                    .field("encoding", serde_json::json!("wide"))
                    .field("name", serde_json::Value::String(name)),
            );
        }
    });

    let orig_addr = ORIG_LOAD_LIBRARY_W.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall" fn(*const u16) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(lib_filename)
}

/// `HMODULE LoadLibraryA(LPCSTR lib_filename)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall" fn load_library_a_detour(lib_filename: *const u8) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            let name = read_ascii_bounded(lib_filename, 256);
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.os.load_library", "info")
                    .field("encoding", serde_json::json!("ansi"))
                    .field("name", serde_json::Value::String(name)),
            );
        }
    });

    let orig_addr = ORIG_LOAD_LIBRARY_A.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall" fn(*const u8) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(lib_filename)
}

/// `HWND GetForegroundWindow(void)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall" fn get_foreground_window_detour() -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if GET_FOREGROUND_WINDOW_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(crate::events::ClientNativeEvent::builder(
                    "client.os.get_foreground_window",
                    "debug",
                ));
            }
        }
    });

    let orig_addr = ORIG_GET_FOREGROUND_WINDOW.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall" fn() -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original()
}

/// ASCII-bounded string read for `LoadLibraryA` — same safety
/// contract as `inline_hooks::read_utf16_bounded` but for narrow
/// strings.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn read_ascii_bounded(ptr: *const u8, max_chars: usize) -> String {
    if ptr.is_null() {
        return "<null>".into();
    }
    let mut len = 0usize;
    while len < max_chars {
        // SAFETY: bounded by max_chars; caller's buffer is the
        // path passed to LoadLibraryA which is always NUL-terminated.
        let ch = unsafe { *ptr.add(len) };
        if ch == 0 {
            break;
        }
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}

#[cfg(test)]
mod tests {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    use super::*;

    /// IAT slot pin — bumps tests if any address shifts.
    #[test]
    fn iat_slots_match_manifest() {
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        {
            assert_eq!(IAT_LUA_PCALL, 0x01988A0C);
            assert_eq!(IAT_LUA_CALL, 0x01988904);
            assert_eq!(IAT_LUA_NEWSTATE, 0x01988656);
            assert_eq!(IAT_CREATE_THREAD, 0x0196B65A);
            assert_eq!(IAT_LOAD_LIBRARY_W, 0x0196B5BC);
            assert_eq!(IAT_LOAD_LIBRARY_A, 0x0196B5AC);
            assert_eq!(IAT_GET_FOREGROUND_WINDOW, 0x0196AF20);
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
}
