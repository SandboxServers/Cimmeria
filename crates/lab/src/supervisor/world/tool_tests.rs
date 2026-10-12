//! The world tools end to end against the simulated client.

use serde_json::Value;

use super::camera::{self, CameraRequest};
use super::click::{self, ClickRequest, Expect};
use super::find::{self, FindRequest};
use super::geometry::{wrap_pi, Vec3};
use super::sim::{Sim, SimEntity, UNITS};
use super::{PointArg, Space};

pub(super) fn client_point(x: f64, y: f64, z: f64) -> PointArg {
    PointArg {
        x,
        y,
        z,
        space: Some(Space::Client),
    }
}

fn target_of(sim: &Sim) -> u32 {
    sim.slots.get(&UNITS.target).copied().unwrap_or(0)
}

/// A guard 10 m ahead, a medic 20 m to the left, an unrendered entity.
pub(super) fn room() -> Sim {
    let mut s = Sim::new()
        .with_entity(10, "Cellblock Guard", Vec3::new(1000.0, 0.0, 0.0))
        .with_entity(11, "Medic Ogilvie", Vec3::new(0.0, 2000.0, 0.0));
    s.entities[0].hostility = "Hostile".into();
    s.entities[1].window = Some("DialogWin".into());
    s.entities.push(SimEntity {
        id: 12,
        name: "Hidden".into(),
        pos: Vec3::new(500.0, 500.0, 0.0),
        hostility: "Friendly".into(),
        window: None,
        rendered: false,
    });
    s
}

#[tokio::test]
async fn find_names_sorts_and_projects() {
    let mut sim = room();
    let out = find::run(&mut sim, FindRequest::default()).await.unwrap();
    assert_eq!(out["native_level"], "read");
    let m = out["matches"].as_array().unwrap();
    // Nearest first; the unrendered one has no pose, so it sorts last.
    let ids: Vec<u64> = m.iter().map(|e| e["id"].as_u64().unwrap()).collect();
    assert_eq!(ids, vec![10, 11, 12]);
    assert_eq!(m[0]["name"], "Cellblock Guard");
    assert_eq!(m[0]["distance_m"], 10.0);
    assert_eq!(m[0]["on_screen"], true);
    assert_eq!(m[0]["targetable"], true);
    // 90 degrees left of a 90-degree view: not on screen.
    assert_eq!(m[1]["on_screen"], false);
    assert_eq!(m[2]["rendered"], false);
    assert_eq!(m[2]["targetable"], false);
}

