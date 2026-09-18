//! Per-frame engine drivers: `FEngineLoop::Tick` (main game loop)
//! and `FFullScreenMovieBink::Tick` (cinematic playback). Both are
//! heavily sampled — their value is cadence, not per-call detail.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

use crate::hooks::sampling::SamplingCounter;

/// Address of `FEngineLoop::Tick(void)` — UE3 main engine tick.
/// Resolved via RTTI walk from `.?AVResetDeferredUpdates@?CA@??Tick@FEngineLoop@@QAEXXZ@`
/// (the local RAII helper inside Tick) at 0x01d8f838. Body
/// 0x00416ec0 - 0x00417333.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_FENGINE_LOOP_TICK: usize = 0x00416ec0;

/// `FFullScreenMovieBink::Tick` — cinematic playback per-frame
/// driver. Vfunc 1 of FFullScreenMovieBink, 154 B body. The
/// presence of this firing distinguishes a real stall from
/// expected cinematic-bound frames.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_BINK_TICK: usize = 0x0050bbc0;

/// Trampoline pointer for `FEngineLoop::Tick`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static TICK_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static BINK_TICK_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Sampling counter for `FEngineLoop::Tick`. At 30-120 fps the
/// game ticks 30-120 Hz; 1/100 sampling yields ~0.3-1.2 emits/sec
/// — enough to see frame-time anomalies in SigNoz without burning
/// telemetry budget.
///
/// Only the detour reads this in production. On non-x86-Windows
/// targets the detour is cfg'd out and the static is reachable
/// only from the unit tests below; allow dead_code there so the
/// workspace lib build stays warning-clean.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static TICK_SAMPLER: SamplingCounter = SamplingCounter::new(100);

/// `Bink::Tick` runs at the cinematic's framerate (30 fps typical).
/// 1/30 sampling yields ~1 emit/sec — enough to detect "stuck on
/// a cinematic frame" without flooding.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static BINK_TICK_SAMPLER: SamplingCounter = SamplingCounter::new(30);

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_engine_tick(producer: &Producer) {
    super::install_one(
        producer,
        "fengineloop_tick",
        ADDR_FENGINE_LOOP_TICK,
        engine_tick_detour as *mut c_void,
        &TICK_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_bink_tick(producer: &Producer) {
    super::install_one(
        producer,
        "bink_tick",
        ADDR_BINK_TICK,
        bink_tick_detour as *mut c_void,
        &BINK_TICK_TRAMPOLINE,
    );
}

/// Detour for `FEngineLoop::Tick(void)`.
///
/// Signature: `extern "thiscall" fn(*mut FEngineLoop)` — `this` in
/// ECX, no stack args, no return value.
///
/// **Hot path discipline:** runs at 30-120 Hz on the main game
/// thread. Sampled at 1/100 via `TICK_SAMPLER` so the wire rate
/// is bounded to ~0.3-1.2 emits/sec.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn engine_tick_detour(this: *mut c_void) {
    let _ = std::panic::catch_unwind(|| {
        if TICK_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(crate::events::ClientNativeEvent::builder(
                    "client.engine.tick",
                    "debug",
                ));
            }
        }
    });

    if let Some(t) = TICK_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void) = unsafe { std::mem::transmute(*t) };
        original(this);
    }
    // No trampoline → don't call anything. Skipping a Tick is
    // safer than crashing the main thread, and the missing
    // trampoline implies install failed entirely (in which case
    // the hook patch was never applied either).
}

/// Detour for `FFullScreenMovieBink::Tick` — vtable slot 1.
///
/// Signature: `extern "thiscall" fn(*mut FFullScreenMovieBink, float DeltaSeconds)`.
///
/// **Hot path discipline:** fires every frame during cinematic
/// playback (30 fps typical). Sampled at 1/30 via `BINK_TICK_SAMPLER`
/// for ~1 emit/sec during cinematics.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn bink_tick_detour(this: *mut c_void, delta_seconds: f32) {
    let _ = std::panic::catch_unwind(|| {
        if BINK_TICK_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder("client.engine.bink_tick", "debug")
                        .field("delta_seconds", serde_json::json!(delta_seconds)),
                );
            }
        }
    });

    if let Some(t) = BINK_TICK_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, f32) =
            unsafe { std::mem::transmute(*t) };
        original(this, delta_seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sampling rates are load-bearing — they bound the wire cost
    /// of the hot-path hooks. Pin them so a careless edit doesn't
    /// 100× the telemetry volume.
    #[test]
    fn sampler_rates_are_bounded() {
        // 1/100 → ~0.3-1.2 emits/sec at 30-120 fps. Already
        // exercised by `samples_every_n` in the sampling module;
        // here we just pin the configured rate.
        let tick_emits: usize = (0..100).filter(|_| TICK_SAMPLER.should_emit()).count();
        // First 100 calls yield exactly 1 emit (n=0 triggers).
        assert_eq!(tick_emits, 1, "tick sampler should emit 1/100");

        // 1/30 — Bink frame cadence.
        let bink_emits: usize = (0..30).filter(|_| BINK_TICK_SAMPLER.should_emit()).count();
        assert_eq!(bink_emits, 1, "bink sampler should emit 1/30");
    }
}
