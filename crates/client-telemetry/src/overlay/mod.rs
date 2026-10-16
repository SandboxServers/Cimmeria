//! Lab-only, bounded D3D9 overlay. The companion window owns detailed UI.

pub mod device;
mod render;

#[cfg(test)]
mod render_tests;

#[cfg(all(windows, target_arch = "x86"))]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(all(windows, target_arch = "x86"))]
use std::time::{Duration, Instant};

/// Status bits atomically published by lab controls; zero means unknown.
pub const STATUS_TRACING: u32 = 1;
/// A reproduction or flight recorder is active.
pub const STATUS_RECORDING: u32 = 2;
/// A trace or recording has a known evidence gap.
pub const STATUS_GAP: u32 = 4;

#[cfg(all(windows, target_arch = "x86"))]
static STATUS: AtomicU32 = AtomicU32::new(0);
#[cfg(all(windows, target_arch = "x86"))]
const FRAME_BUDGET: Duration = Duration::from_millis(1);

/// Publish only coarse status bits. No command or bridge token enters pixels.
#[cfg(all(windows, target_arch = "x86"))]
#[allow(dead_code)] // DW-18 publishes these bits when controls land.
pub fn publish_status(bits: u32) {
    STATUS.store(
        bits & (STATUS_TRACING | STATUS_RECORDING | STATUS_GAP),
        Ordering::Release,
    );
}

/// Draw on the D3D device thread immediately before the client's EndScene.
#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn render_frame(device_ptr: usize) {
    if crate::bridge::bridge_epoch().is_none() {
        return;
    }
    let Some(_frame) = device::try_enter(device_ptr) else {
        return;
    };
    let start = Instant::now();
    let mut ops = render::D3dDevice::new(device_ptr);
    if render::draw_panel(&mut ops, STATUS.load(Ordering::Acquire), || {
        start.elapsed() <= FRAME_BUDGET
    })
    .is_err()
    {
        device::on_fault(device_ptr);
    }
}
