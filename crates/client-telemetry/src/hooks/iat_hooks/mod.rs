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
//! - `lua_pcall` @ IAT `0x017F0228` — most common Lua dispatch; a
//!   non-zero return reports the error string as `client.lua.error`
//!   ([`lua_error`])
//! - `lua_call`  @ IAT `0x017F0244` — unprotected Lua call
//! - `lua_newstate` @ IAT `0x017F0288` — Lua state creation
//!
//! Phase 5 (OS correlators):
//! - `CreateThread`       @ IAT `0x017EF290` (KERNEL32) — thread timeline
//! - `LoadLibraryW`       @ IAT `0x017EF26C` (KERNEL32) — module timeline
//! - `LoadLibraryA`       @ IAT `0x017EF268` (KERNEL32) — module timeline (ANSI)
//! - `GetForegroundWindow`@ IAT `0x017EFDF8` (USER32)   — focus correlation
//!
//! The slot addresses come from the import directory of the QA
//! `SGW.exe`. The addresses used before 2026-09-27 (`0x01988A0C`,
//! `0x0196B65A`, ...) were the on-disk *contents* of these slots: RVAs
//! of the hint/name entries, read as if they were VAs. They pointed into
//! UTF-16 strings in `.rdata`, so every install would have written a
//! detour pointer over string data.
//!
//! Each slot is checked before it is swapped: it must hold exactly the
//! address its import resolves to (`GetProcAddress` on the loaded
//! module). Anything else (a different build, another IAT hook, a
//! delay-load stub) skips that one hook with a `slot_mismatch` warning.
//!
//! `lua51.dll` raises Lua errors as C++ exceptions (it imports
//! `_CxxThrowException`), so an error inside `lua_call` unwinds through
//! `lua_call_detour` to the nearest `lua_pcall`. Every detour here uses
//! an `-unwind` ABI so that unwind reaches its handler instead of
//! aborting the process. The Win32 APIs do not throw, but they use
//! `stdcall-unwind` too: it costs nothing, and one rule is easier to
//! review than a list of exceptions.
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

mod imports;
// Only the i686 `lua_pcall` detour calls its reader; the pure parts are
// unit-tested everywhere.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
mod lua_error;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::hooks::ability_trace::shown;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use imports::*;

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
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_RECVFROM: AtomicUsize = AtomicUsize::new(0);

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
    install_one(
        &producer,
        "recvfrom",
        IAT_RECVFROM,
        recvfrom_detour as *const c_void as usize,
        &ORIG_RECVFROM,
    );

    super::emit_info(
        &producer,
        "client.hooks.iat.install_complete",
        [("hook_count", serde_json::json!(8))],
    );
}

/// Install one IAT hook: swap the slot via
/// [`primitives::replace_iat_slot`](super::primitives::replace_iat_slot)
/// (which owns the `VirtualProtect` → write → restore dance), stash
/// the displaced original in `orig_slot` for the detour to chain
/// through, and emit the per-hook install/failure event.
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
    import: Import,
    detour: usize,
    orig_slot: &AtomicUsize,
) {
    let iat_slot_addr = import.slot;
    let (expected, current) = (import.resolved(), import.current());
    let Some(original) = expected.filter(|e| Some(*e) == current) else {
        let show = |v: Option<usize>| {
            serde_json::Value::String(v.map_or("none".into(), |v| format!("0x{v:08x}")))
        };
        super::emit_warn(
            producer,
            "client.hooks.iat.slot_mismatch",
            [
                ("hook", serde_json::Value::String(hook_name.into())),
                (
                    "address",
                    serde_json::Value::String(format!("0x{iat_slot_addr:08x}")),
                ),
                ("expected", show(expected)),
                ("actual", show(current)),
            ],
        );
        return;
    };
    // Publish the original before the swap: a call through the slot can
    // reach the detour the moment it is written.
    orig_slot.store(original, Ordering::Release);
    // Delegate the protect → swap → restore mechanics to the shared
    // primitive. On failure it returns `ProtectFailed`, which we map
    // to the same `protect_failed` event this hook always emitted.
    let original = match super::primitives::replace_iat_slot(iat_slot_addr, detour) {
        Ok(orig) => orig,
        Err(_) => {
            orig_slot.store(0, Ordering::Release);
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
    };

    // Publish the displaced original so the detour can chain through.
    // `Release` pairs with the detour's `Acquire` load.
    orig_slot.store(original, Ordering::Release);

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
unsafe extern "C-unwind" fn lua_pcall_detour(
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
    let original: unsafe extern "C-unwind" fn(*mut c_void, i32, i32, i32) -> i32 =
        unsafe { std::mem::transmute(orig_addr) };
    // An ability UI handler (`client.ability.shown`): named, and its
    // arguments read, before the call consumes them.
    let shown = std::panic::catch_unwind(|| shown::before_call(l, nargs))
        .ok()
        .flatten();
    let status = original(l, nargs, nresults, errfunc);
    if let Some(p) = shown {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            shown::after_call(p, Some(status))
        }));
    }
    if status != 0 {
        // The error value is on top of the stack. Read it after the call
        // has returned; the stack is left as the caller expects it.
        let _ = std::panic::catch_unwind(|| lua_error::report(l, status, nargs));
    }
    status
}

