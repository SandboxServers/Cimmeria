//! Inline (JMP-trampoline) hooks via MinHook.
//!
//! Patches the first ~5 bytes of a target function with a JMP to
//! our detour; MinHook builds a trampoline that runs the original
//! prologue + jumps back. Our detour emits an event and calls the
//! trampoline to invoke the original function transparently.
//!
//! # What's installed
//!
//! Phase 2 (5 hooks) + Phase 3/5 (6 additional inline hooks) below.
//! All Phase 3-5 anchors resolved + signature-confirmed in Ghidra
//! on 2026-06-04 (see `client-instrumentation-entry-points.md`).
//!
//! | Function | Address | Target | Sampling |
//! |---|---|---|---|
//! | `Mercury::Nub::handleMessage` | `0x01b18be0` | `client.mercury.dispatch` | none (network-bounded) |
//! | `FEngineLoop::Tick` | `0x00416ec0` | `client.engine.tick` | 1/100 (~0.3-1.2 Hz at 30-120 fps) |
//! | `FArchiveAsync::Serialize` (vtbl slot 1) | `0x004c7ae0` | `client.engine.async_archive_serialize` | 1/1000 (hot during loads) |
//! | `UWorld::UpdateLevelStreamingInner` | `0x0054e9c0` | `client.engine.update_level_streaming` | 1/10 (fires per streaming level per frame) |
//! | `UObject::StaticLoadObject` | `0x004a8e10` | `client.engine.static_load_object` (with `package_name` field) | 1/10 (bursts during cold loads) |
//! | `GameBeing::onStateFieldUpdate` | `0x00e01c90` | `client.state.field_update` | 1/1 (one hook covers all 9 BSF_* flags via dispatcher) |
//! | `USGWAnimNotify_Event::Notify` (A) | `0x00e974b0` | `client.anim.notify` (`variant=a`) | 1/100 (shared with B) |
//! | `USGWAnimNotify_Event::Notify` (B) | `0x00e97070` | `client.anim.notify` (`variant=b`) | 1/100 (shared with A) |
//! | Cooked-data PAK load | `0x00420074` | `client.engine.pak_load` (with `category` field) | 1/1 (21 PAKs total) |
//! | `APlayerController::execConsoleCommand` | `0x00539850` | `client.input.console_command` | 1/1 |
//! | `FFullScreenMovieBink::Tick` (vtbl slot 1) | `0x0050bbc0` | `client.engine.bink_tick` (with `delta_seconds` field) | 1/30 (~1/sec during cinematics) |
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

/// Address of `UWorld::UpdateLevelStreamingInner(ULevelStreaming*,
/// FVector*)` — the per-streaming-level helper.
///
/// Resolved 2026-06-04 by xref to the "StreamingLevel" check-macro
/// string @ 0x01837518; signature confirmed by decompile (param_1
/// is StreamingLevel pointer, param_2 is FVector pointer derefed
/// as 3 floats for distance check).
///
/// The OUTER `UWorld::UpdateLevelStreaming` is at 0x005527a0 —
/// it loops over streaming levels and invokes this inner per-
/// level. Hooking the inner gives per-level transition events.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_UPDATE_LEVEL_STREAMING_INNER: usize = 0x0054e9c0;

/// Address of `UObject::StaticLoadObject(UClass*, UObject*, TCHAR*,
/// TCHAR*, DWORD, UPackageMap*, UBOOL)` — the canonical generic
/// object loader (UnObj.cpp).
///
/// Resolved 2026-06-04 by xref to "FailedLoadPackage" wstring @
/// 0x0180f104 (in the SEH catch handler); signature confirmed by
/// decompile against UE3 leaked source. The 7 cdecl args match
/// the UE3 source exactly; the recursive self-call at 0x004a8f8d
/// is the redirector-handling pattern.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_STATIC_LOAD_OBJECT: usize = 0x004a8e10;

// ─── Phase 3 Tier 3 inline targets (resolved 2026-06-04) ────────

/// `GameBeing::onStateFieldUpdate` — CME-subscribed dispatcher
/// that XOR-delta-decodes the 9 BSF_* state flags (Dead,
/// AutoCycling, Crouching, InCombat, PlayingMinigame, InStealth,
/// MovementLock, Walking, Holster). One hook here covers all
/// state-flag transitions. See `state-flag-broadcast.md`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_STATE_FIELD_UPDATE: usize = 0x00e01c90;

