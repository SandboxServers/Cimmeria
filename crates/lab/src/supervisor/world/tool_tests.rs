//! The world tools end to end against the simulated client.

use serde_json::Value;

use super::camera::{self, CameraRequest};
use super::click::{self, ClickRequest, Expect};
use super::find::{self, FindRequest};
use super::geometry::Vec3;
use super::movement::{self, MoveRequest};
use super::sim::{Sim, SimEntity, UNITS};
use super::{PointArg, Space};

fn client_point(x: f64, y: f64, z: f64) -> PointArg {
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
fn room() -> Sim {
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
    assert!(e.to_json()["state"]
        .to_string()
        .contains("\"other_entity\":20"));
    assert!(sim.buttons.is_empty());
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
async fn move_to_walks_to_a_point_and_lets_go_of_the_keys() {
    let mut sim = Sim::new();
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(3000.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert_eq!(out["native_level"], "real_input");
    assert!((sim.pos.x - 3000.0).abs() <= 150.0 + 60.0);
    assert!(!sim.keys["W"], "forward key left held");
}

/// Target behind, inverted and slow mouse, pawn does not turn standing:
/// the controller learns all three and still arrives.
#[tokio::test]
async fn move_to_learns_an_inverted_gain_and_walking_turns() {
    let mut sim = Sim::new();
    sim.gain = -200.0;
    sim.standing_turn = false;
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(-2500.0, 800.0, 0.0)),
            timeout_ms: Some(60_000),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert!(out["turn_gain"]["counts_per_rad"].as_f64().unwrap() < 0.0);
    assert_eq!(out["legs"][0]["standing_turn"], false);
}

#[tokio::test]
async fn move_to_follows_waypoints_in_order() {
    let mut sim = Sim::new();
    let out = movement::run(
        &mut sim,
        MoveRequest {
            waypoints: vec![client_point(1000.0, 1000.0, 0.0)],
            point: Some(client_point(0.0, 2000.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["legs"].as_array().unwrap().len(), 2);
    assert_eq!(out["legs"][0]["outcome"]["arrived"], true);
}

#[tokio::test]
async fn move_to_jumps_out_of_a_snag() {
    let mut sim = Sim::new();
    sim.wall_x = Some(1000.0);
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(2500.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["legs"][0]["unsticks"][0], "jump");
}

#[tokio::test]
async fn move_to_reports_a_snag_it_cannot_clear() {
    let mut sim = Sim::new();
    sim.wall_x = Some(1000.0);
    sim.jump_clears_wall = false;
    let e = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(2500.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "leg_0");
    assert!(e.message.contains("stuck"));
    assert!(!sim.keys["W"], "forward key left held after a failure");
}

#[tokio::test]
async fn move_to_times_out_with_the_distance_left() {
    let mut sim = Sim::new();
    sim.speed_uu_s = 10.0;
    let e = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(5000.0, 0.0, 0.0)),
            timeout_ms: Some(2000),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert!(e.message.contains("timed out"));
    assert!(e.to_json()["state"]["distance_m"].as_f64().unwrap() > 40.0);
}

#[tokio::test]
async fn move_to_an_entity_stops_beside_it() {
    let mut sim = room();
    let out = movement::run(
        &mut sim,
        MoveRequest {
            name: Some("guard".into()),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    let d = super::geometry::horizontal_m(sim.pos, Vec3::new(1000.0, 0.0, 0.0));
    assert!(d <= 2.5 + 0.1, "stopped {d} m away");
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
