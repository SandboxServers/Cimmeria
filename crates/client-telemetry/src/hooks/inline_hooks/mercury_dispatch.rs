//! Network-thread dispatch hook: the entity-method **silent-drop
//! oracle** — the client-side half of the round-trip verification
//! loop.
//!
//! The inbound-packet hook on `Mercury::Nub::handleMessage` that used
//! to live here was removed: its address (`0x01b18be0`) is a log
//! string, not code, and the real function takes four stack
//! arguments. Re-adding it is #989.

use crate::hooks::entity_trace::{Ctx, Fields};

/// The fields of `client.dispatch.method_dropped`: the method index, and the
/// entity, type and message id when the drop happened inside a dispatch the
/// entity detours tagged.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
pub(crate) fn dropped_fields(method_index: u32, ctx: Option<Ctx>) -> Fields {
    let mut f: Fields = vec![("method_index", serde_json::json!(method_index))];
    if let Some(c) = ctx {
        f.push(("entity_id", serde_json::json!(c.entity_id)));
        if let Some(t) = c.type_id {
            f.push(("type_id", serde_json::json!(t)));
        }
        if c.msg_id != 0 {
            f.push(("msg_id", serde_json::json!(c.msg_id)));
        }
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

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
        // The dispatch context (set by the `onEntityMethod` / queue-replay
        // detours) names the entity the dropped method was for.
        crate::hooks::emit::emit(
            "client.dispatch.method_dropped",
            "warn",
            dropped_fields(method_index, crate::hooks::entity_trace::current_ctx()),
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drop_names_the_entity_when_the_dispatch_is_tagged() {
        let f = dropped_fields(
            91,
            Some(Ctx {
                entity_id: 7,
                type_id: Some(3),
                msg_id: 0x3e,
            }),
        );
        assert!(f.contains(&("method_index", serde_json::json!(91))));
        assert!(f.contains(&("entity_id", serde_json::json!(7))));
        assert!(f.contains(&("type_id", serde_json::json!(3))));
        assert!(f.contains(&("msg_id", serde_json::json!(0x3e))));
        assert_eq!(dropped_fields(91, None).len(), 1, "no context, no entity");
    }
}
