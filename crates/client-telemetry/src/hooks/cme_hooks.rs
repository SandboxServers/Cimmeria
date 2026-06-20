//! CME EventSignal hooks — Tier 1.
//!
//! Subscribes static `CmeMemberCallback` objects to specific signals
//! in SGW.exe so the host's own dispatch trampoline routes through
//! our extern "thiscall" handlers. No code patching — just a set
//! insert in the host's subscriber map.
//!
//! ## Signals subscribed in this PR
//!
//! - `Event_NetIn_onClientMapLoad` — fired when the client finishes
//!   loading a map (cell-side world entry sync point).
//! - `Event_NetIn_onClientReady` — fired when the client acks
//!   readiness to the server (the trigger that lets the server run
//!   `handle_init_player_state`).
//!
//! Together these two events form the back half of the world-entry
//! handshake — pairing them with the server's
//! `world_entry.init_player_state` span makes the cold-relog freeze
//! investigation actionable.
//!
//! ## Lifetime
//!
//! Each `CmeMemberCallback` is a `static` in `.data` (process-
//! lifetime). The producer handle stored at `this_ptr` is a leaked
//! `Box<Producer>` so the producer outlives the static reference
//! to it. This is OK because the DLL never unloads — see
//! `boot.rs` for the no-detach contract.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::events::ClientNativeEvent;
use crate::queue::Producer;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::cme::{
    lookup_by_name, subscribe, CmeMemberCallback, FakeVtable, ADDR_INVOKE_MEMBER_CALLBACK,
};

/// Install both CME hooks. Reports success/failure as one event
/// per hook in the queue so we can see in SigNoz which subscribes
/// landed.
pub fn install(_producer: Producer) {
    // Active only on the real DLL target (Windows x86). Other
    // targets — Linux unit tests, Windows x64 workspace check —
    // get a no-op because `extern "thiscall"` isn't a valid ABI
    // off x86 and the SGW.exe addresses don't exist anyway.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        install_inner(_producer);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_inner(producer: Producer) {
    // Leak the producer into a stable address; the static
    // CmeMemberCallback objects point at it via `this_ptr`. Each
    // hook's static is constructed by `register_hook` below.
    let leaked: &'static Producer = Box::leak(Box::new(producer.clone()));

    register_hook(
        leaked,
        b"Event_NetIn_onClientMapLoad\0",
        "client.network.on_client_map_load",
        &MAP_LOAD_CB,
        on_client_map_load_thunk,
    );
    register_hook(
        leaked,
        b"Event_NetIn_onClientReady\0",
        "client.network.on_client_ready",
        &READY_CB,
        on_client_ready_thunk,
    );

    // Pair-completion event so SigNoz can pivot on whether the
    // CME subset of Phase 2 is fully wired.
    super::emit_info(
        &producer,
        "client.hooks.cme.install_complete",
        [("hook_count", serde_json::json!(2))],
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn register_hook(
    leaked: &'static Producer,
    signal_name: &[u8],
    log_target: &'static str,
    callback: &'static CmeMemberCallback,
    _thunk: unsafe extern "thiscall" fn(*mut c_void, *mut c_void),
) {
    let signal = lookup_by_name(signal_name);
    if signal.is_null() {
        super::emit_warn(
            leaked,
            "client.hooks.cme.signal_missing",
            [(
                "signal",
                serde_json::Value::String(
                    std::str::from_utf8(&signal_name[..signal_name.len() - 1])
                        .unwrap_or("")
                        .to_string(),
                ),
            )],
        );
        return;
    }
    subscribe(signal, callback);
    super::emit_info(
        leaked,
        "client.hooks.cme.subscribed",
        [
            (
                "signal",
                serde_json::Value::String(
                    std::str::from_utf8(&signal_name[..signal_name.len() - 1])
                        .unwrap_or("")
                        .to_string(),
                ),
            ),
            ("log_target", serde_json::Value::String(log_target.into())),
        ],
    );
}

// ─── Static vtables + callbacks ─────────────────────────────────
//
// One vtable + one CmeMemberCallback per hook. The vtable's slot
// 5 (`invoke_member_callback`) MUST point to SGW.exe's own
// dispatch trampoline at ADDR_INVOKE_MEMBER_CALLBACK — that
// function reads `this_ptr` + `method_ptr` from the
// CmeMemberCallback and tail-calls into the method, which is our
// thunk.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static MAP_LOAD_VTABLE: FakeVtable = FakeVtable {
    destructor: placeholder_slot_static(),
    slot_1: placeholder_slot_static(),
    slot_2: placeholder_slot_static(),
    slot_3: placeholder_slot_static(),
    slot_4: placeholder_slot_static(),
    invoke_member_callback: ADDR_INVOKE_MEMBER_CALLBACK as *const c_void,
};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static READY_VTABLE: FakeVtable = FakeVtable {
    destructor: placeholder_slot_static(),
    slot_1: placeholder_slot_static(),
    slot_2: placeholder_slot_static(),
    slot_3: placeholder_slot_static(),
    slot_4: placeholder_slot_static(),
    invoke_member_callback: ADDR_INVOKE_MEMBER_CALLBACK as *const c_void,
};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static MAP_LOAD_CB: CmeMemberCallback = CmeMemberCallback {
    vtable: &MAP_LOAD_VTABLE,
    // Static-init: producer pointer gets filled in by
    // `install_inner` via a `static mut` shadow — kept simple by
    // routing through `MAP_LOAD_PRODUCER` below in the thunk.
    this_ptr: std::ptr::null(),
    method_ptr: on_client_map_load_thunk as *const c_void,
};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static READY_CB: CmeMemberCallback = CmeMemberCallback {
    vtable: &READY_VTABLE,
    this_ptr: std::ptr::null(),
    method_ptr: on_client_ready_thunk as *const c_void,
};

/// `const fn` wrapper around `placeholder_slot()` so the FakeVtable
/// statics can be initialized in const context. `placeholder_slot()`
/// itself returns a function pointer cast which isn't const-eval'able
/// directly; we route through `null_mut()` as the placeholder since
/// the host never reads these slots for static subscribers (the
/// no-unsubscribe contract).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const fn placeholder_slot_static() -> *const c_void {
    // Per the no-unsubscribe contract documented in
    // `docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`,
    // the host never reads slots 0-4 for our static subscribers.
    // A null pointer here would crash IF the host ever vcalled one
    // of these slots — but it doesn't. Defence-in-depth would point
    // them all at a real noop function, but `placeholder_slot()`'s
    // function-pointer cast isn't usable in a const-fn vtable
    // initializer. We accept the null + document the assumption.
    std::ptr::null()
}

// ─── Producer access from thunks ────────────────────────────────
//
// Thunks run inside SGW.exe's threads; they need to reach the
// process-global Producer to emit events. We use `boot::producer()`
// (returns `Option<&'static Producer>`) rather than the leaked
// reference in install_inner — the producer handle is the same
// either way (cloned from the same OnceLock), but the
// `boot::producer()` route is what other future hooks will share.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn on_client_map_load_thunk(_this: *mut c_void, _event_data: *mut c_void) {
    if let Some(p) = crate::boot::producer() {
        p.try_emit(ClientNativeEvent::builder(
            "client.network.on_client_map_load",
            "info",
        ));
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn on_client_ready_thunk(_this: *mut c_void, _event_data: *mut c_void) {
    if let Some(p) = crate::boot::producer() {
        p.try_emit(ClientNativeEvent::builder(
            "client.network.on_client_ready",
            "info",
        ));
    }
}