/// `USGWAnimNotify_Event::Notify` variant A — 1.2 KB body.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_ANIM_NOTIFY_A: usize = 0x00e974b0;

/// `USGWAnimNotify_Event::Notify` variant B — 450 B body, likely
/// a specialized notify path (per-tick or per-key variant).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_ANIM_NOTIFY_B: usize = 0x00e97070;

/// Cooked-data PAK load — fires once per PAK load (21 categories).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_COOKED_DATA_LOAD: usize = 0x00420074;

/// `APlayerController::execConsoleCommand` — UnrealScript exec
/// wrapper, FuncMap-bound at 0x01db2460. Fires on every script-
/// invoked console command.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_CONSOLE_COMMAND: usize = 0x00539850;

// ─── Phase 5 Tier 6 inline target (resolved 2026-06-04) ─────────

/// `FFullScreenMovieBink::Tick` — cinematic playback per-frame
/// driver. Vfunc 1 of FFullScreenMovieBink, 154 B body. The
/// presence of this firing distinguishes a real stall from
/// expected cinematic-bound frames.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ADDR_BINK_TICK: usize = 0x0050bbc0;

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

/// Trampoline pointer for `UWorld::UpdateLevelStreamingInner`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static UPDATE_LEVEL_STREAMING_INNER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Trampoline pointer for `UObject::StaticLoadObject`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static STATIC_LOAD_OBJECT_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static STATE_FIELD_UPDATE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ANIM_NOTIFY_A_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ANIM_NOTIFY_B_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static COOKED_DATA_LOAD_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static CONSOLE_COMMAND_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static BINK_TICK_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

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

/// Sampling counter for `UWorld::UpdateLevelStreamingInner`. Fires
/// per streaming level per frame — typically 1-10 active streaming
/// levels at any time, so 1/10 sampling keeps the wire rate ≤ ~1
/// emit/sec at 30 fps with 10 streaming levels.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static UPDATE_LEVEL_STREAMING_SAMPLER: SamplingCounter = SamplingCounter::new(10);

/// Sampling counter for `UObject::StaticLoadObject`. Called bursts
/// of dozens to thousands during a cold load; 1/10 sampling makes
/// the load storm visible without flooding SigNoz with every nested
/// import resolve.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static STATIC_LOAD_OBJECT_SAMPLER: SamplingCounter = SamplingCounter::new(10);

/// `Bink::Tick` runs at the cinematic's framerate (30 fps typical).
/// 1/30 sampling yields ~1 emit/sec — enough to detect "stuck on
/// a cinematic frame" without flooding.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static BINK_TICK_SAMPLER: SamplingCounter = SamplingCounter::new(30);

