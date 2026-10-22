//! The 4 Hz pose sampler: when the camera, the pawn or the input moved, and
//! the fields of one `client.nav.pose` row.
//!
//! Pure, on every target. The tick ([`super::tick`]) reads the engine and
//! offers each snapshot here; nothing is emitted while the pose is still.

use serde_json::json;

use super::controller::InputAxes;
use crate::hooks::entity_trace::Fields;

/// The least time between two pose checks, in milliseconds (4 Hz).
pub const POSE_PERIOD_MS: u64 = 250;
/// Degrees an angle must move before the pose counts as changed.
pub const ANGLE_EPS_DEG: f32 = 1.0;
/// Zoom units the distance must move before the pose counts as changed.
pub const ZOOM_EPS: f32 = 1.0;
/// Server units the pawn must move before the pose counts as changed.
pub const POS_EPS: f32 = 0.10;
/// Axis units an axis must move before the input counts as changed.
pub const AXIS_EPS: f32 = 0.01;

/// One reading of the camera, the pawn, the input axes and the controller words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseSnapshot {
    /// Camera zoom (its third-person distance).
    pub zoom: f32,
    /// Camera yaw offset, degrees.
    pub yaw_offset_deg: f32,
    /// Camera pitch offset, degrees.
    pub pitch_offset_deg: f32,
    /// The pawn's server position and yaw in degrees, when readable.
    pub pawn: Option<([f32; 3], f32)>,
    /// The `PlayerInput` axes, when readable.
    pub axes: Option<InputAxes>,
    /// The controller's mode bits.
    pub mode: u32,
    /// The controller's button bits.
    pub buttons: u32,
    /// The controller's hover entity id (`0` for none).
    pub hover_id: i32,
}

/// When the pose was last checked, and the snapshot last emitted.
#[derive(Default)]
pub struct PoseSampler {
    last_check_ms: Option<u64>,
    last_emitted: Option<PoseSnapshot>,
}

impl PoseSampler {
    /// A sampler that has not checked yet. `const`, so it can seed a `static`.
    pub const fn new() -> Self {
        Self {
            last_check_ms: None,
            last_emitted: None,
        }
    }

    /// Is a check due? True before the first check, then once
    /// [`POSE_PERIOD_MS`] has passed since the last one.
    pub fn due(&self, now_ms: u64) -> bool {
        match self.last_check_ms {
            None => true,
            Some(last) => now_ms.saturating_sub(last) >= POSE_PERIOD_MS,
        }
    }

    /// Records a check at `now_ms` with `snap`. True, and stores `snap`, when it
    /// is the first snapshot or it changed since the last emitted one.
    pub fn offer(&mut self, now_ms: u64, snap: PoseSnapshot) -> bool {
        self.last_check_ms = Some(now_ms);
        let changed = match &self.last_emitted {
            None => true,
            Some(prev) => changed(prev, &snap),
        };
        if changed {
            self.last_emitted = Some(snap);
        }
        changed
    }

    /// Records a check at `now_ms` that found no camera, so the next check
    /// waits a full period.
    pub fn skip(&mut self, now_ms: u64) {
        self.last_check_ms = Some(now_ms);
    }
}

/// Did the pose or the input move past its threshold, or change its mode,
/// buttons or hover?
pub fn changed(a: &PoseSnapshot, b: &PoseSnapshot) -> bool {
    angle_moved(a.yaw_offset_deg, b.yaw_offset_deg)
        || angle_moved(a.pitch_offset_deg, b.pitch_offset_deg)
        || (a.zoom - b.zoom).abs() >= ZOOM_EPS
        || pawn_changed(a.pawn, b.pawn)
        || axes_changed(a.axes, b.axes)
        || a.mode != b.mode
        || a.buttons != b.buttons
        || a.hover_id != b.hover_id
}

/// Did an angle move by [`ANGLE_EPS_DEG`] or more?
fn angle_moved(a: f32, b: f32) -> bool {
    angle_delta_deg(a, b).abs() >= ANGLE_EPS_DEG
}

/// The pawn went Some or None, or moved or turned past its threshold.
fn pawn_changed(a: Option<([f32; 3], f32)>, b: Option<([f32; 3], f32)>) -> bool {
    match (a, b) {
        (Some((pa, ya)), Some((pb, yb))) => {
            let d2: f32 = (0..3).map(|i| (pa[i] - pb[i]).powi(2)).sum();
            d2.sqrt() >= POS_EPS || angle_moved(ya, yb)
        }
        (None, None) => false,
        _ => true,
    }
}

/// The axes went Some or None, or one moved by [`AXIS_EPS`] or more.
fn axes_changed(a: Option<InputAxes>, b: Option<InputAxes>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => {
            (x.turn - y.turn).abs() >= AXIS_EPS
                || (x.fwd - y.fwd).abs() >= AXIS_EPS
                || (x.strafe - y.strafe).abs() >= AXIS_EPS
        }
        (None, None) => false,
        _ => true,
    }
}

/// The shortest signed difference `b - a` between two angles, in degrees, in
/// `-180..180`.
pub fn angle_delta_deg(a: f32, b: f32) -> f32 {
    let d = (b - a).rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

/// The fields of one `client.nav.pose` row (C-LN2). `pawn_ok` says whether the
/// pawn keys are present; `turn`, `fwd` and `strafe` only when the axes were
/// readable.
pub fn pose_fields(snap: &PoseSnapshot) -> Fields {
    let mut f: Fields = vec![
        ("zoom", json!(snap.zoom)),
        ("yaw_offset_deg", json!(snap.yaw_offset_deg)),
        ("pitch_offset_deg", json!(snap.pitch_offset_deg)),
        ("pawn_ok", json!(snap.pawn.is_some())),
    ];
    if let Some((pos, yaw)) = snap.pawn {
        f.push(("pawn_x", json!(pos[0])));
        f.push(("pawn_y", json!(pos[1])));
        f.push(("pawn_z", json!(pos[2])));
        f.push(("pawn_yaw_deg", json!(yaw)));
    }
    if let Some(axes) = snap.axes {
        f.push(("fwd", json!(axes.fwd)));
        f.push(("strafe", json!(axes.strafe)));
        f.push(("turn", json!(axes.turn)));
    }
    f.push(("mode", json!(snap.mode)));
    f.push(("buttons", json!(snap.buttons)));
    f.push(("hover_id", json!(snap.hover_id)));
    f
}
