//! Vtable-slot swap hooks.
//!
//! Replaces one slot in a known vtable with a pointer to our
//! detour. Every call through that virtual method on instances of
//! the class (and all subclasses sharing the same vtable entry)
//! routes through the detour. The detour calls the saved original
//! to chain.
//!
//! # Hook surface (this module)
//!
//! - **CEGUI::DefaultLogger::logEvent** (vtable @ `0x01ac1ba8`,
//!   slot 1) — every UI log line (Phase 4).
//! - **AActor::Tick** (vtable @ `0x0183c40c`, slot 88) — every
//!   per-frame actor tick across ~110 subclasses (Phase 3 Tier 4).
//! - **USequence::UpdateOp** (vtable @ `0x01854a84`, slot 84) —
//!   per-frame kismet sequence tick (Phase 3 Tier 4).
//!
//! # Technique
//!
//! Vtable slots live in `.rdata`. Swap procedure:
//!
//! 1. `VirtualProtect` the slot to `PAGE_READWRITE`.
//! 2. Read original, write detour.
//! 3. Restore protection.
//!
//! Same shape as IAT hooks; the difference is the data layout
//! (vtables are arrays of code pointers; IAT is one slot per
//! import).
//!
//! # Single-base-vtable shortcut
//!
//! For `AActor::Tick` and `USequence::UpdateOp` the same address
//! lives in the base vtable AND ~110 (AActor) / 3 (USequence)
//! subclass vtables — because most subclasses don't override the
//! method. We only swap the base; subclasses that DO override get
//! their own override called (we don't intercept those), but the
//! ~99% of un-overriding subclasses route through our detour as
//! intended.
//!
//! Full coverage (intercepting every override) would require
//! walking the RTTI class hierarchy and swapping every per-subclass
//! vtable slot — deferred to a follow-up phase if needed.
//!
//! # Safety
//!
//! Same as IAT: detour ABI MUST match the virtual method exactly.
//! All three targets here are `__thiscall`.

#![allow(clippy::missing_safety_doc)] // FFI bindings — safety doc at fn-level

use crate::queue::Producer;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::atomic::{AtomicUsize, Ordering};

// ─── Vtable slot addresses ─────────────────────────────────────
//
// SLOT addresses = `vtable_base + slot_index * 4`. These are the
// addresses we patch.

/// CEGUI::DefaultLogger vtable @ `0x01ac1ba8`, slot 1 (logEvent).
/// Slot ptr = `0x01ac1ba8 + 1*4` = `0x01ac1bac`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const VTSLOT_CEGUI_LOG_EVENT: usize = 0x01ac1ba8 + 4;

/// AActor vtable @ `0x0183c40c`, slot 88 (Tick).
/// Slot ptr = `0x0183c40c + 88*4` = `0x0183c56c`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const VTSLOT_AACTOR_TICK: usize = 0x0183c40c + 88 * 4;

/// USequence vtable @ `0x01854a84`, slot 84 (UpdateOp).
/// Slot ptr = `0x01854a84 + 84*4` = `0x01854bd4`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const VTSLOT_USEQUENCE_UPDATE_OP: usize = 0x01854a84 + 84 * 4;

// ─── Saved originals ───────────────────────────────────────────

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_CEGUI_LOG_EVENT: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_AACTOR_TICK: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ORIG_USEQUENCE_UPDATE_OP: AtomicUsize = AtomicUsize::new(0);

// ─── Sampling ─────────────────────────────────────────────────

/// `AActor::Tick` fires for every tickable actor every frame —
/// potentially thousands per second. Sample 1/1000.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static AACTOR_TICK_SAMPLER: super::sampling::SamplingCounter =
    super::sampling::SamplingCounter::new(1000);

/// `USequence::UpdateOp` fires per active kismet sequence per frame
/// — typically 1-5 active sequences. Sample 1/10.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static USEQUENCE_UPDATE_OP_SAMPLER: super::sampling::SamplingCounter =
    super::sampling::SamplingCounter::new(10);

// CEGUI logEvent is rare (UI events only) — 1/1.

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
        "cegui_log_event",
        VTSLOT_CEGUI_LOG_EVENT,
        cegui_log_event_detour as *const c_void as usize,
        &ORIG_CEGUI_LOG_EVENT,
    );
    install_one(
        &producer,
        "aactor_tick",
        VTSLOT_AACTOR_TICK,
        aactor_tick_detour as *const c_void as usize,
        &ORIG_AACTOR_TICK,
    );
    install_one(
        &producer,
        "usequence_update_op",
        VTSLOT_USEQUENCE_UPDATE_OP,
        usequence_update_op_detour as *const c_void as usize,
        &ORIG_USEQUENCE_UPDATE_OP,
    );

    super::emit_info(
        &producer,
        "client.hooks.vtable.install_complete",
        [("hook_count", serde_json::json!(3))],
    );
}

