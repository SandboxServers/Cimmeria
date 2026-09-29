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
//! ## Late signals
//!
//! The DLL installs its hooks about 200 ms after `SGW.exe` starts, before
//! the client has created these signals: the first live run (2026-09-28)
//! logged `signal_missing` for both, and they stayed unsubscribed for the
//! whole session. A signal not found at install is retried from the
//! `FEngineLoop::Tick` detour ([`retry_pending`]), on the main thread, every
//! [`RETRY_EVERY_FRAMES`] frames, until it subscribes or
//! [`RETRY_GIVE_UP_FRAMES`] frames have passed.
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

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/// Frames between retries of a signal that was missing at install.
pub const RETRY_EVERY_FRAMES: u32 = 30;
/// Frames after which retrying stops (about 10 minutes at 60 fps).
pub const RETRY_GIVE_UP_FRAMES: u32 = 36_000;

/// Bit per hook still waiting for its signal (see [`HOOK_BITS`]).
static PENDING: AtomicU8 = AtomicU8::new(0);
/// Tick calls seen since install, while anything is pending.
static FRAMES: AtomicU32 = AtomicU32::new(0);

const MAP_LOAD_BIT: u8 = 1;
const READY_BIT: u8 = 2;
#[cfg(test)]
const HOOK_BITS: [u8; 2] = [MAP_LOAD_BIT, READY_BIT];

/// What the tick detour does on frame `frame` while signals are pending.
#[derive(Debug, PartialEq, Eq)]
pub enum RetryStep {
    Wait,
    Retry,
    GiveUp,
}

/// Retry cadence: every [`RETRY_EVERY_FRAMES`] frames, then give up once.
pub fn retry_step(frame: u32) -> RetryStep {
    if frame == RETRY_GIVE_UP_FRAMES {
        RetryStep::GiveUp
    } else if frame < RETRY_GIVE_UP_FRAMES && frame.is_multiple_of(RETRY_EVERY_FRAMES) {
        RetryStep::Retry
    } else {
        RetryStep::Wait
    }
}

/// Called from the `FEngineLoop::Tick` detour, on the main thread. Free
/// when nothing is pending.
pub fn retry_pending() {
    if PENDING.load(Ordering::Relaxed) == 0 {
        return;
    }
    let frame = FRAMES.fetch_add(1, Ordering::Relaxed) + 1;
    match retry_step(frame) {
        RetryStep::Wait => {}
        RetryStep::Retry => {
            #[cfg(all(target_os = "windows", target_arch = "x86"))]
            unsafe {
                retry_inner(frame);
            }
        }
        RetryStep::GiveUp => {
            #[cfg_attr(not(windows), allow(unused_variables))]
            let still = PENDING.swap(0, Ordering::Relaxed);
            #[cfg(windows)]
            if let Some(p) = crate::boot::producer() {
                super::emit_warn(
                    p,
                    "client.hooks.cme.gave_up",
                    [
                        ("pending_mask", serde_json::json!(still)),
                        ("frames", serde_json::json!(frame)),
                    ],
                );
            }
        }
    }
}

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
    let _ = LEAKED.set(leaked);

    let mut pending = 0u8;
    if !register_map_load(leaked, true, 0) {
        pending |= MAP_LOAD_BIT;
    }
    if !register_ready(leaked, true, 0) {
        pending |= READY_BIT;
    }
    PENDING.store(pending, Ordering::Relaxed);

    // Pair-completion event so SigNoz can pivot on whether the
    // CME subset of Phase 2 is fully wired.
    super::emit_info(
        &producer,
        "client.hooks.cme.install_complete",
        [("hook_count", serde_json::json!(2))],
    );
}

