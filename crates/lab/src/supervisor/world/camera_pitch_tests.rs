//! A pitch pinned at the camera's limit: reported, recovered, and the face
//! loop reads the camera's own gains (#1660 item 6).

use super::camera::{self, CameraRequest};
use super::geometry::Vec3;
use super::memory::CAMERA_PITCH_LIMIT_DEG;
use super::sim::Sim;

/// A guard 10 m ahead and a medic 20 m to the left, the camera pinned at the
/// pitch limit (readout +78.75 = looking straight down), with the live clamp.
fn pinned_room() -> Sim {
    let mut s = Sim::new()
        .with_entity(10, "Cellblock Guard", Vec3::new(1000.0, 0.0, 0.0))
        .with_entity(11, "Medic Ogilvie", Vec3::new(0.0, 2000.0, 0.0));
    s.live_pitch_clamp = true;
    s.cam_pitch = -CAMERA_PITCH_LIMIT_DEG.to_radians();
    s
}

#[tokio::test]
async fn a_face_from_a_pinned_pitch_says_so_and_comes_back() {
    let mut sim = pinned_room();
    let out = camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(10),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["face"]["centred"], true, "{out}");
    assert_eq!(out["face"]["pitch_clamped"], true, "{out}");
    let from = out["face"]["pitch_recovered_from"].as_f64().unwrap();
    assert!((from - CAMERA_PITCH_LIMIT_DEG).abs() < 0.1, "{out}");
    let after = out["view_after"]["pitch_offset_deg"].as_f64().unwrap();
    assert!(
        after.abs() < CAMERA_PITCH_LIMIT_DEG - 5.0,
        "still pinned: {out}"
    );
    assert!(out["pitch_clamped"].is_null(), "{out}");
}

/// A face that fails names the clamp (max_steps 0 stops it before it can turn).
#[tokio::test]
async fn a_failed_face_from_a_pinned_pitch_names_the_clamp() {
    let mut sim = pinned_room();
    let e = camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(11),
            max_steps: Some(0),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "face");
    assert!(e.message.contains("pitch clamped"), "{}", e.message);
    assert_eq!(e.to_json()["state"]["pitch_clamped"], true);
}

/// Raw pitch past the limit ends pinned: the result says so instead of
/// leaving the next call to find out.
#[tokio::test]
async fn a_camera_call_that_ends_pinned_warns() {
    let mut sim = Sim::new();
    sim.live_pitch_clamp = true;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            pitch_counts: Some(900),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["pitch_clamped"], true, "{out}");
    assert!(out["warning"].as_str().unwrap().contains("78.75"), "{out}");
}

/// An inverted pitch axis (flag 0x8) with a target far above the view: the
/// gains come from the readout, so the first look goes the right way and one
/// look centres it. With the old hard-coded guess (-400 counts/rad) the first
/// look went the wrong way and the sign had to be learned first.
#[tokio::test]
async fn an_inverted_pitch_axis_is_right_from_the_first_look() {
    let mut sim = Sim::new().with_entity(10, "High Sign", Vec3::new(1000.0, 0.0, 1500.0));
    sim.live_pitch_clamp = true;
    sim.pitch_inverted = true;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(10),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["face"]["centred"], true, "{out}");
    assert!(out["face"]["looks"].as_u64().unwrap() <= 2, "{out}");
}
