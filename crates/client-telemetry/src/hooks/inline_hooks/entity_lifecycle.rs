//! Entity lifecycle + movement-correction hooks: two `ServerConnection`
//! message handlers on the main thread, both new for the seam survey
//! (`docs/reverse-engineering/findings/client-telemetry-seam-survey.md`).
//!
//! - [`install_forced_position`] — `ServerConnection_forcedPosition`
//!   (msg 0x31): the server told the client "you are not where you
//!   think you are." A player report of "I got yanked back" or a
//!   suspected desync between server and client position is this
//!   event firing (or not).
//! - [`install_create_base_player`] — `ServerConnection_createBasePlayer`
//!   (msg 0x05): the client's own base-entity id is assigned. Answers
//!   "did world entry actually create the local player" independent
//!   of anything CME-side.
//!
//! Both are resolved from existing RE findings
//! (`entity-creation-wire-formats.md`, `position-movement-wire-formats.md`)
//! with debug format strings embedded at the call site — a much
//! stronger signal than a bare name string, which is what misled the
//! three original `BWConnection` lifecycle anchors in the hookpoints
//! doc (see the seam survey's "dead ends" section). Both were
//! re-confirmed against the QA `SGW.exe` on 2026-09-28: real function
//! entries, `__thiscall(this, param_1)`, one stack argument each.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

/// `ServerConnection_forcedPosition` — see module docs. Body starts
/// `sub esp,0x10; push ebx; push ebp; push esi; mov esi,[esp+0x20];
/// push edi; mov edi,ecx`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_FORCED_POSITION: usize = 0x00dd9ee0;

/// `ServerConnection_createBasePlayer` — see module docs. Body starts
/// `sub esp,0x8; push ebx; push ebp; push esi; mov esi,[esp+0x18];
/// mov eax,[esi]`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_CREATE_BASE_PLAYER: usize = 0x00dddca0;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static FORCED_POSITION_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static CREATE_BASE_PLAYER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_forced_position(producer: &Producer) {
    super::install_one(
        producer,
        "forced_position",
        ADDR_FORCED_POSITION,
        forced_position_detour as *mut c_void,
        &FORCED_POSITION_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_create_base_player(producer: &Producer) {
    super::install_one(
        producer,
        "create_base_player",
        ADDR_CREATE_BASE_PLAYER,
        create_base_player_detour as *mut c_void,
        &CREATE_BASE_PLAYER_TRAMPOLINE,
    );
}

/// Detour for `ServerConnection_forcedPosition(this, args)`.
///
/// `args` is the pre-parsed 49-byte `forcedPosition` struct (see
/// `position-movement-wire-formats.md`); its first field is the
/// entity id. We read it **before** calling the trampoline, exactly
/// mirroring the original function's own first real read
/// (`ServerConnection__unknown_00e221a0(this_00, local_8, param_1)`,
/// which itself takes `param_1` — our `args` — as a plain input, not
/// a stream that advances). Unlike `createBasePlayer` below, nothing
/// here is a stream cursor, so a pre-read has no side effects to step
/// on.
///
/// **Not sampled** — a forced position correction is a rare,
/// meaningful event (the whole reason the hookpoints doc calls this
/// out as Tier 3 material); any volume is itself the finding.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn forced_position_detour(this: *mut c_void, args: *mut c_void) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            // SAFETY: `args` is the same pointer the original function
            // reads as its very first field access; guarded against
            // null regardless, since our read happens before the
            // original's own null handling would run.
            let entity_id = (!args.is_null()).then(|| unsafe { *(args as *const u32) });
            let mut event = crate::events::ClientNativeEvent::builder(
                "client.movement.forced_position",
                "info",
            );
            if let Some(id) = entity_id {
                event = event.field("entity_id", serde_json::json!(id));
            }
            p.try_emit(event);
        }
    });

    if let Some(t) = FORCED_POSITION_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(this, args);
    }
}

/// Detour for `ServerConnection_createBasePlayer(this, stream)`.
///
/// `stream` is a live `BinaryIStream`-style cursor: the original reads
/// the class id and entity id off it via a vtable call
/// (`(**(code**)(*param_1+4))(4)`) that **advances the stream**.
/// Calling that ourselves to peek the id first would double-consume
/// those bytes and corrupt the real parse that follows — so unlike
/// `forced_position_detour`, this one calls the trampoline **first**,
/// then reads the id back out of `ServerConnection::playerEntityID_`
/// (`this+0x16c`), which the original just finished writing
/// (`MOV [EDI+0x16c], EBP`). Same field `client.session.identity`'s
/// join-key read uses (`crate::identity`), just observed at the
/// moment it changes instead of read on demand.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn create_base_player_detour(
    this: *mut c_void,
    stream: *mut c_void,
) {
    if let Some(t) = CREATE_BASE_PLAYER_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(this, stream);
    }

    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            // SAFETY: reading a plain `u32` field on the same
            // `ServerConnection*` the trampoline call above just
            // finished writing through; `this` came from the engine's
            // own `ecx` for this call and is non-null by construction
            // of a virtual/thiscall dispatch, but checked anyway.
            if !this.is_null() {
                let entity_id = unsafe { *((this as *const u8).add(0x16c) as *const u32) };
                p.try_emit(
                    crate::events::ClientNativeEvent::builder(
                        "client.entity.create_base_player",
                        "info",
                    )
                    .field("entity_id", serde_json::json!(entity_id)),
                );
            }
        }
    });
}