/// `AnimNotify::Notify` fires on every animation notify event —
/// can fire several times per actor per frame during combat
/// animations. 1/100 sampling keeps emit rate manageable.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static ANIM_NOTIFY_SAMPLER: SamplingCounter = SamplingCounter::new(100);

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
    install_update_level_streaming_inner(&producer);
    install_static_load_object(&producer);
    install_state_field_update(&producer);
    install_anim_notify_a(&producer);
    install_anim_notify_b(&producer);
    install_cooked_data_load(&producer);
    install_console_command(&producer);
    install_bink_tick(&producer);

    super::emit_info(
        &producer,
        "client.hooks.inline.install_complete",
        [("hook_count", serde_json::json!(11))],
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

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_update_level_streaming_inner(producer: &Producer) {
    install_one(
        producer,
        "uworld_update_level_streaming_inner",
        ADDR_UPDATE_LEVEL_STREAMING_INNER,
        update_level_streaming_inner_detour as *mut c_void,
        &UPDATE_LEVEL_STREAMING_INNER_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_static_load_object(producer: &Producer) {
    install_one(
        producer,
        "uobject_static_load_object",
        ADDR_STATIC_LOAD_OBJECT,
        static_load_object_detour as *mut c_void,
        &STATIC_LOAD_OBJECT_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_state_field_update(producer: &Producer) {
    install_one(
        producer,
        "gamebeing_state_field_update",
        ADDR_STATE_FIELD_UPDATE,
        state_field_update_detour as *mut c_void,
        &STATE_FIELD_UPDATE_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_anim_notify_a(producer: &Producer) {
    install_one(
        producer,
        "anim_notify_a",
        ADDR_ANIM_NOTIFY_A,
        anim_notify_a_detour as *mut c_void,
        &ANIM_NOTIFY_A_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_anim_notify_b(producer: &Producer) {
    install_one(
        producer,
        "anim_notify_b",
        ADDR_ANIM_NOTIFY_B,
        anim_notify_b_detour as *mut c_void,
        &ANIM_NOTIFY_B_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_cooked_data_load(producer: &Producer) {
    install_one(
        producer,
        "cooked_data_load",
        ADDR_COOKED_DATA_LOAD,
        cooked_data_load_detour as *mut c_void,
        &COOKED_DATA_LOAD_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_console_command(producer: &Producer) {
    install_one(
        producer,
        "console_command",
        ADDR_CONSOLE_COMMAND,
        console_command_detour as *mut c_void,
        &CONSOLE_COMMAND_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_bink_tick(producer: &Producer) {
    install_one(
        producer,
        "bink_tick",
        ADDR_BINK_TICK,
        bink_tick_detour as *mut c_void,
        &BINK_TICK_TRAMPOLINE,
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

/// Detour for `UWorld::UpdateLevelStreamingInner(ULevelStreaming*,
/// FVector*)`.
///
/// Signature: `extern "thiscall" fn(*mut UWorld, *mut ULevelStreaming,
/// *mut FVector)`. ECX = this, stack = (StreamingLevel, FVector*).
///
/// **Hot path discipline:** fires per active streaming level per
/// frame on the main game thread. Sampled at 1/10 via
/// `UPDATE_LEVEL_STREAMING_SAMPLER`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn update_level_streaming_inner_detour(
    this: *mut c_void,
    streaming_level: *mut c_void,
    delta_position: *mut c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if UPDATE_LEVEL_STREAMING_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(crate::events::ClientNativeEvent::builder(
                    "client.engine.update_level_streaming",
                    "debug",
                ));
            }
        }
    });

    if let Some(t) = UPDATE_LEVEL_STREAMING_INNER_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, *mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(this, streaming_level, delta_position);
    }
}

/// Detour for `UObject::StaticLoadObject(UClass*, UObject*, TCHAR*,
/// TCHAR*, DWORD, UPackageMap*, UBOOL) -> UObject*`.
///
/// Signature: 7 `__cdecl` args matching UE3 leaked source. ALL
/// args must be forwarded to the trampoline or the original
/// function reads garbage off the stack.
///
/// **Hot path discipline:** fires in bursts of dozens to thousands
/// during cold loads. Sampled at 1/10 via
/// `STATIC_LOAD_OBJECT_SAMPLER`.
///
/// **Sampled emit captures `package_name`**: when we DO emit, we
/// read up to 256 wchar_t from `in_name` (bounded, null-terminated
/// safe) and ship it as a UTF-8 string field. This is the load-
/// freeze investigation gold: SigNoz can show "which package was
/// loading when the client froze."
///
/// **Safety:** `in_name` is a stack-passed `*const u16`. The caller
/// holds the buffer for the duration of the call; we only read
/// from it inside the detour (before invoking the trampoline) so
/// the buffer is still live. The 256-wchar bound prevents runaway
/// reads if the string isn't null-terminated.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn static_load_object_detour(
    object_class: *mut c_void,
    in_outer: *mut c_void,
    in_name: *const u16,
    filename: *const u16,
    load_flags: u32,
    sandbox: *mut c_void,
    allow_reconciliation: i32,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if STATIC_LOAD_OBJECT_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                let name = read_utf16_bounded(in_name, 256);
                p.try_emit(
                    crate::events::ClientNativeEvent::builder(
                        "client.engine.static_load_object",
                        "debug",
                    )
                    .field("package_name", serde_json::Value::String(name))
                    .field("load_flags", serde_json::json!(load_flags)),
                );
            }
        }
    });

    if let Some(t) = STATIC_LOAD_OBJECT_TRAMPOLINE.get() {
        let original: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *const u16,
            *const u16,
            u32,
            *mut c_void,
            i32,
        ) -> *mut c_void = unsafe { std::mem::transmute(*t) };
        original(
            object_class,
            in_outer,
            in_name,
            filename,
            load_flags,
            sandbox,
            allow_reconciliation,
        )
    } else {
        // Trampoline missing — return null. The caller will treat
        // it as "object not found" and likely log + recover. Better
        // than crashing.
        std::ptr::null_mut()
    }
}

/// Detour for `GameBeing::onStateFieldUpdate` — CME dispatcher
/// for the 9 BSF_* state flags.
///
/// Signature: `extern "thiscall" fn(*mut GameBeing, *mut EventData)`.
///
/// **Hot path discipline:** state-flag changes (combat enter/exit,
/// stealth toggle, holster) are infrequent (a few per minute per
/// player typically). No sampling — emit 1/1.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn state_field_update_detour(this: *mut c_void, event_data: *mut c_void) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(crate::events::ClientNativeEvent::builder(
                "client.state.field_update",
                "debug",
            ));
        }
    });

    if let Some(t) = STATE_FIELD_UPDATE_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(this, event_data);
    }
}

