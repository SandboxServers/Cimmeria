//! Unit tests for the `client.camera.at_interact` bag (`camera_fields`).

use serde_json::Value;

use super::emit::camera_fields;
use super::read::{ActorPose, CameraState};

fn cam(distance: f32, yaw: i32, pitch: i32) -> CameraState {
    CameraState {
        flags: 0x5,
        gain: 1.5,
        distance,
        pitch,
        yaw,
    }
}

fn pose(pos: [f32; 3], yaw: i32) -> ActorPose {
    ActorPose { pos, pitch: 0, yaw }
}

/// The value of `key` in the bag, if present.
fn get(f: &[(&'static str, Value)], key: &str) -> Option<Value> {
    f.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
}

/// A zoom, offset or angle in the bag, as `f64`.
fn num(f: &[(&'static str, Value)], key: &str) -> f64 {
    get(f, key)
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| panic!("{key} is missing or not a number"))
}

/// Zoom and both offsets come from the camera block, in degrees.
#[test]
fn fields_carry_zoom_and_offsets() {
    let f = camera_fields(
        "interact",
        Some(7),
        None,
        Some(cam(400.0, 16384, -2048)),
        None,
        None,
    );
    assert_eq!(num(&f, "zoom"), 400.0);
    assert_eq!(num(&f, "yaw_offset_deg"), 90.0);
    assert_eq!(num(&f, "pitch_offset_deg"), -11.25);
}

/// A missing camera pose is reported as zero and carries no position.
#[test]
fn zero_camera_pose_omits_position() {
    let f = camera_fields("interact", Some(7), None, Some(cam(1.0, 0, 0)), None, None);
    assert_eq!(get(&f, "camera_pose_zero"), Some(Value::Bool(true)));
    assert!(get(&f, "camera_x").is_none(), "no position when zero");
}

/// Camera positions are sent in server units, not client units.
#[test]
fn camera_pose_is_in_server_units() {
    let f = camera_fields(
        "useItem",
        Some(7),
        None,
        Some(cam(1.0, 0, 0)),
        Some(pose([100.0, 200.0, 300.0], 0)),
        None,
    );
    assert_eq!(get(&f, "camera_pose_zero"), Some(Value::Bool(false)));
    assert_eq!(num(&f, "camera_x"), 2.0);
    assert_eq!(num(&f, "camera_y"), 3.0);
    assert_eq!(num(&f, "camera_z"), 1.0);
}

/// No camera means no row at all, so the caller emits nothing.
#[test]
fn no_camera_no_row() {
    let f = camera_fields("interact", Some(7), Some(1), None, None, None);
    assert!(f.is_empty());
}

/// `send_id` appears only when the caller passes one.
#[test]
fn send_id_only_when_given() {
    let c = Some(cam(1.0, 0, 0));
    let some = camera_fields("interact", Some(7), Some(77), c, None, None);
    assert_eq!(get(&some, "send_id"), Some(Value::from(77u32)));
    let none = camera_fields("interact", Some(7), None, c, None, None);
    assert!(get(&none, "send_id").is_none(), "no key when None");
}

/// Under S-CAM A a readable pawn adds its yaw; the verdict is A (PC-06).
#[test]
fn pawn_fields_follow_s_cam() {
    let f = camera_fields(
        "interact",
        Some(7),
        None,
        Some(cam(1.0, 0, 0)),
        None,
        Some(pose([100.0, 200.0, 300.0], 16384)),
    );
    assert_eq!(num(&f, "pawn_yaw_deg"), 90.0);
    // The pawn position is in server units, like the camera's.
    assert_eq!(num(&f, "pawn_x"), 2.0);
    assert_eq!(num(&f, "pawn_y"), 3.0);
    assert_eq!(num(&f, "pawn_z"), 1.0);
    assert_eq!(num(&f, "pawn_pitch_deg"), 0.0);
    // Under S-CAM B the live reader passes `None` for the pawn, and then no
    // `pawn_*` key is written at all.
    let without = camera_fields("interact", Some(7), None, Some(cam(1.0, 0, 0)), None, None);
    assert!(
        without.iter().all(|(k, _)| !k.starts_with("pawn_")),
        "no pawn key without a pawn"
    );
}

/// The bag names no player, account or character entity (D-PC2).
#[test]
fn no_personal_names() {
    let f = camera_fields(
        "interact",
        Some(7),
        Some(3),
        Some(cam(1.0, 0, 0)),
        Some(pose([1.0, 2.0, 3.0], 1)),
        Some(pose([1.0, 2.0, 3.0], 1)),
    );
    for banned in ["entity_name", "player_name", "account_name"] {
        assert!(get(&f, banned).is_none(), "{banned} must not be sent");
    }
}
