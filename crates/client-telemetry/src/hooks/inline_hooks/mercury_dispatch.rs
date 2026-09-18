//! Network-thread dispatch hooks: inbound Mercury packet dispatch
//! (`Mercury::Nub::handleMessage`) and the entity-method
//! **silent-drop oracle** — the client-side half of the round-trip
//! verification loop.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

/// Address of `Mercury::Nub::handleMessage` in SGW.exe. Stable
/// because ASLR is disabled (AtreaFixASLR.bat).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_HANDLE_MESSAGE: usize = 0x01b18be0;

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
/// game thread — same as `ADDR_HANDLE_MESSAGE`. The producer's
/// `try_emit` is non-blocking (bounded-channel `try_send`,
/// drop-on-full), so this is safe, but nothing here may touch
/// game state.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_ENTITY_METHOD_NOT_FOUND: usize = 0x01590f30;

/// Trampoline pointer for `Mercury::Nub::handleMessage`. Set by
/// MinHook at hook install time. Reading `.get()` gives us the
/// address of the original function prologue + JMP back to
/// address+N, callable as if it were the original.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static HANDLE_MESSAGE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Trampoline pointer for the entity-method drop path.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ENTITY_METHOD_NOT_FOUND_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_handle_message(producer: &Producer) {
    super::install_one(
        producer,
        "mercury_handle_message",
        ADDR_HANDLE_MESSAGE,
        handle_message_detour as *mut c_void,
        &HANDLE_MESSAGE_TRAMPOLINE,
    );
}

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
unsafe extern "thiscall" fn entity_method_not_found_detour(
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
        let original: unsafe extern "thiscall" fn(*mut c_void, u32) -> *mut c_void =
            unsafe { std::mem::transmute(*t) };
        original(method_table, method_index)
    } else {
        // Trampoline missing — return null, the callee's own
        // "method not found" answer, which the drop site ignores
        // anyway.
        std::ptr::null_mut()
    }
}
