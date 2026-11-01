//! The walker's coast model, the early release and the hold-off, against the
//! pure controller and the simulated client.

use super::coast::{CoastModel, COAST_PRIOR_S};
use super::geometry::{horizontal_m, Pose, Vec3, UU_PER_METRE};
use super::io::WorldIo;
use super::movement::{self, MoveRequest};
use super::sim::Sim;
use super::steer::*;
use super::tool_tests::client_point;

/// The goal of the pure controller tests, client units, straight ahead.
const GOAL_UU: f64 = 10_000.0;

/// A pose `dist_m` short of the goal, facing it.
fn short_of_goal(dist_m: f64) -> Pose {
    Pose {
        pos: Vec3::new(GOAL_UU - dist_m * UU_PER_METRE, 0.0, 0.0),
        yaw: 0.0,
        pitch: 0.0,
    }
}

/// Walk the simulated pawn to `goal` with a 0.5 m arrival radius.
async fn walk_to(sim: &mut Sim, goal: Vec3) {
    movement::run(
        sim,
        MoveRequest {
            point: Some(client_point(goal.x, goal.y, goal.z)),
            arrival_m: Some(0.5),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
}

#[test]
fn first_sample_replaces_the_prior() {
    let mut m = CoastModel::default();
    assert_eq!(m.coast_s, COAST_PRIOR_S);
    m.learn(5.0, 1.5);
    assert!(
        (m.coast_s - 0.3).abs() < 1e-9,
        "first sample: {}",
        m.coast_s
    );
    assert_eq!(m.samples, 1);
    // Later samples blend with the alpha of 0.3: 0.3 + 0.3 * (0.1 - 0.3).
    m.learn(5.0, 0.5);
    assert!((m.coast_s - 0.24).abs() < 1e-9, "blend: {}", m.coast_s);
    assert_eq!(m.samples, 2);
}

#[test]
fn slow_releases_teach_nothing() {
    let mut m = CoastModel::default();
    m.learn(0.3, 0.1);
    assert_eq!(m.coast_s, COAST_PRIOR_S);
    assert_eq!(m.samples, 0);
}

#[test]
fn coast_is_clamped() {
    let mut m = CoastModel::default();
    m.learn(1.0, 5.0);
    assert!((m.coast_s - 0.6).abs() < 1e-9, "clamp: {}", m.coast_s);
}

#[test]
fn zero_lead_is_todays_controller() {
    let cfg = MoveConfig {
        arrival_m: 0.5,
        ..MoveConfig::default()
    };
    let goal = Vec3::new(1000.0, 0.0, 0.0);
    let mut plain = MoveController::new(cfg, TurnRate::default());
    let mut zero = MoveController::new(cfg, TurnRate::default());
    zero.set_lead(0.0, 300);
    let walk = |c: &mut MoveController| -> Vec<MoveStep> {
        (0..40)
            .map(|i| {
                let pose = Pose {
                    pos: Vec3::new(i as f64 * 20.0, 0.0, 0.0),
                    yaw: 0.0,
                    pitch: 0.0,
                };
                c.step(i as u64 * 100, pose, goal)
            })
            .collect()
    };
    assert_eq!(walk(&mut plain), walk(&mut zero));
}

#[test]
fn lead_releases_early_then_judges_settled() {
    let cfg = MoveConfig {
        arrival_m: 0.5,
        ..MoveConfig::default()
    };
    let goal = Vec3::new(GOAL_UU, 0.0, 0.0);
    let mut c = MoveController::new(cfg, TurnRate::default());
    c.set_lead(1.5, 300);
    // Inside arrival + lead: W goes up, no judgement.
    let s = c.step(0, short_of_goal(1.9), goal);
    assert!(!s.forward && s.outcome.is_none());
    assert!(c.coasting());
    // The pawn coasts in during the hold-off: still no judgement.
    for (now, dist) in [(100, 1.2), (200, 0.9)] {
        let s = c.step(now, short_of_goal(dist), goal);
        assert!(!s.forward && s.outcome.is_none(), "judged during hold-off");
    }
    // After the hold-off, the settled distance decides.
    let s = c.step(300, short_of_goal(0.4), goal);
    assert!(
        matches!(s.outcome, Some(MoveOutcome::Arrived { distance_m }) if (distance_m - 0.4).abs() < 1e-9),
        "settled: {:?}",
        s.outcome
    );

    // Settled outside arrival: the walk goes on, it does not call arrival.
    let mut c = MoveController::new(cfg, TurnRate::default());
    c.set_lead(1.5, 300);
    c.step(0, short_of_goal(1.9), goal);
    let s = c.step(300, short_of_goal(0.8), goal);
    assert!(!c.coasting());
    assert!(!matches!(s.outcome, Some(MoveOutcome::Arrived { .. })));
    assert!(s.forward, "a second approach walks plainly");
}

#[tokio::test]
async fn move_to_lands_inside_arrival_when_coasting() {
    let mut sim = Sim::live();
    sim.lag.coast_s = 0.3;
    sim.coast_model = Some(CoastModel {
        coast_s: 0.3,
        samples: 1,
    });
    let goal = Vec3::new(sim.pos.x + 1000.0, sim.pos.y, sim.pos.z);
    let out = movement::run(
        &mut sim,
        MoveRequest {
            point: Some(client_point(goal.x, goal.y, goal.z)),
            arrival_m: Some(0.5),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["arrived"], true);
    sim.sleep(1000).await;
    let end_m = horizontal_m(sim.pos, goal);
    assert!(end_m <= 0.5, "settled {end_m} m from the goal");
}

#[tokio::test]
async fn move_to_learns_across_calls() {
    let mut sim = Sim::live();
    sim.lag.coast_s = 0.3;
    let first = Vec3::new(sim.pos.x + 1000.0, sim.pos.y, sim.pos.z);
    walk_to(&mut sim, first).await;
    sim.sleep(1000).await;
    let m = sim.coast_model.expect("the first walk stores its model");
    assert_eq!(m.samples, 1);
    assert!((m.coast_s - 0.3).abs() <= 0.05, "learned {}", m.coast_s);

    let second = Vec3::new(sim.pos.x + 1000.0, sim.pos.y, sim.pos.z);
    walk_to(&mut sim, second).await;
    sim.sleep(1000).await;
    let end_m = horizontal_m(sim.pos, second);
    assert!(end_m <= 0.5, "second walk ended {end_m} m from the goal");
}

#[tokio::test]
async fn waypoint_legs_do_not_release() {
    let mut sim = Sim::live();
    sim.lag.coast_s = 0.3;
    sim.coast_model = Some(CoastModel {
        coast_s: 0.3,
        samples: 1,
    });
    let corner = Vec3::new(sim.pos.x + 1000.0, sim.pos.y, sim.pos.z);
    let end = Vec3::new(corner.x + 1000.0, corner.y, corner.z);
    let out = movement::run(
        &mut sim,
        MoveRequest {
            waypoints: vec![client_point(corner.x, corner.y, corner.z)],
            point: Some(client_point(end.x, end.y, end.z)),
            ..MoveRequest::default()
        },
    )
    .await
    .unwrap();
    // Only the final leg coasts: the waypoint is judged as it always was.
    assert!(out["legs"][0].get("coast").is_none());
    assert!(out["legs"][1]["coast"].is_object());
    let corner_m = out["legs"][0]["outcome"]["distance_m"].as_f64().unwrap();
    assert!(corner_m <= 0.5, "waypoint released early: {corner_m} m");
}
