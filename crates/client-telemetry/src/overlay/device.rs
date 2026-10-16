//! Device ownership, loss/recursion guards and bounded fault diagnostics.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

const MAX_FAULTS: u8 = 3;
const OWNER_INACTIVITY_MS: u64 = 2_000;

fn monotonic_ms() -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    u64::try_from(EPOCH.get_or_init(Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
}

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
    // Zero means the first owner has not published its starting timestamp.
    last_owner_presentation_ms: AtomicU64,
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
            last_owner_presentation_ms: AtomicU64::new(0),
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
        self.created_at(ptr, vtable_accepted, monotonic_ms());
    }

    fn created_at(&self, ptr: usize, vtable_accepted: bool, now_ms: u64) {
        self.created_after_claim(ptr, vtable_accepted, now_ms, || {});
    }

    // The callback lets the regression pause after ownership is published,
    // before the initial timestamp, without changing the production path.
    fn created_after_claim(
        &self,
        ptr: usize,
        vtable_accepted: bool,
        now_ms: u64,
        after_claim: impl FnOnce(),
    ) {
        // CreateDevice, Reset and TestCooperativeLevel also fire on auxiliary
        // devices using the patched vtable. Only the first accepted device
        // may claim an empty slot; later probes cannot displace its panel.
        if ptr != 0 && vtable_accepted && !self.disabled.load(Ordering::Acquire) {
            match self
                .device
                .compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    after_claim();
                    self.last_owner_presentation_ms
                        .fetch_max(now_ms.saturating_add(1), Ordering::AcqRel);
                    if self.device.load(Ordering::Acquire) == ptr {
                        self.ready.store(true, Ordering::Release);
                    }
                }
                Err(owner) if owner == ptr => self.ready.store(true, Ordering::Release),
                Err(_) => {}
            }
        }
    }

    /// Keep a presenting owner until it is lost or inactive for two seconds.
    pub(super) fn presented(&self, ptr: usize, hr: i32) {
        self.presented_at(ptr, hr, monotonic_ms());
    }

    /// Observe a presentation at a supplied monotonic time for lifecycle tests.
    pub(super) fn presented_at(&self, ptr: usize, hr: i32, now_ms: u64) {
        self.presented_after_lock(ptr, hr, now_ms, || {});
    }

    // The callback gives the concurrency regression a precise point inside
    // the ownership guard; the production caller inlines an empty callback.
    fn presented_after_lock(&self, ptr: usize, hr: i32, now_ms: u64, after_lock: impl FnOnce()) {
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
            self.last_owner_presentation_ms
                .fetch_max(now_ms.saturating_add(1), Ordering::AcqRel);
            return;
        }
        // Multiple auxiliary EndScenes can happen between two healthy owner
        // frames. Their call count says nothing about owner inactivity.
        let last_owner_ms = self.last_owner_presentation_ms.load(Ordering::Acquire);
        let ready = self.ready.load(Ordering::Acquire);
        if ready && last_owner_ms == 0 {
            return;
        }
        if ready && now_ms.saturating_sub(last_owner_ms - 1) < OWNER_INACTIVITY_MS {
            return;
        }
        self.ready.store(false, Ordering::Release);
        self.device.store(ptr, Ordering::Release);
        self.last_owner_presentation_ms
            .fetch_max(now_ms.saturating_add(1), Ordering::AcqRel);
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
        state.presented_at(0xa, 0, 100);
        state.presented_at(0xc, 0, 2_099);
        assert!(state.enter(0xc).is_none());
        state.presented_at(0xc, 0, 2_100);
        assert!(state.enter(0xc).is_some());
    }

    #[test]
    fn foreign_cannot_take_owner_before_initial_timestamp_is_published() {
        let state = Lifecycle::new();
        // A concurrent same-device probe can mark ready between the first
        // owner's pointer claim and its timestamp publication.
        state.device.store(0xa, Ordering::Release);
        state.ready.store(true, Ordering::Release);
        state.presented_at(0xb, 0, OWNER_INACTIVITY_MS + 1);
        assert_eq!(state.device.load(Ordering::Acquire), 0xa);
        state.last_owner_presentation_ms.store(2, Ordering::Release);
        state.presented_at(0xb, 0, OWNER_INACTIVITY_MS + 1);
        assert_eq!(state.device.load(Ordering::Acquire), 0xb);
    }

    #[test]
    fn delayed_creation_timestamp_cannot_age_a_newer_owner_frame() {
        use std::sync::{Arc, Barrier};

        let state = Arc::new(Lifecycle::new());
        let claimed = Arc::new(Barrier::new(2));
        let resume = Arc::new(Barrier::new(2));
        std::thread::scope(|scope| {
            let (state, claimed, resume) = (&state, &claimed, &resume);
            scope.spawn(move || {
                state.created_after_claim(0xa, true, 0, || {
                    claimed.wait();
                    resume.wait();
                });
            });
            claimed.wait();
            state.presented_at(0xa, 0, 1_999);
            resume.wait();
        });
        state.presented_at(0xb, 0, 2_000);
        assert_eq!(state.device.load(Ordering::Acquire), 0xa);
        assert!(state.enter(0xa).is_some());
    }

    #[test]
    fn delayed_owner_callback_cannot_age_a_newer_owner_frame() {
        let state = Lifecycle::new();
        state.created_at(0xa, true, 0);
        state.presented_at(0xa, 0, 1_999);
        // The callback took its timestamp before the newer callback but
        // reached the frame guard only after the newer callback finished.
        state.presented_at(0xa, 0, 1_000);
        state.presented_at(0xb, 0, 2_000);
        assert_eq!(state.device.load(Ordering::Acquire), 0xa);
    }

    #[test]
    fn ownership_decision_and_panel_frame_share_one_guard() {
        use std::sync::{Arc, Barrier};

        let state = Arc::new(Lifecycle::new());
        state.created_at(0xa, true, 0);
        state.created_at(0xb, true, 0);
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let a_entered_during_handoff = std::thread::scope(|scope| {
            let (state, entered, release) = (&state, &entered, &release);
            scope.spawn(move || {
                state.presented_after_lock(0xb, 0, OWNER_INACTIVITY_MS, || {
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