/// `lua_call(lua_State* L, int nargs, int nresults) -> void`
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C-unwind" fn lua_call_detour(l: *mut c_void, nargs: i32, nresults: i32) {
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
    let original: unsafe extern "C-unwind" fn(*mut c_void, i32, i32) =
        unsafe { std::mem::transmute(orig_addr) };
    let shown = std::panic::catch_unwind(|| shown::before_call(l, nargs))
        .ok()
        .flatten();
    original(l, nargs, nresults);
    // Reached only when the call returned; a Lua error unwinds past it and
    // is reported by the enclosing `lua_pcall` as `client.lua.error`.
    if let Some(p) = shown {
        let _ =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| shown::after_call(p, None)));
    }
}

/// `lua_newstate(lua_Alloc f, void* ud) -> lua_State*`
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C-unwind" fn lua_newstate_detour(f: *mut c_void, ud: *mut c_void) -> *mut c_void {
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
    let original: unsafe extern "C-unwind" fn(*mut c_void, *mut c_void) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(f, ud)
}

/// `HANDLE CreateThread(LPSECURITY_ATTRIBUTES, SIZE_T stack_size,
///   LPTHREAD_START_ROUTINE start, LPVOID parameter, DWORD flags,
///   LPDWORD out_thread_id)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn create_thread_detour(
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
    let original: unsafe extern "stdcall-unwind" fn(
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
unsafe extern "stdcall-unwind" fn load_library_w_detour(lib_filename: *const u16) -> *mut c_void {
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
    let original: unsafe extern "stdcall-unwind" fn(*const u16) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(lib_filename)
}

/// `HMODULE LoadLibraryA(LPCSTR lib_filename)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn load_library_a_detour(lib_filename: *const u8) -> *mut c_void {
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
    let original: unsafe extern "stdcall-unwind" fn(*const u8) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(lib_filename)
}

/// `HWND GetForegroundWindow(void)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn get_foreground_window_detour() -> *mut c_void {
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

    // Lab virtual focus: report the game window as foreground so a lab
    // client in the background keeps processing the injected input.
    #[cfg(feature = "lab-bridge")]
    if let Some(hwnd) = crate::bridge::input::virtual_focus_hwnd() {
        return hwnd as *mut c_void;
    }
    let orig_addr = ORIG_GET_FOREGROUND_WINDOW.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall-unwind" fn() -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original()
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[link(name = "Ws2_32")]
unsafe extern "system" {
    fn WSAGetLastError() -> i32;
    fn WSASetLastError(error: i32);
}

/// `recvfrom(SOCKET, char*, int, int, sockaddr*, int*) -> int` — the
/// Winsock boundary. The original runs first, before we inspect the raw
/// encrypted bytes or the socket's failure code. Restore the thread's WSA
/// error after telemetry so the client observes exactly the original result.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn recvfrom_detour(
    socket: usize,
    buf: *mut u8,
    len: i32,
    flags: i32,
    from: *mut c_void,
    from_len: *mut i32,
) -> i32 {
    let orig_addr = ORIG_RECVFROM.load(Ordering::Acquire);
    if orig_addr == 0 {
        return -1;
    }
    let original: unsafe extern "stdcall-unwind" fn(
        usize,
        *mut u8,
        i32,
        i32,
        *mut c_void,
        *mut i32,
    ) -> i32 = unsafe { std::mem::transmute(orig_addr) };
    let result = unsafe { original(socket, buf, len, flags, from, from_len) };
    let error = (result < 0).then(|| unsafe { WSAGetLastError() });
    let _ = std::panic::catch_unwind(|| {
        let wire = if result >= 0 && result <= len && !buf.is_null() {
            Some(unsafe { std::slice::from_raw_parts(buf, result as usize) })
        } else {
            None
        };
        let peer =
            if result >= 0 && !from.is_null() && !from_len.is_null() && unsafe { *from_len } >= 8 {
                let bytes = unsafe { std::slice::from_raw_parts(from.cast::<u8>(), 8) };
                super::mercury_recv::wire::ipv4_peer(bytes)
            } else {
                None
            };
        if let Some((level, fields)) =
            super::mercury_recv::wire::recv_event(socket, len, wire, error, peer.as_deref())
        {
            super::emit::emit("client.mercury.socket_recv", level, fields);
        }
    });
    if let Some(error) = error {
        unsafe { WSASetLastError(error) };
    }
    result
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
mod tests;
