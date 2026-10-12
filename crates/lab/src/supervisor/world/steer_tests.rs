//! Tests for the steering state machines in `steer.rs`.

use std::f64::consts::FRAC_PI_2;

use super::geometry::{Pose, Screen, Vec3};
use super::steer::*;

#[test]
fn turn_model_learns_gain_and_sign() {
    let mut m = TurnModel::new(400.0);
    // The real axis is inverted and twice as sensitive: 200 counts/rad, negative.
    assert!(m.observe(200, -1.0));
    assert!((m.counts_per_rad + 200.0).abs() < 1e-9);
    assert!(m.observe(-100, 0.5));
    assert!((m.counts_per_rad + 200.0).abs() < 1e-9);
    // Too small to learn from.
    assert!(!m.observe(5, 1.0));
    assert!(!m.observe(300, 0.001));
    assert_eq!(m.samples, 2);
}

#[test]
fn turn_model_counts_track_the_learned_gain() {
    let m = TurnModel::new(-250.0);
    assert_eq!(m.counts_for(1.0), -250);
    assert_eq!(m.counts_for(-0.5), 125);
}

fn pose(x: f64, y: f64, yaw: f64) -> Pose {
    Pose {
        pos: Vec3::new(x, y, 0.0),
        yaw,
        pitch: 0.0,
    }
}

#[test]
fn move_controller_arrives_inside_the_radius() {
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0));
    assert!(matches!(s.outcome, Some(MoveOutcome::Arrived { .. })));
}

#[test]
fn move_controller_holds_the_key_that_turns_toward_the_goal() {
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    // Straight ahead: walk, no turn.
    let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(2000.0, 0.0, 0.0));
    assert!(s.forward);
    assert_eq!(s.turn, TurnCmd::Off);
    // 90 degrees toward +Y (yaw must rise): hold Right, stand while turning.
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(0.0, 2000.0, 0.0));
    assert!(!s.forward);
    assert_eq!(s.turn, TurnCmd::Hold(TurnKey::Right));
    // 90 degrees the other way: hold Left.
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(0.0, -2000.0, 0.0));
    assert_eq!(s.turn, TurnCmd::Hold(TurnKey::Left));
}

#[test]
fn a_small_error_is_a_timed_pulse() {
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    // 0.2 rad off: 0.2 / 1.92 * 1000 * 0.9 = 94 ms.
    let t = Vec3::new(2000.0 * 0.2f64.cos(), 2000.0 * 0.2f64.sin(), 0.0);
    let s = c.step(0, pose(0.0, 0.0, 0.0), t);
    assert!(s.forward);
    assert_eq!(s.turn, TurnCmd::Pulse(TurnKey::Right, 94));
    // Inside the deadband: nothing.
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    let t = Vec3::new(2000.0 * 0.03f64.cos(), 2000.0 * 0.03f64.sin(), 0.0);
    assert_eq!(c.step(0, pose(0.0, 0.0, 0.0), t).turn, TurnCmd::Off);
}

#[test]
fn turn_rate_learns_the_rate_and_flips_a_wrong_key() {
    let mut r = TurnRate::new(1.0);
    // Right held 100 ms, yaw rose 0.2 rad: 2 rad/s.
    assert!(r.observe(TurnKey::Right, 0.2, 100));
    assert!((r.rad_s - 2.0).abs() < 1e-9);
    // Too short or too small teaches nothing.
    assert!(!r.observe(TurnKey::Right, 0.2, 10));
    assert!(!r.observe(TurnKey::Right, 0.001, 100));
    // Two wrong-way samples flip the sign; one does not.
    assert!(!r.observe(TurnKey::Right, -0.2, 100));
    assert_eq!(r.right_sign, 1.0);
    assert!(!r.observe(TurnKey::Right, -0.2, 100));
    assert_eq!(r.right_sign, -1.0);
    assert_eq!(r.sign_flips, 1);
    assert_eq!(r.key_for(1.0), TurnKey::Left);
}

#[test]
fn move_controller_unsticks_then_gives_up() {
    let cfg = MoveConfig {
        max_unsticks: 2,
        ..MoveConfig::default()
    };
    let mut c = MoveController::new(cfg, TurnRate::default());
    let target = Vec3::new(5000.0, 0.0, 0.0);
    let p = pose(0.0, 0.0, 0.0);
    let mut unsticks = Vec::new();
    let mut outcome = None;
    for t in (0..20_000).step_by(100) {
        let s = c.step(t, p, target);
        if let Some(u) = s.unstick {
            unsticks.push(u);
        }
        if s.outcome.is_some() {
            outcome = s.outcome;
            break;
        }
    }
    assert_eq!(unsticks, vec![Unstick::Jump, Unstick::StrafeRight]);
    assert!(matches!(
        outcome,
        Some(MoveOutcome::Stuck { attempts: 2, .. })
    ));
}