/// Generic vtable-slot swap. Identical mechanics to the IAT swap
/// in `iat_hooks::install_one` but logs under
/// `client.hooks.vtable.*` for SigNoz filtering.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_one(
    producer: &Producer,
    hook_name: &'static str,
    slot_addr: usize,
    detour: usize,
    orig_slot: &AtomicUsize,
) {
    use windows_sys::Win32::System::Memory::{
        VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE,
    };

    let slot_ptr = slot_addr as *mut usize;
    let mut old_protect: PAGE_PROTECTION_FLAGS = 0;

    let ok = VirtualProtect(
        slot_ptr as *const c_void,
        std::mem::size_of::<usize>(),
        PAGE_READWRITE,
        &mut old_protect,
    );
    if ok == 0 {
        super::emit_warn(
            producer,
            "client.hooks.vtable.protect_failed",
            [
                ("hook", serde_json::Value::String(hook_name.into())),
                (
                    "address",
                    serde_json::Value::String(format!("0x{slot_addr:08x}")),
                ),
            ],
        );
        return;
    }

    let original = *slot_ptr;
    *slot_ptr = detour;
    orig_slot.store(original, Ordering::Release);

    let mut _unused: PAGE_PROTECTION_FLAGS = 0;
    let _ = VirtualProtect(
        slot_ptr as *const c_void,
        std::mem::size_of::<usize>(),
        old_protect,
        &mut _unused,
    );

    super::emit_info(
        producer,
        "client.hooks.vtable.installed",
        [
            ("hook", serde_json::Value::String(hook_name.into())),
            (
                "address",
                serde_json::Value::String(format!("0x{slot_addr:08x}")),
            ),
            (
                "original",
                serde_json::Value::String(format!("0x{original:08x}")),
            ),
        ],
    );
}

// ─── Detours ───────────────────────────────────────────────────

/// `CEGUI::DefaultLogger::logEvent(const String& message,
///   LoggingLevel level)` — vtable slot 1.
///
/// Signature: `__thiscall fn(this, String const* message, int level)`.
/// CEGUI's `String` is a custom class with a raw `wchar_t*` field
/// at offset 0; we don't dereference it from the detour because
/// CEGUI String semantics (refcount + grow buffer) are intricate
/// and we'd rather not risk a refcount imbalance. The bare event
/// is the load-bearing telemetry.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn cegui_log_event_detour(
    this: *mut c_void,
    message: *mut c_void,
    level: i32,
) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.ui.cegui_log", "debug")
                    .field("level", serde_json::json!(level)),
            );
        }
    });

    let orig_addr = ORIG_CEGUI_LOG_EVENT.load(Ordering::Acquire);
    if orig_addr == 0 {
        return;
    }
    let original: unsafe extern "thiscall" fn(*mut c_void, *mut c_void, i32) =
        unsafe { std::mem::transmute(orig_addr) };
    original(this, message, level);
}

/// `AActor::Tick(FLOAT DeltaSeconds, ELevelTick TickType) -> UBOOL`
/// — vtable slot 88.
///
/// **Hot path discipline:** runs for every tickable actor every
/// frame. Sample 1/1000 (~few/sec at typical actor counts).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn aactor_tick_detour(
    this: *mut c_void,
    delta_seconds: f32,
    tick_type: i32,
) -> i32 {
    let _ = std::panic::catch_unwind(|| {
        if AACTOR_TICK_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.engine.actor_tick", "debug")
                        .field("delta_seconds", serde_json::json!(delta_seconds))
                        .field("tick_type", serde_json::json!(tick_type)),
                );
            }
        }
    });

    let orig_addr = ORIG_AACTOR_TICK.load(Ordering::Acquire);
    if orig_addr == 0 {
        // Trampoline missing — return UBOOL 1 (success-ish) to let
        // the engine think the tick ran. Safer than 0 which might
        // trigger a "actor disabled itself" path.
        return 1;
    }
    let original: unsafe extern "thiscall" fn(*mut c_void, f32, i32) -> i32 =
        unsafe { std::mem::transmute(orig_addr) };
    original(this, delta_seconds, tick_type)
}

/// `USequence::UpdateOp(FLOAT DeltaTime) -> UBOOL` — vtable slot 84.
///
/// **Hot path discipline:** per active kismet sequence per frame.
/// Sample 1/10.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn usequence_update_op_detour(this: *mut c_void, delta_time: f32) -> i32 {
    let _ = std::panic::catch_unwind(|| {
        if USEQUENCE_UPDATE_OP_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder(
                        "client.engine.sequence_tick",
                        "debug",
                    )
                    .field("delta_time", serde_json::json!(delta_time)),
                );
            }
        }
    });

    let orig_addr = ORIG_USEQUENCE_UPDATE_OP.load(Ordering::Acquire);
    if orig_addr == 0 {
        return 1;
    }
    let original: unsafe extern "thiscall" fn(*mut c_void, f32) -> i32 =
        unsafe { std::mem::transmute(orig_addr) };
    original(this, delta_time)
}

#[cfg(test)]
mod tests {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    use super::*;

    /// Vtable slot addresses pin against the manifest.
    #[test]
    fn vtable_slot_addresses_match_manifest() {
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        {
            assert_eq!(VTSLOT_CEGUI_LOG_EVENT, 0x01ac1bac);
            assert_eq!(VTSLOT_AACTOR_TICK, 0x0183c56c);
            assert_eq!(VTSLOT_USEQUENCE_UPDATE_OP, 0x01854bd4);
        }
    }

    /// Sampler rates pin — vtable hooks at hot rates.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn vtable_sampler_rates() {
        let actor_emits: usize = (0..1000)
            .filter(|_| AACTOR_TICK_SAMPLER.should_emit())
            .count();
        assert_eq!(actor_emits, 1, "actor tick sampler should emit 1/1000");

        let seq_emits: usize = (0..10)
            .filter(|_| USEQUENCE_UPDATE_OP_SAMPLER.should_emit())
            .count();
        assert_eq!(seq_emits, 1, "sequence sampler should emit 1/10");
    }
}
