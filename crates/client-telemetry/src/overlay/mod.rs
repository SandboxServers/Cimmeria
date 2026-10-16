//! Lab-only, bounded D3D9 overlay. The companion window owns detailed UI.

pub mod device;
mod render;

#[cfg(test)]
mod render_tests;

#[cfg(all(windows, target_arch = "x86"))]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(all(windows, target_arch = "x86"))]
use std::sync::OnceLock;
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
static DRAIN: OnceLock<bool> = OnceLock::new();
#[cfg(all(windows, target_arch = "x86"))]
const FRAME_BUDGET: Duration = Duration::from_millis(1);

/// Start one non-render diagnostic drain for this lab process. EndScene only
/// flips atomics; JSON allocation and queue publication happen here.
#[cfg(all(windows, target_arch = "x86"))]
pub(crate) fn start_diagnostic_drain() -> bool {
    *DRAIN.get_or_init(|| {
        let spawned = std::thread::Builder::new()
            .name("cimmeria-overlay-diagnostics".into())
            .spawn(|| loop {
                std::thread::sleep(Duration::from_millis(200));
                drain_once();
                if crate::shutdown::global().stop_requested() {
                    break;
                }
            });
        if spawned.is_err() {
            device::on_install_failure();
            device::on_install_disabled();
            drain_once();
            return false;
        }
        true
    })
}

#[cfg(all(windows, target_arch = "x86"))]
fn drain_once() {
    let Some(d) = device::take_diagnostics() else {
        return;
    };
    crate::hooks::sinks::emit::emit(
        "client.gfx.overlay_fault",
        "warn",
        "gfx.overlay_fault",
        diagnostic_fields(d),
    );
}

#[cfg(any(all(windows, target_arch = "x86"), test))]
fn diagnostic_fields(d: device::Diagnostics) -> crate::hooks::sinks::emit::Fields {
    let reasons: Vec<&'static str> = [
        (device::FailureReason::Patch, "patch"),
        (device::FailureReason::Viewport, "viewport"),
        (device::FailureReason::State, "state"),
        (device::FailureReason::Clear, "clear"),
        (device::FailureReason::Restore, "restore"),
        (device::FailureReason::Panic, "panic"),
    ]
    .into_iter()
    .filter_map(|(reason, name)| (d.reasons & reason as u32 != 0).then_some(name))
    .collect();
    vec![
        ("reasons", serde_json::json!(reasons)),
        ("failures", serde_json::json!(d.failures)),
        ("disabled", serde_json::json!(d.disabled)),
    ]
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn the_non_render_drain_keeps_reason_and_terminal_count() {
        let fields = diagnostic_fields(device::Diagnostics {
            reasons: device::FailureReason::Patch as u32 | device::FailureReason::Restore as u32,
            failures: 3,
            disabled: true,
        });
        assert_eq!(fields[0].1, serde_json::json!(["patch", "restore"]));
        assert_eq!(fields[1].1, serde_json::json!(3));
        assert_eq!(fields[2].1, serde_json::json!(true));
    }
}

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
    if let Err(reason) = render::draw_panel(&mut ops, STATUS.load(Ordering::Acquire), || {
        start.elapsed() <= FRAME_BUDGET
    }) {
        device::on_fault(device_ptr, reason);
    }
}
