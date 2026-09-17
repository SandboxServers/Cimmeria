//! `GameBeing::onStateFieldUpdate` hook — one dispatcher hook
//! covers all 9 BSF_* state-flag transitions.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

/// `GameBeing::onStateFieldUpdate` — CME-subscribed dispatcher
/// that XOR-delta-decodes the 9 BSF_* state flags (Dead,
/// AutoCycling, Crouching, InCombat, PlayingMinigame, InStealth,
/// MovementLock, Walking, Holster). One hook here covers all
/// state-flag transitions. See `state-flag-broadcast.md`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_STATE_FIELD_UPDATE: usize = 0x00e01c90;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static STATE_FIELD_UPDATE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_state_field_update(producer: &Producer) {
    super::install_one(
        producer,
        "gamebeing_state_field_update",
        ADDR_STATE_FIELD_UPDATE,
        state_field_update_detour as *mut c_void,
        &STATE_FIELD_UPDATE_TRAMPOLINE,
    );
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
