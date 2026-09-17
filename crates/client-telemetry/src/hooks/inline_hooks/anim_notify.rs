//! `USGWAnimNotify_Event::Notify` hooks — variants A and B share
//! one sampling counter so the combined emit rate stays 1/100.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

use crate::hooks::sampling::SamplingCounter;

/// `USGWAnimNotify_Event::Notify` variant A — 1.2 KB body.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_ANIM_NOTIFY_A: usize = 0x00e974b0;

/// `USGWAnimNotify_Event::Notify` variant B — 450 B body, likely
/// a specialized notify path (per-tick or per-key variant).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_ANIM_NOTIFY_B: usize = 0x00e97070;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ANIM_NOTIFY_A_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ANIM_NOTIFY_B_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// `AnimNotify::Notify` fires on every animation notify event —
/// can fire several times per actor per frame during combat
/// animations. 1/100 sampling keeps emit rate manageable.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static ANIM_NOTIFY_SAMPLER: SamplingCounter = SamplingCounter::new(100);

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_anim_notify_a(producer: &Producer) {
    super::install_one(
        producer,
        "anim_notify_a",
        ADDR_ANIM_NOTIFY_A,
        anim_notify_a_detour as *mut c_void,
        &ANIM_NOTIFY_A_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_anim_notify_b(producer: &Producer) {
    super::install_one(
        producer,
        "anim_notify_b",
        ADDR_ANIM_NOTIFY_B,
        anim_notify_b_detour as *mut c_void,
        &ANIM_NOTIFY_B_TRAMPOLINE,
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the shared A+B sampling rate — it bounds the wire cost
    /// of the anim-notify hooks.
    #[test]
    fn sampler_rate_is_bounded() {
        // 1/100 shared between anim variants A + B.
        let anim_emits: usize = (0..100)
            .filter(|_| ANIM_NOTIFY_SAMPLER.should_emit())
            .count();
        assert_eq!(anim_emits, 1, "anim sampler should emit 1/100");
    }
}