/// Detour for `USGWAnimNotify_Event::Notify` (variant A).
///
/// Signature: `extern "thiscall" fn(*mut this, int notify_type)`.
///
/// **Hot path discipline:** fires several times per actor per frame
/// during combat animations. Sampled at 1/100 via `ANIM_NOTIFY_SAMPLER`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn anim_notify_a_detour(this: *mut c_void, notify_arg: i32) {
    let _ = std::panic::catch_unwind(|| {
        if ANIM_NOTIFY_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.anim.notify", "debug")
                        .field("variant", serde_json::json!("a")),
                );
            }
        }
    });

    if let Some(t) = ANIM_NOTIFY_A_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, i32) =
            unsafe { std::mem::transmute(*t) };
        original(this, notify_arg);
    }
}

/// Detour for `USGWAnimNotify_Event::Notify` (variant B).
/// Shares sampling counter with variant A so the combined rate is
/// 1/100 per (A or B) call.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn anim_notify_b_detour(this: *mut c_void, notify_arg: i32) {
    let _ = std::panic::catch_unwind(|| {
        if ANIM_NOTIFY_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.anim.notify", "debug")
                        .field("variant", serde_json::json!("b")),
                );
            }
        }
    });

    if let Some(t) = ANIM_NOTIFY_B_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, i32) =
            unsafe { std::mem::transmute(*t) };
        original(this, notify_arg);
    }
}

/// Detour for the cooked-data PAK loader.
///
/// Signature: `extern "cdecl" fn(u32 category, void** out_a, void** out_b)`.
///
/// Fires once per PAK load (21 categories). Emit 1/1 — rare event.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn cooked_data_load_detour(
    category: u32,
    out_a: *mut *mut c_void,
    out_b: *mut *mut c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.engine.pak_load", "info")
                    .field("category", serde_json::json!(category)),
            );
        }
    });

    if let Some(t) = COOKED_DATA_LOAD_TRAMPOLINE.get() {
        let original: unsafe extern "C" fn(u32, *mut *mut c_void, *mut *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(category, out_a, out_b);
    }
}

/// Detour for `APlayerController::execConsoleCommand`.
///
/// Signature: `extern "thiscall" fn(*mut APlayerController, int frame_ptr)`.
/// The UnrealScript exec wrapper — actual command string is in the
/// FFrame at param_1. Decoding it here would require parsing FFrame
/// bytecode; skip for now and emit the bare event.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn console_command_detour(this: *mut c_void, frame: i32) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(crate::events::ClientNativeEvent::builder(
                "client.input.console_command",
                "info",
            ));
        }
    });

    if let Some(t) = CONSOLE_COMMAND_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, i32) =
            unsafe { std::mem::transmute(*t) };
        original(this, frame);
    }
}

/// Detour for `FFullScreenMovieBink::Tick` — vtable slot 1.
///
/// Signature: `extern "thiscall" fn(*mut FFullScreenMovieBink, float DeltaSeconds)`.
///
/// **Hot path discipline:** fires every frame during cinematic
/// playback (30 fps typical). Sampled at 1/30 via `BINK_TICK_SAMPLER`
/// for ~1 emit/sec during cinematics.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn bink_tick_detour(this: *mut c_void, delta_seconds: f32) {
    let _ = std::panic::catch_unwind(|| {
        if BINK_TICK_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.engine.bink_tick", "debug")
                        .field("delta_seconds", serde_json::json!(delta_seconds)),
                );
            }
        }
    });

    if let Some(t) = BINK_TICK_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, f32) =
            unsafe { std::mem::transmute(*t) };
        original(this, delta_seconds);
    }
}

