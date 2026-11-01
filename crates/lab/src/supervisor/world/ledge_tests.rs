//! The ledge guard (C14): probe geometry, the probe throttle, the self-check
//! and a failing trace, against the fake `trace_line` of [`Sim`].

use super::geometry::{Pose, Vec3};
use super::ledge::{self, Guard};
use super::sim::Sim;

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
