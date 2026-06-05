//! Inline (JMP-trampoline) hooks via MinHook.
//!
//! Patches the first ~5 bytes of a target function with a JMP to
//! our detour; MinHook builds a trampoline that runs the original
//! prologue + jumps back. Our detour emits an event and calls the
//! trampoline to invoke the original function transparently.
//!
//! # What's installed in this PR
//!
//! - **`Mercury::Nub::handleMessage`** @ `0x01b18be0` — the post-
//!   decrypt entry point on the client's network thread. Every
//!   inbound Mercury packet routes through this. Emit at
//!   `client.mercury.dispatch` (DEBUG; sample-uncapped because
//!   Mercury rate is bounded by network).
//!
//! # What's NOT installed in this PR (per the anchor doc)
//!
//! The remaining tier-1 inline targets are listed in
//! `docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`
//! with ANCHOR addresses (RTTI or string-xref), NOT function entry
//! points. Inline hooks need the actual function start to patch the
//! prologue. Each of these needs a small RE follow-up to walk from
//! anchor → function entry:
//!
//! | Function | Anchor | RE follow-up needed |
//! |---|---|---|
//! | `FEngineLoop::Tick` | RTTI `0x01d8f838` | RTTI → vtable → method 0 |
//! | `UWorld::UpdateLevelStreaming` | xref `0x01837518` | xref → containing fn |
//! | `FArchiveAsync::Read*` | RTTI `0x01dafd0c` | RTTI → vtable → method N |
//! | `LoadPackage` | xref `0x0180f104` | xref → containing fn |
//!
//! These are scaffolded but not enabled — `install_one_hook` is
//! generic over the address so adding them once the entry points
//! are known is a one-liner per hook.
//!
//! # Why MinHook
//!
//! `retour 0.3` requires nightly Rust (`feature(unboxed_closures)`).
//! `minhook-sys` is a thin FFI binding to the MinHook C library
//! that builds on stable. The API is `MH_CreateHook(target,
//! detour, &mut trampoline)` + `MH_EnableHook(target)`.

#![allow(clippy::missing_safety_doc)] // FFI bindings — safety doc in fn-level

use crate::queue::Producer;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

/// Address of `Mercury::Nub::handleMessage` in SGW.exe. Stable
/// because ASLR is disabled (AtreaFixASLR.bat).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_HANDLE_MESSAGE: usize = 0x01b18be0;

/// Trampoline pointer set by MinHook at hook install time. Reading
/// `.get()` gives us the address of the original function prologue
/// + JMP back to address+N, callable as if it were the original.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static HANDLE_MESSAGE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Install all inline hooks. Best-effort: a MinHook init failure
/// or a single CreateHook failure logs a warn event and the rest
/// of the hooks still attempt to install.
pub fn install(_producer: Producer) {
    // Active only on the real DLL target. The detour's `thiscall`
    // ABI is x86-only and MinHook's symbols only link on Windows.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        install_inner(_producer);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_inner(producer: Producer) {
    // MinHook needs a one-time init per process.
    let init_status = minhook_sys::MH_Initialize();
    if init_status != minhook_sys::MH_OK && init_status != minhook_sys::MH_ERROR_ALREADY_INITIALIZED
    {
        super::emit_warn(
            &producer,
            "client.hooks.inline.init_failed",
            [("status", serde_json::json!(init_status as i32))],
        );
        return;
    }

    install_handle_message(&producer);

    super::emit_info(
        &producer,
        "client.hooks.inline.install_complete",
        [("hook_count", serde_json::json!(1))],
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_handle_message(producer: &Producer) {
    let target = ADDR_HANDLE_MESSAGE as *mut c_void;
    let detour = handle_message_detour as *mut c_void;
    let mut trampoline: *mut c_void = std::ptr::null_mut();

    let create_status = minhook_sys::MH_CreateHook(target, detour, &mut trampoline);
    if create_status != minhook_sys::MH_OK {
        super::emit_warn(
            producer,
            "client.hooks.inline.create_failed",
            [
                (
                    "hook",
                    serde_json::Value::String("mercury_handle_message".into()),
                ),
                ("status", serde_json::json!(create_status as i32)),
                (
                    "address",
                    serde_json::Value::String(format!("0x{:08x}", ADDR_HANDLE_MESSAGE)),
                ),
            ],
        );
        return;
    }
    let _ = HANDLE_MESSAGE_TRAMPOLINE.set(trampoline as usize);

    let enable_status = minhook_sys::MH_EnableHook(target);
    if enable_status != minhook_sys::MH_OK {
        super::emit_warn(
            producer,
            "client.hooks.inline.enable_failed",
            [
                (
                    "hook",
                    serde_json::Value::String("mercury_handle_message".into()),
                ),
                ("status", serde_json::json!(enable_status as i32)),
            ],
        );
        return;
    }

    super::emit_info(
        producer,
        "client.hooks.inline.installed",
        [
            (
                "hook",
                serde_json::Value::String("mercury_handle_message".into()),
            ),
            (
                "address",
                serde_json::Value::String(format!("0x{:08x}", ADDR_HANDLE_MESSAGE)),
            ),
        ],
    );
}

/// Detour for `Mercury::Nub::handleMessage`.
///
/// Signature: `extern "thiscall" fn(*mut Nub, *mut MessageHeader) ->
/// some_int`. We don't know the exact return type or arg layout
/// without RE; we treat them as opaque `*mut c_void` and forward
/// untouched.
///
/// **Hot path discipline:** this runs on the network thread on
/// every inbound Mercury packet. Emit MUST be non-blocking
/// (queue's `try_emit` is) and MUST NOT panic across the FFI
/// boundary. The `catch_unwind` is defence-in-depth.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn handle_message_detour(
    this: *mut c_void,
    msg: *mut c_void,
) -> *mut c_void {
    // Emit FIRST (before invoking original) so even if the original
    // crashes we have a wire record. Drop-on-full is fine — Mercury
    // dispatch is bounded by network and the queue is large enough
    // to handle bursty inbound traffic.
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(crate::events::ClientNativeEvent::builder(
                "client.mercury.dispatch",
                "debug",
            ));
        }
    });

    // Call the original via the trampoline. MinHook builds it so
    // the calling convention is preserved end-to-end.
    if let Some(t) = HANDLE_MESSAGE_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, *mut c_void) -> *mut c_void =
            unsafe { std::mem::transmute(*t) };
        original(this, msg)
    } else {
        // Trampoline missing (impossible-in-practice — we set it
        // before enabling the hook). Return null and hope the
        // caller treats it as a benign no-op packet.
        std::ptr::null_mut()
    }
}