/// The producer the callbacks use, kept for [`retry_pending`].
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static LEAKED: std::sync::OnceLock<&'static Producer> = std::sync::OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn register_map_load(leaked: &'static Producer, first: bool, frame: u32) -> bool {
    register_hook(
        leaked,
        b"Event_NetIn_onClientMapLoad\0",
        "client.network.on_client_map_load",
        &MAP_LOAD_CB,
        on_client_map_load_thunk,
        first,
        frame,
    )
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn register_ready(leaked: &'static Producer, first: bool, frame: u32) -> bool {
    register_hook(
        leaked,
        b"Event_NetIn_onClientReady\0",
        "client.network.on_client_ready",
        &READY_CB,
        on_client_ready_thunk,
        first,
        frame,
    )
}

/// Retry every pending signal once. Main thread only (the tick detour).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn retry_inner(frame: u32) {
    let Some(leaked) = LEAKED.get().copied() else {
        return;
    };
    let pending = PENDING.load(Ordering::Relaxed);
    let mut still = pending;
    if pending & MAP_LOAD_BIT != 0 && register_map_load(leaked, false, frame) {
        still &= !MAP_LOAD_BIT;
    }
    if pending & READY_BIT != 0 && register_ready(leaked, false, frame) {
        still &= !READY_BIT;
    }
    PENDING.store(still, Ordering::Relaxed);
}

/// Look `signal_name` up and subscribe `callback`. Reports a miss only on
/// the `first` attempt (retries are silent until they succeed), and the
/// frame a late subscribe landed on.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn register_hook(
    leaked: &'static Producer,
    signal_name: &[u8],
    log_target: &'static str,
    callback: &'static CmeMemberCallback,
    _thunk: unsafe extern "thiscall" fn(*mut c_void, *mut c_void),
    first: bool,
    frame: u32,
) -> bool {
    let signal = lookup_by_name(signal_name);
    if signal.is_null() {
        if !first {
            return false;
        }
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
        return false;
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
            ("after_frames", serde_json::json!(frame)),
        ],
    );
    true
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
//
// Plain `thiscall`, not `thiscall-unwind`: these are callbacks, not
// detours. They call nothing in the game, so no C++ exception can pass
// through them, and the `catch_unwind` keeps a Rust panic from reaching
// the engine's signal emitter.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn on_client_map_load_thunk(_this: *mut c_void, _event_data: *mut c_void) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(ClientNativeEvent::builder(
                "client.network.on_client_map_load",
                "info",
            ));
        }
    });
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn on_client_ready_thunk(_this: *mut c_void, _event_data: *mut c_void) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(ClientNativeEvent::builder(
                "client.network.on_client_ready",
                "info",
            ));
            emit_session_identity(p);
        }
    });
}

/// The client↔server join key (see [`crate::identity`]). `onClientReady`
/// is the client's own ack that it is in world — the same trigger the
/// server pairs against `world_entry.init_player_state` — so it is the
/// natural place to read `ServerConnection::playerEntityID_` back out
/// and ship it once per world entry (including a relog: `onClientReady`
/// fires again then, with a fresh id).
///
/// Account name and server address are not included: neither survives
/// to client-side memory this crate can verify statically today — see
/// `crate::identity`'s module docs and
/// `docs/reverse-engineering/findings/client-telemetry-seam-survey.md`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn emit_session_identity(producer: &Producer) {
    let Some(player_entity_id) = crate::identity::read_local_player_entity_id() else {
        return;
    };
    producer.try_emit(
        ClientNativeEvent::builder("client.session.identity", "info")
            .field("player_entity_id", serde_json::json!(player_entity_id)),
    );
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[test]
    fn retries_on_the_cadence_then_gives_up_once() {
        assert_eq!(retry_step(1), RetryStep::Wait);
        assert_eq!(retry_step(RETRY_EVERY_FRAMES), RetryStep::Retry);
        assert_eq!(retry_step(RETRY_EVERY_FRAMES + 1), RetryStep::Wait);
        assert_eq!(retry_step(RETRY_GIVE_UP_FRAMES), RetryStep::GiveUp);
        assert_eq!(
            retry_step(RETRY_GIVE_UP_FRAMES + RETRY_EVERY_FRAMES),
            RetryStep::Wait
        );
    }

    #[test]
    fn hook_bits_are_distinct() {
        assert_eq!(HOOK_BITS[0] & HOOK_BITS[1], 0);
    }

    /// Nothing pending: the tick detour's call costs one load and does not
    /// count frames.
    #[test]
    fn nothing_pending_is_a_no_op() {
        PENDING.store(0, Ordering::Relaxed);
        let before = FRAMES.load(Ordering::Relaxed);
        retry_pending();
        assert_eq!(FRAMES.load(Ordering::Relaxed), before);
    }
}