/// Read a UTF-16 C string up to `max_chars`. Stops at the first
/// NUL or at the bound, whichever comes first. Returns the UTF-8
/// representation with `U+FFFD` replacement for invalid surrogates.
///
/// Used by the `static_load_object_detour` to capture the package
/// name for the emit field. The bound exists because we don't
/// trust the caller's buffer to be NUL-terminated — a runaway read
/// inside the detour would risk an access violation that takes
/// the whole game with it.
///
/// Returns `"<null>"` on null pointer, `""` on empty string. Never
/// reads past `max_chars` wchars (= 2 × max_chars bytes).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) fn read_utf16_bounded(ptr: *const u16, max_chars: usize) -> String {
    if ptr.is_null() {
        return "<null>".into();
    }
    let mut len = 0usize;
    while len < max_chars {
        // SAFETY: bounded by max_chars; caller's contract is the
        // buffer is valid for at least max_chars wchars OR
        // NUL-terminated sooner. We exit on first NUL.
        let ch = unsafe { *ptr.add(len) };
        if ch == 0 {
            break;
        }
        len += 1;
    }
    // SAFETY: `len` is the number of u16 elements before NUL,
    // bounded by max_chars. `from_raw_parts` on the same pointer
    // with that length is sound.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
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
            assert_eq!(ADDR_UPDATE_LEVEL_STREAMING_INNER, 0x0054e9c0);
            assert_eq!(ADDR_STATIC_LOAD_OBJECT, 0x004a8e10);
            // Phase 3 + 5 inline targets (2026-06-04 manifest):
            assert_eq!(ADDR_STATE_FIELD_UPDATE, 0x00e01c90);
            assert_eq!(ADDR_ANIM_NOTIFY_A, 0x00e974b0);
            assert_eq!(ADDR_ANIM_NOTIFY_B, 0x00e97070);
            assert_eq!(ADDR_COOKED_DATA_LOAD, 0x00420074);
            assert_eq!(ADDR_CONSOLE_COMMAND, 0x00539850);
            assert_eq!(ADDR_BINK_TICK, 0x0050bbc0);
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

        // 1/10 for the two coarser hooks.
        let stream_emits: usize = (0..10)
            .filter(|_| UPDATE_LEVEL_STREAMING_SAMPLER.should_emit())
            .count();
        assert_eq!(stream_emits, 1, "stream sampler should emit 1/10");

        let load_emits: usize = (0..10)
            .filter(|_| STATIC_LOAD_OBJECT_SAMPLER.should_emit())
            .count();
        assert_eq!(load_emits, 1, "load-object sampler should emit 1/10");

        // 1/30 — Bink frame cadence.
        let bink_emits: usize = (0..30).filter(|_| BINK_TICK_SAMPLER.should_emit()).count();
        assert_eq!(bink_emits, 1, "bink sampler should emit 1/30");

        // 1/100 shared between anim variants A + B.
        let anim_emits: usize = (0..100)
            .filter(|_| ANIM_NOTIFY_SAMPLER.should_emit())
            .count();
        assert_eq!(anim_emits, 1, "anim sampler should emit 1/100");
    }

    /// `read_utf16_bounded` must:
    /// 1. Stop at the NUL terminator.
    /// 2. Cap at `max_chars` so a missing NUL can't cause a runaway
    ///    read (the safety contract for use inside the StaticLoadObject
    ///    detour — caller's buffer is foreign-allocated, not trusted).
    /// 3. Handle a null pointer without UB.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_stops_at_nul() {
        let buf: Vec<u16> = "Engine/EngineMaterials/DefaultMaterial\0extra"
            .encode_utf16()
            .collect();
        let got = read_utf16_bounded(buf.as_ptr(), 256);
        assert_eq!(got, "Engine/EngineMaterials/DefaultMaterial");
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_caps_at_max() {
        // 300 chars, no NUL — must stop at max_chars=8.
        let buf: Vec<u16> = (0..300).map(|_| 'A' as u16).collect();
        let got = read_utf16_bounded(buf.as_ptr(), 8);
        assert_eq!(got, "AAAAAAAA");
        assert_eq!(got.chars().count(), 8);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_null_ptr() {
        assert_eq!(read_utf16_bounded(std::ptr::null(), 256), "<null>");
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_empty_string() {
        let buf: Vec<u16> = vec![0u16, 0u16];
        let got = read_utf16_bounded(buf.as_ptr(), 256);
        assert_eq!(got, "");
    }
}
