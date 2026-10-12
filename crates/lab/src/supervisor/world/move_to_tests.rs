//! `client_move_to` end to end against the simulated client.

use super::geometry::Vec3;
use super::movement::{self, MoveRequest};
use super::sim::Sim;
use super::tool_tests::{client_point, room};

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
    let mut sim = Sim::new().with_entity(10, "Quartermaster", Vec3::new(380.0, 0.0, 0.0));
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
    let st = e.to_json()["state"].clone();
    assert!(st["start_distance_m"].as_f64().unwrap() > 20.0);
    assert!(st["path"].is_array());
    assert!(st["turn_gain"]["counts_per_rad"].is_number());
    assert!(st["hint"].as_str().unwrap().contains("waypoints"));
}

/// A pawn that ignores the forward key must fail as `input_probe`, not as
/// a generic stuck/timeout (C-P6).
#[tokio::test]
async fn move_to_names_a_dead_forward_key() {
    let mut sim = Sim::new();
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
    let mut sim = Sim::new();
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
    let mut sim = Sim::new();
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
