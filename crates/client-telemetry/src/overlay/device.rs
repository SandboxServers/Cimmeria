//! Device ownership, loss/recursion guards and bounded fault diagnostics.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};

const MAX_FAULTS: u8 = 3;
const OWNER_ABSENCE_PRESENTATIONS: u8 = 3;

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
    foreign_presentations: AtomicU8,
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
            foreign_presentations: AtomicU8::new(0),
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

    /// Keep a presenting owner until it is lost or absent for several EndScenes.
    pub(super) fn presented(&self, ptr: usize, hr: i32) {
        self.presented_after_lock(ptr, hr, || {});
    }

    // The callback gives the concurrency regression a precise point inside
    // the ownership guard; the production caller inlines an empty callback.
    fn presented_after_lock(&self, ptr: usize, hr: i32, after_lock: impl FnOnce()) {
        if ptr == 0 || hr < 0 || self.disabled.load(Ordering::Acquire) {
            return;
        }
        // A frame and a handoff must own the same guard. In particular, a
        // foreign EndScene cannot replace A while A still has a fault to log.
        if self
            .in_frame
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let _guard = FrameGuard { state: self };
        after_lock();
        if self.disabled.load(Ordering::Acquire) {
            return;
        }
        let owner = self.device.load(Ordering::Acquire);
        if owner == ptr {
            self.foreign_presentations.store(0, Ordering::Release);
            return;
        }
        // A successful auxiliary frame alone does not mean the selected
        // device retired. Count its absence only from real EndScene results.
        let absent = self.foreign_presentations.load(Ordering::Acquire);
        if self.ready.load(Ordering::Acquire)
            && absent.saturating_add(1) < OWNER_ABSENCE_PRESENTATIONS
        {
            self.foreign_presentations
                .store(absent.saturating_add(1), Ordering::Release);
            return;
        }
        self.ready.store(false, Ordering::Release);
        self.device.store(ptr, Ordering::Release);
        self.foreign_presentations.store(0, Ordering::Release);
        self.ready.store(true, Ordering::Release);
    }

    /// Mark only the selected device unavailable after a failed probe.
    pub(super) fn lost(&self, ptr: usize) {
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
        for _ in 0..OWNER_ABSENCE_PRESENTATIONS {
            state.presented(0xc, 0);
        }
        assert!(state.enter(0xc).is_some());
    }

    #[test]
    fn ownership_decision_and_panel_frame_share_one_guard() {
        use std::sync::{Arc, Barrier};

        let state = Arc::new(Lifecycle::new());
        state.created(0xa, true);
        state.created(0xb, true);
        for _ in 1..OWNER_ABSENCE_PRESENTATIONS {
            state.presented(0xb, 0);
        }
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let a_entered_during_handoff = std::thread::scope(|scope| {
            let (state, entered, release) = (&state, &entered, &release);
            scope.spawn(move || {
                state.presented_after_lock(0xb, 0, || {
                    entered.wait();
                    release.wait();
                });
            });
            entered.wait();
            let a_entered = state.enter(0xa).is_some();
            release.wait();
            a_entered
        });
        assert!(
            !a_entered_during_handoff,
            "A cannot start a frame mid-handoff"
        );
        assert!(state.enter(0xb).is_some());

        let active = Lifecycle::new();
        active.created(0xa, true);
        let frame = active.enter(0xa).expect("A owns a frame");
        std::thread::scope(|scope| {
            scope.spawn(|| active.presented(0xb, 0));
        });
        active.fault(0xa, FailureReason::Clear);
        drop(frame);
        assert_eq!(active.device.load(Ordering::Acquire), 0xa);
        assert_eq!(active.failures.load(Ordering::Acquire), 1);
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
