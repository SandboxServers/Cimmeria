//! Device lifecycle, recursion guard and repeated-fault shutoff.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

const MAX_FAULTS: u8 = 3;
static DEVICE: AtomicUsize = AtomicUsize::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static IN_FRAME: AtomicBool = AtomicBool::new(false);
static FAULTS: AtomicU8 = AtomicU8::new(0);
static DISABLED: AtomicBool = AtomicBool::new(false);

/// The first created D3D9 device owns this render hook; new vtables are not
/// patched onto one shared original-function pointer.
pub(crate) fn on_created(ptr: usize) {
    if ptr == 0 || DISABLED.load(Ordering::Acquire) {
        return;
    }
    READY.store(false, Ordering::Release);
    DEVICE.store(ptr, Ordering::Release);
    READY.store(true, Ordering::Release);
}

/// Suspend drawing when the client reports a lost device.
pub(crate) fn on_lost(ptr: usize) {
    if DEVICE.load(Ordering::Acquire) == ptr {
        READY.store(false, Ordering::Release);
    }
}

/// A successful Reset allows drawing again; failed Reset leaves it suspended.
pub(crate) fn on_reset(ptr: usize, hr: i32) {
    if DEVICE.load(Ordering::Acquire) == ptr {
        READY.store(
            hr >= 0 && !DISABLED.load(Ordering::Acquire),
            Ordering::Release,
        );
    }
}

/// Disable this overlay after three rendering faults in one process.
pub(crate) fn on_fault(ptr: usize) {
    if DEVICE.load(Ordering::Acquire) == ptr
        && FAULTS.fetch_add(1, Ordering::AcqRel).saturating_add(1) >= MAX_FAULTS
    {
        DISABLED.store(true, Ordering::Release);
        READY.store(false, Ordering::Release);
    }
}

pub(crate) struct FrameGuard;

impl Drop for FrameGuard {
    fn drop(&mut self) {
        IN_FRAME.store(false, Ordering::Release);
    }
}

pub(crate) fn try_enter(ptr: usize) -> Option<FrameGuard> {
    if ptr == 0
        || DEVICE.load(Ordering::Acquire) != ptr
        || !READY.load(Ordering::Acquire)
        || DISABLED.load(Ordering::Acquire)
    {
        return None;
    }
    IN_FRAME
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .ok()?;
    if !READY.load(Ordering::Acquire) || DISABLED.load(Ordering::Acquire) {
        IN_FRAME.store(false, Ordering::Release);
        return None;
    }
    Some(FrameGuard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lost_reset_recursion_and_three_faults_gate_drawing() {
        let ptr = 0x1234;
        assert!(try_enter(ptr).is_none());
        on_created(ptr);
        let frame = try_enter(ptr).expect("created device is ready");
        assert!(try_enter(ptr).is_none(), "a nested EndScene cannot draw");
        drop(frame);
        on_lost(ptr);
        assert!(try_enter(ptr).is_none());
        on_reset(ptr, -1);
        assert!(try_enter(ptr).is_none(), "failed Reset keeps it lost");
        on_reset(ptr, 0);
        assert!(try_enter(ptr).is_some());
        for _ in 0..3 {
            on_fault(ptr);
        }
        assert!(try_enter(ptr).is_none(), "fault shutoff persists");
        on_reset(ptr, 0);
        assert!(try_enter(ptr).is_none(), "Reset cannot re-enable shutoff");
    }
}
