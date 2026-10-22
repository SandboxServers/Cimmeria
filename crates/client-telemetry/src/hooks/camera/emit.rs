//! `client.camera.at_interact`: the camera and pawn as the player clicked.
//!
//! The row is built before the original router runs, so it is the camera at
//! the moment of the click (D-PC3, under the diagnostics opt-in). The bag is
//! built by [`camera_fields`], a pure function the tests drive; the live
//! reader is [`at_interact`], which only exists on the 32-bit DLL target.

use serde_json::json;

use super::read::{self, ActorPose, CameraState};
use crate::hooks::entity_trace::Fields;

/// The cached `ASGWCamera_Player` pointer; [`read::camera_ptr`] rechecks its vtable on every use.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static CAMERA: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The fields of one `client.camera.at_interact`. Empty when there is no
/// camera (the caller then emits nothing). `send_id` is set only when given.
/// A camera pose that is missing or at the origin reports
/// `camera_pose_zero: true` and no `camera_*` position.
pub fn camera_fields(
    method: &str,
    entity_id: Option<i32>,
    send_id: Option<u32>,
    cam: Option<CameraState>,
    cam_pose: Option<ActorPose>,
    pawn: Option<ActorPose>,
) -> Fields {
    let Some(cam) = cam else {
        return Vec::new();
    };
    let mut f: Fields = vec![("method", json!(method))];
    if let Some(id) = entity_id {
        f.push(("entity_id", json!(id)));
    }
    if let Some(id) = send_id {
        f.push(("send_id", json!(id)));
    }
    let (zoom, yaw_offset_deg, pitch_offset_deg) = camera_angles(&cam);
    f.push(("zoom", json!(zoom)));
    f.push(("yaw_offset_deg", json!(yaw_offset_deg)));
    f.push(("pitch_offset_deg", json!(pitch_offset_deg)));
    f.push(("camera_flags", json!(cam.flags)));
    f.push(("camera_gain", json!(cam.gain)));

    let zero = cam_pose.is_none_or(|p| p.pos == [0.0; 3]);
    f.push(("camera_pose_zero", json!(zero)));
    if let Some(p) = cam_pose.filter(|_| !zero) {
        push_pose(&mut f, p, CAMERA_KEYS);
    }
    if let Some(p) = pawn {
        push_pose(&mut f, p, PAWN_KEYS);
        f.push(("pawn_pitch_deg", json!(read::rot_deg(p.pitch))));
    }
    f
}

/// Keys of the camera pose: x, y, z in server units, then the yaw.
const CAMERA_KEYS: [&str; 4] = ["camera_x", "camera_y", "camera_z", "camera_yaw_deg"];
/// Keys of the pawn pose, as [`CAMERA_KEYS`].
const PAWN_KEYS: [&str; 4] = ["pawn_x", "pawn_y", "pawn_z", "pawn_yaw_deg"];

/// The camera's zoom (its distance), and its yaw and pitch offsets in degrees.
pub(crate) fn camera_angles(cam: &CameraState) -> (f32, f32, f32) {
    (cam.distance, read::rot_deg(cam.yaw), read::rot_deg(cam.pitch))
}

/// A pose's position in server units, and its yaw in degrees wrapped to `[0, 360)`.
pub(crate) fn pawn_fields(pawn: &ActorPose) -> ([f32; 3], f32) {
    (read::client_to_server(pawn.pos), read::rot_deg_360(pawn.yaw))
}

/// Adds one pose under `keys`: the position in server units and the yaw.
fn push_pose(f: &mut Fields, p: ActorPose, keys: [&'static str; 4]) {
    let (pos, yaw) = pawn_fields(&p);
    f.push((keys[0], json!(pos[0])));
    f.push((keys[1], json!(pos[1])));
    f.push((keys[2], json!(pos[2])));
    f.push((keys[3], json!(yaw)));
}

/// Reads the camera and, under S-CAM A, the pawn, then emits the row for an
/// outbound `interact` or `useItem`. Under S-CAM B the pawn is passed as
/// `None`, so no `pawn_*` key is ever written.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub fn at_interact(method: &str, entity_id: Option<i32>, send_id: Option<u32>) {
    use crate::hooks::entity_trace::{self as trace, map::LiveMem};

    let mem = LiveMem;
    let Some(cam) = read::camera_ptr(&mem, &CAMERA) else {
        return;
    };
    let fields = camera_fields(
        method,
        entity_id,
        send_id,
        read::read_camera(&mem, cam),
        read::read_camera_pose(&mem, cam),
        read::read_pawn_pose(&mem),
    );
    if fields.is_empty() {
        return;
    }
    let target = "client.camera.at_interact";
    let decision = trace::throttle(&format!("{target}:{method}"), entity_id.unwrap_or(0));
    if let Some(f) = trace::with_suppressed(fields, decision) {
        crate::hooks::emit::emit(target, "info", f);
    }
}
