//! Pure fake-device regression tests for the render and lifecycle rules.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

use super::device::{FailureReason, Lifecycle};
use super::render::{draw_panel, DeviceOps, Rect, Snapshot};
use super::{STATUS_GAP, STATUS_RECORDING, STATUS_TRACING};
use crate::hooks::seams::d3d9::{install_overlay_vtable, OverlayInstall};

#[test]
fn concurrent_same_vtable_install_preserves_the_original() {
    let lock = Arc::new(Mutex::new(()));
    let owner = Arc::new(AtomicUsize::new(0));
    let installed = Arc::new(AtomicBool::new(false));
    let slot = Arc::new(AtomicUsize::new(0x1234));
    let original = Arc::new(AtomicUsize::new(0));
    let patches = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let (lock, owner, installed, slot, original, patches, barrier) = (
                &lock, &owner, &installed, &slot, &original, &patches, &barrier,
            );
            scope.spawn(move || {
                barrier.wait();
                assert_eq!(
                    install_overlay_vtable(lock, owner, installed, 0xa000, || {
                        patches.fetch_add(1, Ordering::Relaxed);
                        original.store(0x1234, Ordering::Release);
                        slot.compare_exchange(0x1234, 0x5678, Ordering::AcqRel, Ordering::Acquire)
                            .is_ok()
                    }),
                    OverlayInstall::Accepted
                );
            });
        }
    });
    assert_eq!(patches.load(Ordering::Acquire), 1);
    assert_eq!(original.load(Ordering::Acquire), 0x1234);
    assert_eq!(slot.load(Ordering::Acquire), 0x5678);
    for foreign in [0xb000, 0xc000, 0xd000, 0xe000] {
        assert_eq!(
            install_overlay_vtable(&lock, &owner, &installed, foreign, || panic!(
                "foreign vtable patched"
            )),
            OverlayInstall::Foreign
        );
    }
    assert_eq!(owner.load(Ordering::Acquire), 0xa000);
    assert_eq!(patches.load(Ordering::Acquire), 1);
}

struct FakeDevice {
    size: (u32, u32),
    calls: Vec<(Rect, u32)>,
    restored: Rc<Cell<u32>>,
    fail_draw: bool,
    fail_viewport: bool,
    fail_snapshot: bool,
    fail_restore: Rc<Cell<bool>>,
}

struct FakeSnapshot(Rc<Cell<u32>>, Rc<Cell<bool>>);

impl Snapshot for FakeSnapshot {
    fn restore(&mut self) -> Result<(), ()> {
        self.0.set(self.0.get() + 1);
        if self.1.get() {
            Err(())
        } else {
            Ok(())
        }
    }
}

impl DeviceOps for FakeDevice {
    type Saved = FakeSnapshot;

    fn viewport(&mut self) -> Result<(u32, u32), ()> {
        if self.fail_viewport {
            Err(())
        } else {
            Ok(self.size)
        }
    }

    fn snapshot(&mut self) -> Result<Self::Saved, ()> {
        if self.fail_snapshot {
            Err(())
        } else {
            Ok(FakeSnapshot(
                self.restored.clone(),
                self.fail_restore.clone(),
            ))
        }
    }

    fn clear(&mut self, rect: Rect, color: u32) -> Result<(), ()> {
        self.calls.push((rect, color));
        if self.fail_draw {
            Err(())
        } else {
            Ok(())
        }
    }
}

fn fake() -> FakeDevice {
    FakeDevice {
        size: (1280, 720),
        calls: Vec::new(),
        restored: Rc::new(Cell::new(0)),
        fail_draw: false,
        fail_viewport: false,
        fail_snapshot: false,
        fail_restore: Rc::new(Cell::new(false)),
    }
}

