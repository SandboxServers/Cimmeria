//! Pure fake-device regression tests for the render and lifecycle rules.

use std::cell::Cell;
use std::rc::Rc;

use super::render::{draw_panel, DeviceOps, Rect, Snapshot};
use super::{STATUS_GAP, STATUS_RECORDING, STATUS_TRACING};

struct FakeDevice {
    size: (u32, u32),
    calls: Vec<(Rect, u32)>,
    restored: Rc<Cell<u32>>,
    fail_draw: bool,
}

struct FakeSnapshot(Rc<Cell<u32>>);

impl Snapshot for FakeSnapshot {
    fn restore(&mut self) -> Result<(), ()> {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

impl DeviceOps for FakeDevice {
    type Saved = FakeSnapshot;

    fn viewport(&mut self) -> Result<(u32, u32), ()> {
        Ok(self.size)
    }

    fn snapshot(&mut self) -> Result<Self::Saved, ()> {
        Ok(FakeSnapshot(self.restored.clone()))
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
    }
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
    assert_eq!(draw_panel(&mut device, STATUS_GAP, || true), Err(()));
    assert_eq!(device.calls[0].1, 0xff10_1720);
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
