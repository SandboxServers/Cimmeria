//! The ledge guard (C14): probe geometry, the probe throttle, the self-check
//! and a failing trace, against the fake `trace_line` of [`Sim`].

use super::geometry::{Pose, Vec3};
use super::ledge::{self, Guard};
use super::movement::{self, MoveRequest};
use super::sim::Sim;
use super::tool_tests::client_point;

/// A pawn at `(x, 0)`, standing 100 UU above the floor.
fn pose(x: f64, yaw: f64) -> Pose {
    Pose {
        pos: Vec3::new(x, 0.0, 100.0),
        yaw,
        pitch: 0.0,
    }
}

#[test]
fn probe_points_ahead_and_down() {
    let (start, end) = ledge::probe(pose(0.0, 0.0));
    assert_eq!(start, Vec3::new(150.0, 0.0, 100.0));
    assert_eq!(end, Vec3::new(150.0, 0.0, -178.0));
}

#[test]
fn self_check_span_is_straight_down_under_the_pawn() {
    let (start, end) = ledge::under(pose(0.0, 0.0));
    assert_eq!(start, Vec3::new(0.0, 0.0, 100.0));
    assert_eq!(end, Vec3::new(0.0, 0.0, -178.0));
}

#[test]
fn guard_labels() {
    assert_eq!(Guard::On { last_ms: None }.label(), "on");
    assert_eq!(Guard::Off.label(), "off");
    assert_eq!(
        Guard::Unavailable("boom".into()).label(),
        "unavailable: boom"
    );
}

#[tokio::test]
async fn probe_with_no_floor_ahead_stops_the_walk() {
    let mut sim = Sim::new();
    sim.floor_end_x = Some(0.0);
    let mut g = Guard::On { last_ms: None };
    assert!(ledge::check(&mut g, &mut sim, 0, pose(0.0, 0.0)).await);
}

#[tokio::test]
async fn probe_runs_at_most_every_500_ms() {
    let mut sim = Sim::new();
    let mut g = Guard::On { last_ms: None };
    ledge::check(&mut g, &mut sim, 0, pose(0.0, 0.0)).await;
    ledge::check(&mut g, &mut sim, 499, pose(0.0, 0.0)).await;
    assert_eq!(sim.trace_calls, 1, "a probe 499 ms after the last one");
    ledge::check(&mut g, &mut sim, 500, pose(0.0, 0.0)).await;
    assert_eq!(sim.trace_calls, 2);
}

#[tokio::test]
async fn a_failed_trace_makes_the_guard_unavailable() {
    let mut sim = Sim::new();
    sim.trace_error = Some("trace_line unsupported".into());
    let mut g = Guard::On { last_ms: None };
    assert!(!ledge::check(&mut g, &mut sim, 0, pose(0.0, 0.0)).await);
    assert_eq!(g.label(), "unavailable: trace_line unsupported");
}

#[tokio::test]
async fn no_floor_under_the_pawn_disables_the_guard() {
    let mut sim = Sim::new();
    sim.floor_end_x = Some(-1.0);
    let mut g = Guard::On { last_ms: None };
    ledge::self_check(&mut g, &mut sim, pose(0.0, 0.0)).await;
    assert_eq!(g.label(), "unavailable: no floor under pawn");
    assert!(!ledge::check(&mut g, &mut sim, 0, pose(0.0, 0.0)).await);
}

/// The walk with the floor ending at x = 500: it stops before the edge, lets
/// go of the keys, and says the guard ran.
#[tokio::test]
async fn walk_stops_before_a_drop() {
    let mut sim = Sim::live();
    sim.floor_end_x = Some(500.0);
    let e = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(2000.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap_err();
    let v = e.to_json();
    assert_eq!(v["stop"]["kind"], "ledge");
    assert!(v.to_string().contains("ledge ahead: no floor within 2 m"));
    assert!(v.to_string().contains("\"ledge_guard\":\"on\""));
    assert!(sim.pos.x < 700.0, "walked past the edge: {}", sim.pos.x);
    assert!(!sim.keys["W"], "forward key left held");
}

#[tokio::test]
async fn allow_drops_walks_off_the_edge() {
    let mut sim = Sim::live();
    sim.floor_end_x = Some(500.0);
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(900.0, 0.0, 0.0)),
            allow_drops: Some(true),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert_eq!(out["ledge_guard"], "off");
}

#[tokio::test]
async fn a_failing_trace_walks_on_with_the_guard_unavailable() {
    let mut sim = Sim::live();
    sim.floor_end_x = Some(500.0);
    sim.trace_error = Some("trace_line unsupported".into());
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(900.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert_eq!(out["ledge_guard"], "unavailable: trace_line unsupported");
}

#[tokio::test]
async fn a_walk_without_a_visible_floor_never_stops_as_a_ledge() {
    let mut sim = Sim::live();
    sim.floor_end_x = Some(-1.0);
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(900.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    assert_eq!(out["ledge_guard"], "unavailable: no floor under pawn");
}

#[tokio::test]
async fn the_guard_probes_at_most_twice_a_second_during_a_walk() {
    let mut sim = Sim::live();
    movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(2000.0, 0.0, 0.0)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert!(sim.now > 1000, "the walk too short to test: {} ms", sim.now);
    assert!(u64::from(sim.trace_calls) <= 2 + sim.now / 500);
}

#[test]
fn probe_follows_the_yaw() {
    let (s, e) = ledge::probe(pose(0.0, std::f64::consts::FRAC_PI_2));
    assert!(s.x.abs() < 1e-9 && (s.y - 150.0).abs() < 1e-9 && s.z == 100.0);
    assert!(e.x.abs() < 1e-9 && (e.y - 150.0).abs() < 1e-9 && e.z == -178.0);
}

#[test]
fn trace_tool_is_classified() {
    use crate::proof::claims::{claim, Claim};
    assert_eq!(claim("client_trace_line"), Claim::NoClaim);
    assert!(crate::lease::policy::LEASED.contains(&"client_trace_line"));
}

#[tokio::test]
async fn timeout_stop_names_the_guard() {
    let mut sim = Sim::live();
    let e = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(2000.0, 0.0, 0.0)),
            timeout_ms: Some(800),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap_err();
    let v = e.to_json();
    assert_eq!(v["stop"]["kind"], "timeout");
    assert!(v.to_string().contains("\"ledge_guard\":\"on\""));
}
