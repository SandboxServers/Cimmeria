//! Network-thread dispatch hooks: `Mercury_Nub_handleMessage` (every
//! inbound Mercury message) and the entity-method **silent-drop
//! oracle** — the client-side half of the round-trip verification
//! loop.
//!
//! `Mercury_Nub_handleMessage` was re-resolved for #989. The old
//! anchor, `0x01b18be0`, is the log string
//! `"Mercury::Nub::handleMessage: received the wrong kind of
//! message!\n"` that this function prints on a malformed message, not
//! code. Ghidra's own decompile (headless pass, 2026-09-28) names the
//! function `Mercury_Nub_handleMessage` and confirms the entry at
//! `0x0157bd30`, an SEH-guarded `__thiscall` whose five exits are all
//! `ret 0x10` — four stack dwords beyond the `this` in ECX. The
//! decompiler could account for only three of them by name
//! (`param_1`, reassigned before it is ever read, so it is dead on
//! entry; `param_2`, the message struct; `param_3`, the handler
//! interface): the fourth stack slot is real (the immediate `ret 0x10`
//! is unambiguous) but nothing in the decompiled body touches it, so
//! its purpose is unresolved. The detour below declares and forwards
//! all four regardless — an unused stack slot must still be present
//! and passed through untouched, or the trampoline's `ret 0x10`
//! unbalances the caller's stack.
//!
//! The function gates on `*param_2 == -1` (its own sentinel check for
//! "is this really one of our messages") before falling through to
//! the real dispatch, so it is a legitimate per-inbound-message
//! choke point, matching `client.mercury.dispatch`'s original intent.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

use crate::hooks::sampling::SamplingCounter;

/// `Mercury_Nub_handleMessage` — see the module docs. Resolved 2026-
/// 09-28 for #989.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_MERCURY_DISPATCH: usize = 0x0157bd30;

/// Trampoline pointer for `Mercury_Nub_handleMessage`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static MERCURY_DISPATCH_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Fires once per inbound Mercury message. 1/20 keeps a busy session
/// (tens of messages/sec) from dominating the wire budget while still
/// showing the dispatch rate as a rough shape in SigNoz.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static MERCURY_DISPATCH_SAMPLER: SamplingCounter = SamplingCounter::new(20);

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_mercury_dispatch(producer: &Producer) {
    super::install_one(
        producer,
        "mercury_dispatch",
        ADDR_MERCURY_DISPATCH,
        mercury_dispatch_detour as *mut c_void,
        &MERCURY_DISPATCH_TRAMPOLINE,
    );
}

/// Detour for `Mercury_Nub_handleMessage(this, param_1, msg, iface,
/// param_4)`. `param_1` and `param_4` are opaque stack slots the
/// original never reads on entry (see module docs) — forwarded
/// untouched, never dereferenced.
///
/// **Hot path discipline:** network thread, fires per inbound message.
/// Sampled at 1/20. The only read is `*msg` (the sentinel byte the
/// original itself reads unconditionally at entry, so this dereference
/// is exactly as safe as the function's own first instruction) —
/// guarded against a null `msg` regardless.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn mercury_dispatch_detour(
    this: *mut c_void,
    param_1: u32,
    msg: *mut c_void,
    iface: *mut c_void,
    param_4: u32,
) {
    let _ = std::panic::catch_unwind(|| {
        if MERCURY_DISPATCH_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                // SAFETY: `msg` is the same pointer the original
                // function dereferences as its very first instruction
                // (`cmp byte ptr [ebx], 0xff`) before any validation of
                // its own — reading one byte here is no less safe than
                // letting the trampoline run. Still null-checked
                // because our detour runs before that instruction.
                let sentinel_ok = (!msg.is_null()) && unsafe { *(msg as *const u8) } == 0xff;
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.mercury.dispatch", "debug")
                        .field("sentinel_ok", serde_json::json!(sentinel_ok)),
                );
            }
        }
    });

    if let Some(t) = MERCURY_DISPATCH_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall-unwind" fn(
            *mut c_void,
            u32,
            *mut c_void,
            *mut c_void,
            u32,
        ) = unsafe { std::mem::transmute(*t) };
        original(this, param_1, msg, iface, param_4);
    }
}