/// Moving away is not a snag: the controller must give up with
/// `Diverged` long before the stuck window or the walk's timeout.
#[test]
fn move_controller_gives_up_when_it_walks_away() {
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    let target = Vec3::new(2000.0, 0.0, 0.0); // 20 m ahead
    let mut outcome = None;
    // The pawn walks backwards 1 m a step (100 uu).
    for k in 0..30u64 {
        let s = c.step(k * 100, pose(-100.0 * k as f64, 0.0, 0.0), target);
        if s.outcome.is_some() {
            outcome = s.outcome;
            break;
        }
    }
    match outcome {
        Some(MoveOutcome::Diverged { distance_m, best_m }) => {
            assert!(distance_m > best_m + 6.0);
            assert!((best_m - 20.0).abs() < 0.1);
        }
        other => panic!("expected Diverged, got {other:?}"),
    }
}

/// A target that is simply approached never diverges.
#[test]
fn move_controller_does_not_diverge_while_closing() {
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    let target = Vec3::new(2000.0, 0.0, 0.0);
    for k in 0..15u64 {
        let s = c.step(k * 100, pose(100.0 * k as f64, 0.0, 0.0), target);
        assert!(!matches!(s.outcome, Some(MoveOutcome::Diverged { .. })));
    }
}

#[test]
fn ten_held_ticks_without_a_turn_end_the_walk() {
    let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
    let target = Vec3::new(0.0, 2000.0, 0.0);
    let mut outcome = None;
    for k in 0..30u64 {
        let s = c.step(k * 100, pose(0.0, 0.0, 0.0), target);
        if s.outcome.is_some() {
            outcome = s.outcome;
            break;
        }
    }
    assert!(
        matches!(outcome, Some(MoveOutcome::NoTurn { ticks: 10 })),
        "{outcome:?}"
    );
}

#[test]
fn face_controller_centres_and_learns_from_pixels() {
    let screen = Screen {
        w: 1000.0,
        h: 800.0,
    };
    let mut f = FaceController::new(FaceConfig::default(), TurnModel::new(400.0));
    assert_eq!(f.step(screen, Some((520.0, 400.0)), None), FaceStep::Done);
    match f.step(screen, Some((900.0, 400.0)), None) {
        FaceStep::Look { dx, dy } => {
            assert!(dx > 0);
            assert_eq!(dy, 0);
        }
        FaceStep::Done => panic!("far right is not centred"),
    }
}

#[test]
fn face_controller_searches_when_behind() {
    let screen = Screen {
        w: 1000.0,
        h: 800.0,
    };
    let mut f = FaceController::new(FaceConfig::default(), TurnModel::new(400.0));
    match f.step(screen, None, Some(-2.0)) {
        FaceStep::Look { dx, .. } => assert_eq!(dx, -800),
        FaceStep::Done => panic!(),
    }
    match f.step(screen, None, None) {
        FaceStep::Look { dx, .. } => assert_eq!(dx, (FRAC_PI_2 * 400.0).round() as i32),
        FaceStep::Done => panic!(),
    }
    assert_eq!(f.searches, 2);
}

#[test]
fn face_controller_seeds_its_gains_from_the_camera_readout() {
    let mut f = FaceController::new(FaceConfig::default(), TurnModel::default());
    // Gain 20 rotator units per count: 65536 / TAU / 20 = 521.5 counts per radian.
    f.seed_gains(Some(20.0), Some(20.0));
    assert!((f.yaw.counts_per_rad - 521.5).abs() < 0.1);
    assert!((f.pitch.counts_per_rad + 521.5).abs() < 0.1);
    // An inverted pitch axis (flag 0x8) reads a negative gain and flips the sign.
    let mut g = FaceController::new(FaceConfig::default(), TurnModel::default());
    g.seed_gains(Some(-20.0), Some(-20.0));
    assert!((g.yaw.counts_per_rad + 521.5).abs() < 0.1);
    assert!((g.pitch.counts_per_rad - 521.5).abs() < 0.1);
    // A gain read before the game loads its settings is 0: keep the guess.
    let mut h = FaceController::new(FaceConfig::default(), TurnModel::new(400.0));
    h.seed_gains(Some(0.0), None);
    assert_eq!(h.yaw.counts_per_rad, 400.0);
    assert_eq!(h.pitch.counts_per_rad, -400.0);
}
