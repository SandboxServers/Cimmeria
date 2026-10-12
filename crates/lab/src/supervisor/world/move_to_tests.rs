//! `client_move_to` end to end against the simulated client.

use super::geometry::Vec3;
use super::movement::{self, MoveRequest};
use super::sim::Sim;
use super::tool_tests::{client_point, room};

#[tokio::test]
async fn move_to_walks_to_a_point_and_lets_go_of_the_keys() {
    let mut sim = Sim::live();
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

/// The 2026-10-12 live defect (#1660 item 8): the pawn ignores mouse-look,
/// so a controller that steers with it never turns and walks away. The
/// target is 90 degrees to the left; only the turn keys can get there.
#[tokio::test]
async fn move_to_turns_a_pawn_that_ignores_mouse_look() {
    let mut sim = Sim::live();
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(0.0, 3000.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert!(
        sim.looks.is_empty(),
        "steered with mouse-look: {:?}",
        sim.looks
    );
    assert!((sim.pos.y - 3000.0).abs() <= 150.0 + 60.0);
    assert!(out["turn_rate"]["samples"].as_u64().unwrap() >= 1);
    assert!(!sim.keys["E"] && !sim.keys["W"], "keys left held");
}

/// A target behind, a slower pawn and a flipped key: the controller learns
/// the rate and the sign from the yaw it reads and still arrives.
#[tokio::test]
async fn move_to_learns_the_turn_rate_and_a_flipped_key() {
    let mut sim = Sim::live();
    sim.turn_rate_rad_s = 1.0;
    sim.right_raises_yaw = false;
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
    assert!(out["turn_rate"]["sign_flips"].as_u64().unwrap() >= 1);
    let rate = out["turn_rate"]["rad_s"].as_f64().unwrap();
    assert!((rate - 1.0).abs() < 0.35, "learned {rate} rad/s");
}

/// The keys are the client's bindings, not a hard-coded Q/E.
#[tokio::test]
async fn move_to_presses_the_bound_turn_keys() {
    let mut sim = Sim::live();
    sim.turn_key_names = ("J".into(), "L".into());
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(0.0, 3000.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert!(
        sim.keys.contains_key("L"),
        "the bound right key was never pressed"
    );
    assert!(!sim.keys.contains_key("E") && !sim.keys.contains_key("Q"));
    assert!(!sim.keys["L"], "turn key left held");
}

/// A cancelled walk's cleanup lets go of the bound turn keys, not just Q/E.
#[tokio::test]
async fn release_stray_lets_go_of_the_bound_turn_keys() {
    let mut sim = Sim::live();
    sim.turn_key_names = ("J".into(), "L".into());
    for k in ["W", "J", "L"] {
        sim.keys.insert(k.into(), true);
    }
    movement::release_stray(&mut sim).await;
    assert!(
        ["W", "J", "L"].iter().all(|k| !sim.keys[*k]),
        "{:?}",
        sim.keys
    );
}

/// A rooted pawn (turn rate 0) ends the walk honestly, with the keys up.
#[tokio::test]
async fn move_to_reports_a_pawn_that_does_not_turn() {
    let mut sim = Sim::live();
    sim.turn_rate_rad_s = 0.0;
    let e = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(0.0, 3000.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "leg_0");
    assert!(e.message.contains("did not turn"), "{}", e.message);
    assert!(!sim.keys["E"] && !sim.keys["W"]);
}

#[tokio::test]
async fn move_to_follows_waypoints_in_order() {
    let mut sim = Sim::live();
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
    let mut sim = Sim::live();
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
    let mut sim = Sim::live();
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
    let mut sim = Sim::live();
    sim.speed_uu_s = 100.0;
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
    let st = e.to_json()["state"].clone();
    assert!(st["hint"].as_str().unwrap().contains("waypoints"));
    assert!(st["path"].is_array() && st["start_distance_m"].is_number());
}

#[tokio::test]
async fn move_to_an_entity_stops_beside_it() {
    let mut sim = room();
    sim.live_pawn = true;
    sim.standing_turn = false;
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
    assert!(d <= 4.0 + 0.1, "stopped {d} m away");
}

/// A vendor already in interact range is a no-op: no key is pressed, and
/// the result says so.
#[tokio::test]
async fn move_to_an_entity_already_in_range_presses_nothing() {
    let mut sim = Sim::live().with_entity(10, "Quartermaster", Vec3::new(380.0, 0.0, 0.0));
    let out = movement::run(
        &mut sim,
        MoveRequest {
            name: Some("quartermaster".into()),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert_eq!(out["already_in_range"], true);
    assert!(sim.looks.is_empty(), "turned toward a target in range");
    assert!((sim.pos.x - 0.0).abs() < 1.0, "walked {} uu", sim.pos.x);
}

/// A failed walk carries the evidence needed to tell a snag from drift.
#[tokio::test]
async fn move_to_failure_carries_path_gain_and_hint() {
    let mut sim = Sim::live();
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
    let st = e.to_json()["state"].clone();
    assert!(st["start_distance_m"].as_f64().unwrap() > 20.0);
    assert!(st["path"].is_array());
    assert!(st["turn_rate"]["rad_s"].is_number());
    assert!(st["hint"].as_str().unwrap().contains("waypoints"));
}

/// A pawn that ignores the forward key must fail as `input_probe`, not as
/// a generic stuck/timeout (C-P6).
#[tokio::test]
async fn move_to_names_a_dead_forward_key() {
    let mut sim = Sim::live();
    sim.speed_uu_s = 0.0;
    let e = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(2500.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "input_probe");
    assert!(e.message.contains("did not move"), "{}", e.message);
    assert!(e.to_json()["state"]["moved_m"].as_f64().unwrap() < 0.1);
    assert!(!sim.keys["W"], "forward key left held after a failure");
}

/// A target already inside the arrival radius costs no real move: the
/// probe sits after the arrival check. A dead-W pawn would fail the probe,
/// so success here proves the probe did not run.
#[tokio::test]
async fn move_to_in_range_does_not_probe() {
    let mut sim = Sim::live();
    sim.speed_uu_s = 0.0;
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(100.0, 0.0, 0.0)), // 1 m, arrival 1.5 m
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert!(!sim.keys.get("W").copied().unwrap_or(false));
}

/// A slow but moving avatar (snared: 0.6 m/s) must pass the 0.1 m probe and
/// still arrive, not be named a dead key.
#[tokio::test]
async fn move_to_probe_passes_a_slow_avatar() {
    let mut sim = Sim::live();
    sim.speed_uu_s = 60.0;
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(400.0, 0.0, 0.0)),
            timeout_ms: Some(60_000),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
}
