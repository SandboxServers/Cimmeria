//! Device ownership, loss/recursion guards and bounded fault diagnostics.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};

const MAX_FAULTS: u8 = 3;

/// One bounded reason code; emitted later from a non-render D3D callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum FailureReason {
    Patch = 1,
    Viewport = 2,
    State = 4,
    Clear = 8,
    Restore = 16,
    Panic = 32,
}

/// One-shot reason flags, total failure count and terminal shutoff state.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Diagnostics {
    pub reasons: u32,
    pub failures: u32,
    pub disabled: bool,
}

/// The one panel owner and its bounded draw/fault state.
pub(super) struct Lifecycle {
    device: AtomicUsize,
    ready: AtomicBool,
    in_frame: AtomicBool,
    faults: AtomicU8,
    disabled: AtomicBool,
    seen_reasons: AtomicU32,
    pending_reasons: AtomicU32,
    failures: AtomicU32,
    disabled_pending: AtomicBool,
}

impl Lifecycle {
    /// Start without a device; the first accepted creation can claim it.
    pub(super) const fn new() -> Self {
        Self {
            device: AtomicUsize::new(0),
            ready: AtomicBool::new(false),
            in_frame: AtomicBool::new(false),
            faults: AtomicU8::new(0),
            disabled: AtomicBool::new(false),
            seen_reasons: AtomicU32::new(0),
            pending_reasons: AtomicU32::new(0),
            failures: AtomicU32::new(0),
            disabled_pending: AtomicBool::new(false),
        }
    }

    /// Accept a first owner or refresh the current owner's successful probe.
    pub(super) fn created(&self, ptr: usize, vtable_accepted: bool) {
        // CreateDevice, Reset and TestCooperativeLevel also fire on auxiliary
        // devices using the patched vtable. Only the first accepted device
        // may claim an empty slot; later probes cannot displace its panel.
        if ptr != 0 && vtable_accepted && !self.disabled.load(Ordering::Acquire) {
            let owner = self
                .device
                .compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Acquire)
                .unwrap_or_else(|owner| owner);
            if owner == 0 || owner == ptr {
                self.ready.store(true, Ordering::Release);
            }
        }
    }

    /// Select a different device only after a successful real EndScene.
    pub(super) fn presented(&self, ptr: usize, hr: i32) {
        // A successful EndScene is evidence that this device actually draws.
        // Never switch ownership while another frame holds the state guard.
        if ptr != 0
            && hr >= 0
            && !self.disabled.load(Ordering::Acquire)
            && !self.in_frame.load(Ordering::Acquire)
            && self.device.load(Ordering::Acquire) != ptr
        {
            self.ready.store(false, Ordering::Release);
            self.device.store(ptr, Ordering::Release);
            self.ready.store(true, Ordering::Release);
        }
    }

    fn lost(&self, ptr: usize) {
        if self.device.load(Ordering::Acquire) == ptr {
            self.ready.store(false, Ordering::Release);
        }
    }

    /// Change readiness only for the selected device after its Reset.
    pub(super) fn reset(&self, ptr: usize, hr: i32) {
        if self.device.load(Ordering::Acquire) == ptr {
            self.ready.store(
                hr >= 0 && !self.disabled.load(Ordering::Acquire),
                Ordering::Release,
            );
        }
    }

    fn failure(&self, reason: FailureReason) {
        self.failures.fetch_add(1, Ordering::AcqRel);
        let bit = reason as u32;
        if self.seen_reasons.fetch_or(bit, Ordering::AcqRel) & bit == 0 {
            self.pending_reasons.fetch_or(bit, Ordering::Release);
        }
    }

    fn disable(&self) {
        if !self.disabled.swap(true, Ordering::AcqRel) {
            self.ready.store(false, Ordering::Release);
            self.disabled_pending.store(true, Ordering::Release);
        }
    }

    fn fault(&self, ptr: usize, reason: FailureReason) {
        if self.device.load(Ordering::Acquire) != ptr {
            return;
        }
        self.failure(reason);
        if self.faults.fetch_add(1, Ordering::AcqRel).saturating_add(1) >= MAX_FAULTS {
            self.disable();
        }
    }

    fn take_diagnostics(&self) -> Option<Diagnostics> {
        let reasons = self.pending_reasons.swap(0, Ordering::AcqRel);
        let disabled = self.disabled_pending.swap(false, Ordering::AcqRel);
        (reasons != 0 || disabled).then(|| Diagnostics {
            reasons,
            failures: self.failures.load(Ordering::Acquire),
            disabled,
        })
    }

    /// Enter one bounded panel frame on the selected ready device.
    pub(super) fn enter(&self, ptr: usize) -> Option<FrameGuard<'_>> {
        if ptr == 0
            || self.device.load(Ordering::Acquire) != ptr
            || !self.ready.load(Ordering::Acquire)
            || self.disabled.load(Ordering::Acquire)
        {
            return None;
        }
        self.in_frame
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        if self.device.load(Ordering::Acquire) != ptr
            || !self.ready.load(Ordering::Acquire)
            || self.disabled.load(Ordering::Acquire)
        {
            self.in_frame.store(false, Ordering::Release);
            return None;
        }
        Some(FrameGuard { state: self })
    }
}