/// The "method not found" callee on the silent-drop path of
/// `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`).
///
/// The dispatcher searches the entity description's red-black
/// method-handler map (`desc+0xe0`) keyed by
/// `(componentKey, methodIndex)`. On a hit it fires the CME event.
/// On a miss it calls this function at `0x00c6fa95` and returns —
/// **the inbound entity method is discarded with no log, no error,
/// and no wire response.** That silence is the failure mode this
/// hook exists to make visible.
///
/// Hooked here rather than at the dispatcher entry for three
/// reasons: the method index is computed *inside* the dispatcher
/// (`FUN_01590bb0`) so an entry hook cannot report it; this is a
/// function entry, so the ordinary inline-hook path works instead
/// of a mid-function splice; and it has **exactly one xref** — the
/// drop site itself — so every call is a genuine drop with zero
/// false positives.
///
/// The v5-campaign RE pass names this function
/// `EntityDescription_GetExposedClientMethodByIndex`: it maps an
/// exposed method index through the `+0x20`/`+0x24` array to the
/// `MethodDescription` vector at `+0x0c` and returns the found
/// `MethodDescription*` in EAX (see
/// `docs/reverse-engineering/v5-campaign/worker-4b1.checkpoint.json`).
///
/// Signature: `__thiscall(void* method_table, uint method_index)
/// -> MethodDescription*`. The detour treats the returned pointer
/// as opaque and forwards it untouched.
///
/// **Thread:** runs on a Mercury *network* thread, not the main
/// game thread. The producer's
/// `try_emit` is non-blocking (bounded-channel `try_send`,
/// drop-on-full), so this is safe, but nothing here may touch
/// game state.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_ENTITY_METHOD_NOT_FOUND: usize = 0x01590f30;

/// Trampoline pointer for the entity-method drop path.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ENTITY_METHOD_NOT_FOUND_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_entity_method_not_found(producer: &Producer) {
    super::install_one(
        producer,
        "entity_method_not_found",
        ADDR_ENTITY_METHOD_NOT_FOUND,
        entity_method_not_found_detour as *mut c_void,
        &ENTITY_METHOD_NOT_FOUND_TRAMPOLINE,
    );
}

/// Detour for the entity-method **silent drop** path.
///
/// Emits one `client.dispatch.method_dropped` event carrying the
/// `method_index` the client could not route. This is the
/// client-side half of the round-trip oracle: the server can prove
/// it *sent* a method, but only this proves the real client failed
/// to *understand* it.
///
/// Emitted at `warn` and **unsampled**. A drop is a correctness
/// failure, not telemetry chatter — on a healthy session this
/// should be silent, so any volume here is itself the finding. If
/// a future client build turns out to drop routinely, add a
/// sampler rather than lowering the level.
///
/// Known-expected drops today: BlackMarket `onBM*` (method 90-95)
/// are parsed and flagged Exposed but were never bound into the
/// handler map, so they always land here until the runtime patch
/// in `black-market-client-window-patch.md` is applied.
///
/// **Hot-path discipline:** network thread. Non-blocking end to
/// end: the event builder heap-allocates (target string + field
/// map) but takes no game-state locks and reads no game state,
/// and `try_emit` is a bounded-channel `try_send` that drops on a
/// full queue rather than blocking the network thread.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn entity_method_not_found_detour(
    method_table: *mut c_void,
    method_index: u32,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.dispatch.method_dropped", "warn")
                    .field("method_index", serde_json::json!(method_index)),
            );
        }
    });

    if let Some(t) = ENTITY_METHOD_NOT_FOUND_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall-unwind" fn(*mut c_void, u32) -> *mut c_void =
            unsafe { std::mem::transmute(*t) };
        original(method_table, method_index)
    } else {
        // Trampoline missing — return null, the callee's own
        // "method not found" answer, which the drop site ignores
        // anyway.
        std::ptr::null_mut()
    }
}