#[test]
fn auxiliary_same_vtable_probes_do_not_steal_the_drawing_device() {
    let state = Lifecycle::new();
    state.created(0xa, true);
    for _ in 0..5 {
        // The auxiliary can be created, pass a cooperative-level probe and
        // reset without ever submitting a frame through EndScene.
        state.created(0xb, true);
        state.created(0xb, true);
        state.reset(0xb, 0);
        assert!(state.enter(0xb).is_none());
        let frame = state.enter(0xa).expect("primary device stays drawable");
        let mut primary = fake();
        draw_panel(&mut primary, STATUS_TRACING, || true).unwrap();
        assert_eq!(primary.calls.len(), 3);
        assert_eq!(primary.restored.get(), 1);
        drop(frame);
    }
}

#[test]
fn successful_end_scene_selects_a_real_replacement_after_the_current_frame() {
    let state = Lifecycle::new();
    state.created(0xa, true);
    state.created(0xb, true);
    state.presented(0xb, -1);
    assert!(state.enter(0xb).is_none());
    let frame = state.enter(0xa).expect("primary frame");
    state.presented(0xb, 0);
    assert!(state.enter(0xb).is_none());
    drop(frame);

    state.presented(0xb, 0);
    assert!(state.enter(0xa).is_none());
    let frame = state.enter(0xb).expect("successful replacement frame");
    let mut replacement = fake();
    draw_panel(&mut replacement, STATUS_RECORDING, || true).unwrap();
    assert_eq!(replacement.calls.len(), 3);
    assert_eq!(replacement.restored.get(), 1);
    drop(frame);
}

#[test]
fn status_panel_is_bounded_opaque_and_restores_state() {
    let mut device = fake();
    draw_panel(&mut device, STATUS_TRACING | STATUS_RECORDING, || true).unwrap();
    assert_eq!(device.calls.len(), 3);
    assert!(device.calls.iter().all(|(_, color)| color >> 24 == 0xff));
    assert_eq!(device.calls[1].1, 0xff44_a6e0);
    assert_eq!(device.calls[2].1, 0xffe0_aa44);
    assert_eq!(device.restored.get(), 1);
}

#[test]
fn gap_and_failed_draw_restore_the_captured_state() {
    let mut device = fake();
    device.fail_draw = true;
    assert_eq!(
        draw_panel(&mut device, STATUS_GAP, || true),
        Err(FailureReason::Clear)
    );
    assert_eq!(device.calls[0].1, 0xff10_1720);
    assert_eq!(device.restored.get(), 1);
}

#[test]
fn each_failed_operation_has_a_reason_and_restore_failure_wins() {
    let mut device = fake();
    device.fail_viewport = true;
    assert_eq!(
        draw_panel(&mut device, 0, || true),
        Err(FailureReason::Viewport)
    );
    device.fail_viewport = false;
    device.fail_snapshot = true;
    assert_eq!(
        draw_panel(&mut device, 0, || true),
        Err(FailureReason::State)
    );
    device.fail_snapshot = false;
    device.fail_draw = true;
    device.fail_restore.set(true);
    assert_eq!(
        draw_panel(&mut device, 0, || true),
        Err(FailureReason::Restore)
    );
    assert_eq!(device.restored.get(), 1);
}

#[test]
fn tiny_viewport_and_expired_budget_do_not_draw() {
    let mut device = fake();
    device.size = (100, 100);
    draw_panel(&mut device, 0, || true).unwrap();
    assert!(device.calls.is_empty());
    assert_eq!(device.restored.get(), 0);
    device.size = (1280, 720);
    draw_panel(&mut device, 0, || false).unwrap();
    assert!(device.calls.is_empty());
}

#[test]
fn budget_expiry_after_first_rectangle_still_restores_state() {
    let mut device = fake();
    let checks = Cell::new(0);
    draw_panel(&mut device, STATUS_TRACING, || {
        let next = checks.get() + 1;
        checks.set(next);
        next <= 3
    })
    .unwrap();
    assert_eq!(device.calls.len(), 1);
    assert_eq!(device.restored.get(), 1);
}