#[cfg(all(windows, target_arch = "x86"))]
static LIFECYCLE: Lifecycle = Lifecycle::new();

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn on_created(ptr: usize, vtable_accepted: bool) {
    LIFECYCLE.created(ptr, vtable_accepted);
}

#[cfg(all(windows, target_arch = "x86"))]
/// Observe the original EndScene result before any ownership handoff.
pub(crate) fn on_presented(ptr: usize, hr: i32) {
    LIFECYCLE.presented(ptr, hr);
}

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn on_lost(ptr: usize) {
    LIFECYCLE.lost(ptr);
}

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn on_reset(ptr: usize, hr: i32) {
    LIFECYCLE.reset(ptr, hr);
}

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn on_fault(ptr: usize, reason: FailureReason) {
    LIFECYCLE.fault(ptr, reason);
}

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn on_install_failure() {
    LIFECYCLE.failure(FailureReason::Patch);
}

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn on_install_disabled() {
    LIFECYCLE.disable();
}

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn take_diagnostics() -> Option<Diagnostics> {
    LIFECYCLE.take_diagnostics()
}

pub(crate) struct FrameGuard<'a> {
    state: &'a Lifecycle,
}

impl Drop for FrameGuard<'_> {
    fn drop(&mut self) {
        self.state.in_frame.store(false, Ordering::Release);
    }
}

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn try_enter(ptr: usize) -> Option<FrameGuard<'static>> {
    LIFECYCLE.enter(ptr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_devices_only_accept_the_patched_vtable() {
        let state = Lifecycle::new();
        state.created(0xa, true);
        assert!(state.enter(0xa).is_some());
        for foreign in [0xb, 0xc, 0xd, 0xe] {
            state.created(foreign, false);
            assert!(state.enter(0xa).is_some());
            assert!(state.enter(foreign).is_none());
        }
        state.presented(0xc, 0);
        assert!(state.enter(0xc).is_some());
    }

    #[test]
    fn loss_reset_recursion_and_three_faults_gate_drawing() {
        let state = Lifecycle::new();
        state.created(0xa, true);
        let frame = state.enter(0xa).unwrap();
        assert!(state.enter(0xa).is_none());
        drop(frame);
        state.lost(0xa);
        assert!(state.enter(0xa).is_none());
        state.reset(0xa, -1);
        assert!(state.enter(0xa).is_none());
        state.reset(0xa, 0);
        assert!(state.enter(0xa).is_some());
        for _ in 0..3 {
            state.fault(0xa, FailureReason::Clear);
        }
        assert!(state.enter(0xa).is_none());
        state.reset(0xa, 0);
        assert!(state.enter(0xa).is_none());
        assert_eq!(
            state.take_diagnostics(),
            Some(Diagnostics {
                reasons: FailureReason::Clear as u32,
                failures: 3,
                disabled: true,
            })
        );
        assert_eq!(state.take_diagnostics(), None);
    }

    #[test]
    fn failure_reasons_emit_once_and_counts_are_bounded() {
        assert_eq!(FailureReason::Panic as u32, 32);
        let state = Lifecycle::new();
        state.failure(FailureReason::Patch);
        state.failure(FailureReason::Patch);
        assert_eq!(
            state.take_diagnostics(),
            Some(Diagnostics {
                reasons: FailureReason::Patch as u32,
                failures: 2,
                disabled: false,
            })
        );
        state.failure(FailureReason::Patch);
        assert_eq!(state.take_diagnostics(), None);
        state.failure(FailureReason::Viewport);
        assert_eq!(
            state.take_diagnostics(),
            Some(Diagnostics {
                reasons: FailureReason::Viewport as u32,
                failures: 4,
                disabled: false,
            })
        );
    }
}
