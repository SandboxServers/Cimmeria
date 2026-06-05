//! Inline (JMP-trampoline) hooks via MinHook.
//!
//! Patches the first ~5 bytes of a target function with a JMP to
//! our detour; MinHook builds a trampoline that runs the original
//! prologue + jumps back. Our detour emits an event and calls the
//! trampoline to invoke the original function transparently.
//!
//! # What's installed in this PR
//!
//! All four engine-side anchors below were resolved in Ghidra on
//! 2026-06-04. The two with well-known UE3 ABIs (taken from the
//! leaked UE3 source) are enabled; the two whose argument shapes
//! are uncertain are scaffolded but disabled pending signature
//! confirmation — see the `Scaffolded` table below.
//!
//! Enabled:
//!
//! | Function | Address | Target | Sampling |
//! |---|---|---|---|
//! | `Mercury::Nub::handleMessage` | `0x01b18be0` | `client.mercury.dispatch` | none (network-bounded) |
//! | `FEngineLoop::Tick` | `0x00416ec0` | `client.engine.tick` | 1/100 (~0.3-1.2 Hz at 30-120 fps) |
//! | `FArchiveAsync::Serialize` (vtbl slot 1) | `0x004c7ae0` | `client.engine.async_archive_serialize` | 1/1000 (hot during loads) |
//!
//! Scaffolded — address known, signature shape needs RE before enable:
//!
//! | Function | Address | Reason deferred |
//! |---|---|---|
//! | `UWorld::UpdateLevelStreaming` | `0x0054e9c0` | Ghidra recovered `(this, int, float*)` — `int` arg between `this` and the `FVector*` doesn't match the public `UWorld::UpdateLevelStreaming(FVector const&)` signature. Likely a 2009-era SGW override or an additional `bForceLevelStream` flag. Need to confirm exact arg count + order before hooking, or the cdecl/thiscall stack will mismatch and crash. |
//! | `LoadPackageInternal` | `0x004a8e10` | Ghidra recovered 7 `__cdecl` args. The public `UObject::LoadPackage(UPackage*, const TCHAR*, DWORD)` is 3 args, so 0x004a8e10 is `LoadPackageInternal` (the heavy internal helper). UE3 source has multiple internal helpers with different signatures; need to map this one before enabling. The single-string `FailedLoadPackage` anchor confirms the address but not the arg count. |
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

use super::sampling::SamplingCounter;

/// Address of `Mercury::Nub::handleMessage` in SGW.exe. Stable
/// because ASLR is disabled (AtreaFixASLR.bat).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_HANDLE_MESSAGE: usize = 0x01b18be0;

/// Address of `FEngineLoop::Tick(void)` — UE3 main engine tick.
/// Resolved via RTTI walk from `.?AVResetDeferredUpdates@?CA@??Tick@FEngineLoop@@QAEXXZ@`
/// (the local RAII helper inside Tick) at 0x01d8f838. Body
/// 0x00416ec0 - 0x00417333.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_FENGINE_LOOP_TICK: usize = 0x00416ec0;

/// Address of `FArchiveAsync::Serialize(void* V, INT Length)` —
/// vtable slot 1, the bulk byte-read path during async loads.
/// Resolved via RTTI walk from `.?AVFArchiveAsync@@` at
/// 0x01dafd0c → COL @ 0x01b54f94 → vtable @ 0x01814198.
/// Body 0x004c7ae0 - 0x004c7bd8.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_ARCHIVE_ASYNC_SERIALIZE: usize = 0x004c7ae0;

/// Trampoline pointer for `Mercury::Nub::handleMessage`. Set by
/// MinHook at hook install time. Reading `.get()` gives us the
/// address of the original function prologue + JMP back to
/// address+N, callable as if it were the original.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static HANDLE_MESSAGE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Trampoline pointer for `FEngineLoop::Tick`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static TICK_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Trampoline pointer for `FArchiveAsync::Serialize`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ARCHIVE_SERIALIZE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Sampling counter for `FEngineLoop::Tick`. At 30-120 fps the
/// game ticks 30-120 Hz; 1/100 sampling yields ~0.3-1.2 emits/sec
/// — enough to see frame-time anomalies in SigNoz without burning
/// telemetry budget.
///
/// Only the detour reads this in production. On non-x86-Windows
/// targets the detour is cfg'd out and the static is reachable
/// only from the unit tests below; allow dead_code there so the
/// workspace lib build stays warning-clean.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static TICK_SAMPLER: SamplingCounter = SamplingCounter::new(100);

