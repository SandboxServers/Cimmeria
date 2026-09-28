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
/// Signature: `thiscall fn(*mut GameBeing, *mut EventData, u32)`. The
/// function returns with `ret 8` at all three exits, so it takes two
/// stack arguments. What the second one is has not been resolved; the
/// detour passes it through untouched.
///
/// **Hot path discipline:** state-flag changes (combat enter/exit,
/// stealth toggle, holster) are infrequent (a few per minute per
/// player typically). No sampling — emit 1/1.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn state_field_update_detour(
    this: *mut c_void,
    event_data: *mut c_void,
    arg2: u32,
) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(crate::events::ClientNativeEvent::builder(
                "client.state.field_update",
                "debug",
            ));
        }
    });

    if let Some(t) = STATE_FIELD_UPDATE_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, u32) =
            unsafe { std::mem::transmute(*t) };
        original(this, event_data, arg2);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    use super::*;

    /// The detour hands both stack arguments to the original (it pops 8
    /// bytes; a one-argument detour popped 4), and an exception from the
    /// original unwinds through it (#915).
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn forwards_both_arguments_and_lets_exceptions_through() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SEEN: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            event: *mut c_void,
            arg2: u32,
        ) {
            for (slot, v) in SEEN
                .iter()
                .zip([this as usize, event as usize, arg2 as usize])
            {
                slot.store(v, Ordering::SeqCst);
            }
            panic!("engine error");
        }
        STATE_FIELD_UPDATE_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        let caught = std::panic::catch_unwind(|| unsafe {
            state_field_update_detour(0x1111 as *mut c_void, 0x2222 as *mut c_void, 0x3333)
        });
        assert!(caught.is_err());
        let seen: Vec<usize> = SEEN.iter().map(|s| s.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, [0x1111, 0x2222, 0x3333]);
    }
}