#[tokio::test]
async fn find_filters_by_name_hostility_and_distance() {
    let mut sim = room();
    let by_name = find::run(
        &mut sim,
        FindRequest {
            name: Some("medic".into()),
            ..FindRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(by_name["count"], 1);
    assert_eq!(by_name["matches"][0]["id"], 11);
    let hostile = find::run(
        &mut sim,
        FindRequest {
            hostility: Some("hostile".into()),
            ..FindRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(hostile["matches"][0]["id"], 10);
    let near = find::run(
        &mut sim,
        FindRequest {
            max_distance_m: Some(15.0),
            ..FindRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(near["count"], 1);
}

#[tokio::test]
async fn target_clicks_the_entity_with_real_input() {
    let mut sim = room();
    let out = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            entity_id: Some(10),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap();
    assert_eq!(target_of(&sim), 10);
    assert_eq!(out["native_level"], "real_input");
    assert_eq!(out["counts_as_native_pass"], true);
    assert_eq!(out["hover_verified"], true);
    assert_eq!(out["result"]["target_is_entity"], true);
    assert_eq!(sim.buttons, vec![(0, true), (0, false)]);
}

/// Off to the side with an inverted, less sensitive mouse: the camera turns
/// until the entity is on screen, then the click lands.
#[tokio::test]
async fn target_turns_the_camera_when_off_screen() {
    let mut sim = room();
    sim.gain = -150.0;
    let out = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            name: Some("Ogilvie".into()),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap();
    assert_eq!(target_of(&sim), 11);
    assert!(out["camera"]["looks"].as_u64().unwrap() >= 1);
    assert_eq!(out["camera"]["centred"], true);
}

#[tokio::test]
async fn off_screen_without_rotation_is_a_named_failure() {
    let mut sim = room();
    let e = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            entity_id: Some(11),
            rotate_camera: Some(false),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "on_screen");
    assert!(e.summary().contains("\"entity_id\":11"));
    assert!(sim.buttons.is_empty());
}

/// Something nearer covers every height tried on the target's body.
#[tokio::test]
async fn an_occluder_stops_the_click() {
    let mut sim = room();
    // 5 m out on the guard's line, raised so it covers the guard on screen.
    sim = sim.with_entity(20, "Crate", Vec3::new(500.0, 0.0, 112.5));
    let e = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            entity_id: Some(10),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "hover");
    let state = e.to_json()["state"].to_string();
    assert!(state.contains("\"other_entity\":20"), "{state}");
    assert!(sim.buttons.is_empty());
    // The view retries ran (pitch up twice, yaw both ways, zoom), are
    // reported, and leave yaw and pitch where they started.
    assert!(state.contains("view_retries"), "{state}");
    let retries: Vec<_> = sim.looks.iter().filter(|l| **l != (0, 0, 0)).collect();
    assert!(
        retries.len() >= click::VIEW_RETRIES.len(),
        "{:?}",
        sim.looks
    );
    let (dx, dy): (i32, i32) = click::VIEW_RETRIES
        .iter()
        .fold((0, 0), |a, r| (a.0 + r.0, a.1 + r.1));
    assert_eq!((dx, dy), (0, 0), "the retries end where they began");
    // Regression guard (review of #1309): a failed retry run zooms back out.
    // The pitch the sim's clamp cut off is taken back in the same look (the
    // readout now reports it), so the view ends level.
    let zoom: i32 = click::VIEW_RETRIES.iter().map(|r| r.2).sum();
    let last = sim.looks.last().unwrap();
    assert_eq!((last.0, last.2), (0, -zoom * 120), "{:?}", sim.looks);
    assert!(sim.cam_pitch.abs() < 0.01, "pitch {}", sim.cam_pitch);
    assert!(state.contains("\"view_restored\":true"), "{state}");
}

#[tokio::test]
async fn mouse_over_that_needs_a_mouse_move_still_verifies() {
    let mut sim = room();
    sim.hover_needs_mouse_move = true;
    let out = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            entity_id: Some(10),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap();
    assert_eq!(out["hover_verified"], true);
    assert_eq!(target_of(&sim), 10);
}

/// A click the client ignores: an error without the fallback; with it,
/// success reported as N3, never as a native pass.
#[tokio::test]
async fn fallback_to_target_unit_is_reported_as_ui_lua() {
    let mut sim = room();
    sim.clicks_do_nothing = true;
    let req = ClickRequest {
        entity_id: Some(10),
        settle_ms: Some(300),
        ..ClickRequest::default()
    };
    let e = click::run(
        &mut sim,
        "client_target",
        req.clone(),
        Some((0, Expect::Target)),
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "observe");
    assert_eq!(e.to_json()["state"]["target_is_entity"], false);

    let out = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            allow_fallback: true,
            ..req
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap();
    assert_eq!(out["native_level"], "ui_lua");
    assert_eq!(out["native_tier"], "N3");
    assert_eq!(out["counts_as_native_pass"], false);
    assert_eq!(sim.target_unit_calls, 1);
    assert_eq!(target_of(&sim), 10);
}

#[tokio::test]
async fn right_click_reports_the_window_it_opened() {
    let mut sim = room();
    sim.cam_yaw = std::f64::consts::FRAC_PI_2;
    sim.pawn_yaw = sim.cam_yaw;
    let out = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(out["button"], "right");
    let opened: Vec<&Value> = out["result"]["windows_opened"]
        .as_array()
        .unwrap()
        .iter()
        .collect();
    assert_eq!(opened, vec![&Value::from("DialogWin")]);
}

#[tokio::test]
async fn camera_faces_an_entity_behind() {
    let mut sim = room();
    sim.cam_yaw = std::f64::consts::PI;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(10),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["face"]["centred"], true);
    let p = sim.project_point(Vec3::new(1000.0, 0.0, 0.0)).unwrap();
    assert!((p.0 - 500.0).abs() <= 125.0);
    assert_eq!(out["native_level"], "real_input");
}

#[tokio::test]
async fn camera_raw_look_and_zoom_are_passed_through() {
    let mut sim = Sim::new();
    let out = camera::run(
        &mut sim,
        CameraRequest {
            yaw_counts: Some(200),
            zoom_notches: Some(-2),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(sim.looks, vec![(200, 0, -240)]);
    assert_eq!(out["native_level"], "real_input");
}

/// Level camera, or one whose readout pitch is `pitch_deg` (the readout
/// rises when the camera looks up, so the sim's pitch is its negative).
fn camera_at_pitch(pitch_deg: f64) -> Sim {
    let mut sim = room();
    sim.cam_pitch = (-pitch_deg).to_radians();
    sim
}

#[tokio::test]
async fn absolute_pitch_lands_from_any_start() {
    for start in [0.0, 30.0, -40.0] {
        let mut sim = camera_at_pitch(start);
        let out = camera::run(
            &mut sim,
            CameraRequest {
                pitch_deg: Some(-28.8),
                ..CameraRequest::default()
            },
        )
        .await
        .unwrap();
        let pitch = &out["absolute"]["pitch"];
        let got = pitch["got"].as_f64().unwrap();
        assert!((got + 28.8).abs() <= 0.5, "from {start}: {out}");
        assert!(pitch["looks"].as_u64().unwrap() <= 4, "from {start}: {out}");
    }
}

/// Face a floor point, optionally then hold an absolute pitch.
async fn face_floor(start_deg: f64, pitch_deg: Option<f64>) -> Value {
    let mut sim = camera_at_pitch(start_deg);
    camera::run(
        &mut sim,
        CameraRequest {
            face_point: Some(client_point(1000.0, 0.0, 0.0)),
            pitch_deg,
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn face_then_absolute_pitch_is_start_independent() {
    let level = face_floor(0.0, Some(-28.8)).await;
    let raised = face_floor(22.0, Some(-28.8)).await;
    let a = level["absolute"]["pitch"]["got"].as_f64().unwrap();
    let b = raised["absolute"]["pitch"]["got"].as_f64().unwrap();
    assert!((a - b).abs() <= 0.5, "{a} vs {b}");

    // Without the field the two faces end at different pitches.
    let level = face_floor(0.0, None).await;
    let raised = face_floor(22.0, None).await;
    let a = level["view_after"]["pitch_offset_deg"].as_f64().unwrap();
    let b = raised["view_after"]["pitch_offset_deg"].as_f64().unwrap();
    assert!((a - b).abs() > 0.5, "{a} vs {b}");
}

#[tokio::test]
async fn yaw_offset_turns_relative_to_the_face() {
    let mut sim = room();
    camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(10),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    let before = sim.cam_yaw;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            yaw_offset_deg: Some(25.3),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    let turned = wrap_pi(sim.cam_yaw - before).to_degrees();
    assert!((turned - 25.3).abs() <= 0.5, "{turned}: {out}");
    // The last look's yaw counts turn the camera the way they point.
    let (dx, _, _) = *sim.looks.last().unwrap();
    assert_eq!(dx > 0, turned > 0.0);
    assert_eq!(out["absolute"]["yaw"]["open_loop"], Value::Null);
}

#[tokio::test]
async fn zoom_and_zoom_to_together_is_refused() {
    let mut sim = Sim::new();
    let e = camera::run(
        &mut sim,
        CameraRequest {
            zoom: Some(300.0),
            zoom_to: Some(300.0),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "arguments");
    assert!(sim.looks.is_empty());
}

#[tokio::test]
async fn yaw_offset_is_world_yaw_on_an_inverted_camera() {
    // Flag 0x4: +counts turn the camera the other way, so +25.3 deg of world
    // yaw needs negative counts.
    let mut sim = room();
    sim.gain = -400.0;
    let before = sim.cam_yaw;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            yaw_offset_deg: Some(25.3),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    let turned = wrap_pi(sim.cam_yaw - before).to_degrees();
    assert!((turned - 25.3).abs() <= 0.5, "{turned}: {out}");
    assert!(
        sim.looks.iter().all(|&(dx, _, _)| dx <= 0),
        "{:?}",
        sim.looks
    );
}

#[tokio::test]
async fn a_yaw_offset_past_half_a_turn_is_refused() {
    for want in [270.0, -200.0] {
        let mut sim = room();
        let e = camera::run(
            &mut sim,
            CameraRequest {
                yaw_offset_deg: Some(want),
                ..CameraRequest::default()
            },
        )
        .await
        .unwrap_err();
        assert_eq!(e.step, "arguments", "{want}");
        assert!(sim.looks.is_empty());
    }
}

#[tokio::test]
async fn absolute_pitch_corrects_a_wrong_readout_gain() {
    // The readout claims 0.7x the true gain: the first look misses, and only
    // the closed loop's correction lands it.
    let mut sim = camera_at_pitch(10.0);
    sim.readout_gain_scale = 0.7;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            pitch_deg: Some(-28.8),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    let pitch = &out["absolute"]["pitch"];
    assert!(
        (pitch["got"].as_f64().unwrap() + 28.8).abs() <= 0.5,
        "{out}"
    );
    let looks = pitch["looks"].as_u64().unwrap();
    assert!((2..=4).contains(&looks), "{out}");
}

#[tokio::test]
async fn absolute_yaw_corrects_a_wrong_readout_gain() {
    let mut sim = room();
    sim.readout_gain_scale = 0.7;
    let before = sim.cam_yaw;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            yaw_offset_deg: Some(-40.0),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    let turned = wrap_pi(sim.cam_yaw - before).to_degrees();
    assert!((turned + 40.0).abs() <= 0.5, "{turned}: {out}");
    assert!(
        out["absolute"]["yaw"]["looks"].as_u64().unwrap() >= 2,
        "{out}"
    );
}

#[tokio::test]
async fn an_unreachable_pitch_fails_at_the_limit_without_idle_looks() {
    let mut sim = camera_at_pitch(0.0);
    let e = camera::run(
        &mut sim,
        CameraRequest {
            pitch_deg: Some(-85.0),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "abs_pitch");
    // One clamped look, then the limit is recognised; never a (0, 0, 0) look.
    assert!(sim.looks.len() <= 2, "{:?}", sim.looks);
    assert!(sim.looks.iter().all(|l| *l != (0, 0, 0)), "{:?}", sim.looks);
}

#[tokio::test]
async fn a_zero_yaw_offset_sends_no_look() {
    let mut sim = room();
    camera::run(
        &mut sim,
        CameraRequest {
            yaw_offset_deg: Some(0.0),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert!(sim.looks.is_empty(), "{:?}", sim.looks);
}

#[tokio::test]
async fn a_zero_readout_gain_fails_instead_of_spinning() {
    // A camera read before the game loads its settings reports gain 0.
    let mut sim = room();
    sim.gain = f64::INFINITY;
    let e = camera::run(
        &mut sim,
        CameraRequest {
            pitch_deg: Some(-28.8),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "abs_pitch");
    assert!(sim.looks.is_empty(), "{:?}", sim.looks);
}

#[tokio::test]
async fn zoom_moves_the_camera_like_zoom_to() {
    let mut sim = Sim::new();
    camera::run(
        &mut sim,
        CameraRequest {
            zoom: Some(400.0),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(sim.looks, vec![(0, 0, -600)]);
    assert!((sim.zoom - 400.0).abs() < 1e-6);
}
