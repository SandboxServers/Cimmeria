//! The engine-tick step of the nav poll: read the controller, sample the pose
//! at 4 Hz, then run the input steps.
//!
//! Called from the engine tick detour on the main game thread, inside its
//! `catch_unwind`. Every read is a poll; nothing is hooked here. The kill
//! switch is the `nav_poll` name in `CIMMERIA_CLIENT_HOOKS_DISABLE`.

use std::sync::atomic::AtomicU32;
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde_json::json;

use super::buttons;
use super::controller::read_controller;
use super::frame::{self, NavFrame, PcCache};
use super::pose::{pose_fields, PoseSampler, PoseSnapshot};
use super::{transitions, ui};
use crate::hooks::camera::emit::{camera_angles, pawn_fields};
use crate::hooks::camera::read;
use crate::hooks::entity_trace::map::{LiveMem, Mem};
use crate::hooks::inline_hooks::{hook_disabled, DISABLE_ENV};

/// The name the kill switch uses for this poll.
const HOOK_NAME: &str = "nav_poll";

/// Whether the kill switch is set, decided once on the first tick.
static DISABLED: OnceLock<bool> = OnceLock::new();
/// The cached controller pointer and the next scan time.
static PC: Mutex<PcCache> = Mutex::new(PcCache {
    ptr: 0,
    next_scan_ms: 0,
});
/// The cached `ASGWCamera_Player` pointer; [`read::camera_ptr`] rechecks its vtable on every use.
static CAMERA: AtomicU32 = AtomicU32::new(0);
/// The pose sampler's clock and last emitted snapshot.
static SAMPLER: Mutex<PoseSampler> = Mutex::new(PoseSampler::new());

/// Locks `m`, recovering a poisoned lock: a panic in another tick must not stop the poll.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether the kill switch is set. Emits `client.hooks.inline.disabled` once when it is.
fn disabled() -> bool {
    *DISABLED.get_or_init(|| {
        let off = hook_disabled(HOOK_NAME);
        if off {
            crate::hooks::emit::emit(
                "client.hooks.inline.disabled",
                "info",
                vec![
                    ("hook", json!(HOOK_NAME)),
                    ("env", json!(DISABLE_ENV)),
                ],
            );
        }
        off
    })
}

/// One engine tick of the nav poll. Returns early when the kill switch is set,
/// when no controller is found, or when its words cannot be read.
pub(crate) fn on_tick() {
    if disabled() {
        return;
    }
    let mem = LiveMem;
    let now = frame::now_ms();
    let pc = {
        let mut cache = lock(&PC);
        frame::controller_ptr(&mem, now, &mut cache)
    };
    let Some(pc) = pc else {
        return;
    };
    let Some(ctrl) = read_controller(&mem, pc) else {
        return;
    };
    let frame = NavFrame {
        now_ms: now,
        ctrl,
        ui_owned: ui::read_ui_owned(&mem),
    };
    sample_pose(&mem, &frame);
    buttons::step(&frame);
    transitions::step(&frame);
}

/// Checks the pose when it is due, and emits `client.nav.pose` when it moved.
/// With no camera the check is skipped, so the next one waits a full period.
fn sample_pose(mem: &impl Mem, frame: &NavFrame) {
    let mut sampler = lock(&SAMPLER);
    if !sampler.due(frame.now_ms) {
        return;
    }
    let camera = read::camera_ptr(mem, &CAMERA).and_then(|c| read::read_camera(mem, c));
    let Some(cam) = camera else {
        sampler.skip(frame.now_ms);
        return;
    };
    let (zoom, yaw_offset_deg, pitch_offset_deg) = camera_angles(&cam);
    let snap = PoseSnapshot {
        zoom,
        yaw_offset_deg,
        pitch_offset_deg,
        pawn: read::read_pawn_pose(mem).as_ref().map(pawn_fields),
        axes: frame.ctrl.axes,
        mode: frame.ctrl.mode,
        buttons: frame.ctrl.buttons,
        hover_id: frame.ctrl.hover_id,
    };
    if sampler.offer(frame.now_ms, snap) {
        crate::hooks::emit::emit("client.nav.pose", "info", pose_fields(&snap));
    }
}