/// Sampling counter for `FArchiveAsync::Serialize`. Bulk byte-read
/// path during package loads; can fire thousands of times per
/// cold-load. 1/1000 sampling keeps it bounded while still showing
/// the load-storm signature in SigNoz.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static ARCHIVE_SERIALIZE_SAMPLER: SamplingCounter = SamplingCounter::new(1000);

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
    install_engine_tick(&producer);
    install_archive_async_serialize(&producer);

    super::emit_info(
        &producer,
        "client.hooks.inline.install_complete",
        [("hook_count", serde_json::json!(3))],
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_handle_message(producer: &Producer) {
    install_one(
        producer,
        "mercury_handle_message",
        ADDR_HANDLE_MESSAGE,
        handle_message_detour as *mut c_void,
        &HANDLE_MESSAGE_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_engine_tick(producer: &Producer) {
    install_one(
        producer,
        "fengineloop_tick",
        ADDR_FENGINE_LOOP_TICK,
        engine_tick_detour as *mut c_void,
        &TICK_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_archive_async_serialize(producer: &Producer) {
    install_one(
        producer,
        "farchiveasync_serialize",
        ADDR_ARCHIVE_ASYNC_SERIALIZE,
        archive_async_serialize_detour as *mut c_void,
        &ARCHIVE_SERIALIZE_TRAMPOLINE,
    );
}

/// Shared CreateHook + EnableHook plumbing — same for every
/// inline target. Reports `installed` on success and
/// `create_failed` / `enable_failed` on failure with the hook name
/// for SigNoz filtering.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_one(
    producer: &Producer,
    hook_name: &'static str,
    address: usize,
    detour: *mut c_void,
    trampoline_slot: &OnceLock<usize>,
) {
    let target = address as *mut c_void;
    let mut trampoline: *mut c_void = std::ptr::null_mut();

    let create_status = minhook_sys::MH_CreateHook(target, detour, &mut trampoline);
    if create_status != minhook_sys::MH_OK {
        super::emit_warn(
            producer,
            "client.hooks.inline.create_failed",
            [
                ("hook", serde_json::Value::String(hook_name.into())),
                ("status", serde_json::json!(create_status as i32)),
                (
                    "address",
                    serde_json::Value::String(format!("0x{address:08x}")),
                ),
            ],
        );
        return;
    }
    let _ = trampoline_slot.set(trampoline as usize);

    let enable_status = minhook_sys::MH_EnableHook(target);
    if enable_status != minhook_sys::MH_OK {
        super::emit_warn(
            producer,
            "client.hooks.inline.enable_failed",
            [
                ("hook", serde_json::Value::String(hook_name.into())),
                ("status", serde_json::json!(enable_status as i32)),
            ],
        );
        return;
    }

    super::emit_info(
        producer,
        "client.hooks.inline.installed",
        [
            ("hook", serde_json::Value::String(hook_name.into())),
            (
                "address",
                serde_json::Value::String(format!("0x{address:08x}")),
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

/// Detour for `FEngineLoop::Tick(void)`.
///
/// Signature: `extern "thiscall" fn(*mut FEngineLoop)` — `this` in
/// ECX, no stack args, no return value.
///
/// **Hot path discipline:** runs at 30-120 Hz on the main game
/// thread. Sampled at 1/100 via `TICK_SAMPLER` so the wire rate
/// is bounded to ~0.3-1.2 emits/sec.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn engine_tick_detour(this: *mut c_void) {
    let _ = std::panic::catch_unwind(|| {
        if TICK_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(crate::events::ClientNativeEvent::builder(
                    "client.engine.tick",
                    "debug",
                ));
            }
        }
    });

    if let Some(t) = TICK_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void) = unsafe { std::mem::transmute(*t) };
        original(this);
    }
    // No trampoline → don't call anything. Skipping a Tick is
    // safer than crashing the main thread, and the missing
    // trampoline implies install failed entirely (in which case
    // the hook patch was never applied either).
}

/// Detour for `FArchiveAsync::Serialize(void* V, INT Length)` —
/// vtable slot 1.
///
/// Signature: `extern "thiscall" fn(*mut FArchiveAsync, *mut c_void,
/// c_int)`. ECX = this, stack = V + Length.
///
/// **Hot path discipline:** can fire thousands of times during a
/// cold package load. Sampled at 1/1000 via `ARCHIVE_SERIALIZE_SAMPLER`
/// so we surface the load storm without flooding.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn archive_async_serialize_detour(
    this: *mut c_void,
    v: *mut c_void,
    length: i32,
) {
    let _ = std::panic::catch_unwind(|| {
        if ARCHIVE_SERIALIZE_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder(
                        "client.engine.async_archive_serialize",
                        "debug",
                    )
                    .field("length", serde_json::json!(length)),
                );
            }
        }
    });

    if let Some(t) = ARCHIVE_SERIALIZE_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, *mut c_void, i32) =
            unsafe { std::mem::transmute(*t) };
        original(this, v, length);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the resolved addresses against the documented anchors.
    /// If a future RE pass moves any of them, this test trips and
    /// reminds us to update the docs + commit comment together.
    #[test]
    fn anchor_addresses_match_re_findings() {
        // Ghidra resolution 2026-06-04 (PR #504 follow-up):
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        {
            assert_eq!(ADDR_HANDLE_MESSAGE, 0x01b18be0);
            assert_eq!(ADDR_FENGINE_LOOP_TICK, 0x00416ec0);
            assert_eq!(ADDR_ARCHIVE_ASYNC_SERIALIZE, 0x004c7ae0);
        }
    }

    /// Sampling rates are load-bearing — they bound the wire cost
    /// of the hot-path hooks. Pin them so a careless edit doesn't
    /// 100× the telemetry volume.
    #[test]
    fn sampler_rates_are_bounded() {
        // 1/100 → ~0.3-1.2 emits/sec at 30-120 fps. Already
        // exercised by `samples_every_n` in the sampling module;
        // here we just pin the configured rate.
        let tick_emits: usize = (0..100).filter(|_| TICK_SAMPLER.should_emit()).count();
        // First 100 calls yield exactly 1 emit (n=0 triggers).
        assert_eq!(tick_emits, 1, "tick sampler should emit 1/100");

        // 1/1000 → 1 emit per 1000 calls. Pin by emitting 1000
        // times and counting.
        let archive_emits: usize = (0..1000)
            .filter(|_| ARCHIVE_SERIALIZE_SAMPLER.should_emit())
            .count();
        assert_eq!(archive_emits, 1, "archive sampler should emit 1/1000");
    }
}
