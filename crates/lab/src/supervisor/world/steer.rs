//! Closed-loop controllers for mouse-look turning, walking and facing.
//!
//! Pure state machines: each `step` takes one observation (a pose, a
//! projection) and returns what to press next. The tools run them against
//! the live client; the tests run them against a simulated one.
//!
//! Nothing assumes the mouse-look sensitivity or its sign. [`TurnModel`]
//! starts from a guess and learns *counts per radian* (signed) from what
//! each motion actually did, so a changed sensitivity, an inverted axis or
//! a different camera mode only costs a step or two.

use std::f64::consts::{FRAC_PI_2, PI};

use serde_json::{json, Value};

use super::geometry::{horizontal_m, wrap_pi, yaw_error, Pose, Screen, Vec3};

/// Starting guess for DirectInput counts per radian of camera yaw.
/// Measured on the live client (see live-research-lab.md § World tools);
/// the model re-learns it every run.
pub const DEFAULT_COUNTS_PER_RAD: f64 = 400.0;
/// Motions smaller than this are not used to learn the gain.
const MIN_LEARN_COUNTS: i32 = 15;
/// Yaw changes smaller than this are "did not turn".
const MIN_LEARN_RAD: f64 = 0.02;
const GAIN_BOUNDS: (f64, f64) = (10.0, 50_000.0);

/// Learned mouse-look gain for one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnModel {
    /// Signed counts per radian: positive means +dx increases the angle.
    pub counts_per_rad: f64,
    pub samples: u32,
    pub sign_flips: u32,
}

impl Default for TurnModel {
    fn default() -> Self {
        Self::new(DEFAULT_COUNTS_PER_RAD)
    }
}

impl TurnModel {
    pub fn new(counts_per_rad: f64) -> Self {
        Self {
            counts_per_rad,
            samples: 0,
            sign_flips: 0,
        }
    }

    /// Counts to turn by `angle` radians.
    pub fn counts_for(&self, angle: f64) -> i32 {
        (angle * self.counts_per_rad).round() as i32
    }

    /// Learn from a motion of `dx` counts that turned the angle by `dangle`.
    /// Returns whether the sample was usable.
    pub fn observe(&mut self, dx: i32, dangle: f64) -> bool {
        if dx.abs() < MIN_LEARN_COUNTS || dangle.abs() < MIN_LEARN_RAD {
            return false;
        }
        let k = dx as f64 / dangle;
        let (lo, hi) = GAIN_BOUNDS;
        let k = k.signum() * k.abs().clamp(lo, hi);
        if self.samples == 0 || k.signum() != self.counts_per_rad.signum() {
            if self.samples > 0 {
                self.sign_flips += 1;
            }
            self.counts_per_rad = k;
        } else {
            self.counts_per_rad = 0.5 * self.counts_per_rad + 0.5 * k;
        }
        self.samples += 1;
        true
    }

    pub fn to_json(self) -> Value {
        json!({
            "counts_per_rad": self.counts_per_rad.round(),
            "counts_per_degree": (self.counts_per_rad.to_radians() * 100.0).round() / 100.0,
            "samples": self.samples,
            "sign_flips": self.sign_flips,
        })
    }
}

/// Tuning for [`MoveController`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveConfig {
    /// Done when the horizontal distance is at most this, metres.
    pub arrival_m: f64,
    /// Stuck when walking this long without getting `stuck_progress_m` closer.
    pub stuck_window_ms: u64,
    pub stuck_progress_m: f64,
    /// Unstick attempts (jump, strafe right, strafe left) before giving up.
    pub max_unsticks: u32,
    /// Heading errors above this turn in place first (when the pawn turns
    /// standing still).
    pub turn_in_place_rad: f64,
    /// Largest correction per step, radians (limits overshoot on a bad gain).
    pub max_step_rad: f64,
    /// Heading errors below this need no correction.
    pub deadband_rad: f64,
}

impl Default for MoveConfig {
    fn default() -> Self {
        Self {
            arrival_m: 1.5,
            stuck_window_ms: 2500,
            stuck_progress_m: 0.5,
            max_unsticks: 3,
            turn_in_place_rad: 1.0,
            max_step_rad: 1.2,
            deadband_rad: 0.05,
        }
    }
}

/// A way out of a snag, tried in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unstick {
    Jump,
    StrafeRight,
    StrafeLeft,
}

impl Unstick {
    pub fn nth(i: u32) -> Self {
        match i % 3 {
            0 => Unstick::Jump,
            1 => Unstick::StrafeRight,
            _ => Unstick::StrafeLeft,
        }
    }

    /// The key it presses (movement keys per the stock `SGWInput.ini`).
    pub fn key(self) -> &'static str {
        match self {
            Unstick::Jump => "Space",
            Unstick::StrafeRight => "D",
            Unstick::StrafeLeft => "A",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Unstick::Jump => "jump",
            Unstick::StrafeRight => "strafe_right",
            Unstick::StrafeLeft => "strafe_left",
        }
    }
}

/// How a walk ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveOutcome {
    Arrived { distance_m: f64 },
    Stuck { distance_m: f64, attempts: u32 },
}

/// What to do this step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveStep {
    /// Mouse-look counts to send now.
    pub dx: i32,
    /// Hold the forward key after this step.
    pub forward: bool,
    pub unstick: Option<Unstick>,
    pub outcome: Option<MoveOutcome>,
    pub distance_m: f64,
    pub heading_error: f64,
}

/// Walk toward a point: turn with mouse-look, hold forward, detect snags.
#[derive(Debug, Clone)]
pub struct MoveController {
    pub cfg: MoveConfig,
    pub turn: TurnModel,
    /// Does the pawn turn while standing? `None` until a motion shows it.
    pub standing_turn: Option<bool>,
    pub unsticks: u32,
    last: Option<(Pose, i32, bool)>,
    best_m: f64,
    best_at: u64,
}

impl MoveController {
    pub fn new(cfg: MoveConfig, turn: TurnModel) -> Self {
        Self {
            cfg,
            turn,
            standing_turn: None,
            unsticks: 0,
            last: None,
            best_m: f64::INFINITY,
            best_at: 0,
        }
    }

    pub fn step(&mut self, now_ms: u64, pose: Pose, target: Vec3) -> MoveStep {
        let dist = horizontal_m(pose.pos, target);
        let err = yaw_error(pose.pos, pose.yaw, target);
        let mut out = MoveStep {
            dx: 0,
            forward: false,
            unstick: None,
            outcome: None,
            distance_m: dist,
            heading_error: err,
        };

        // Learn from what the previous step's motion did.
        let was_forward = match self.last {
            Some((prev, dx, fwd)) => {
                let dyaw = wrap_pi(pose.yaw - prev.yaw);
                let used = self.turn.observe(dx, dyaw);
                if !fwd && dx.abs() >= MIN_LEARN_COUNTS && self.standing_turn.is_none() {
                    self.standing_turn = Some(used);
                }
                fwd
            }
            None => false,
        };

        if dist <= self.cfg.arrival_m {
            out.outcome = Some(MoveOutcome::Arrived { distance_m: dist });
            self.last = None;
            return out;
        }

        // Progress is only expected while walking: turning in place or a
        // fresh start restarts the window.
        if !was_forward || dist < self.best_m - self.cfg.stuck_progress_m {
            self.best_m = dist;
            self.best_at = now_ms;
        } else if now_ms.saturating_sub(self.best_at) >= self.cfg.stuck_window_ms {
            if self.unsticks >= self.cfg.max_unsticks {
                out.outcome = Some(MoveOutcome::Stuck {
                    distance_m: dist,
                    attempts: self.unsticks,
                });
                return out;
            }
            out.unstick = Some(Unstick::nth(self.unsticks));
            self.unsticks += 1;
            self.best_m = dist;
            self.best_at = now_ms;
        }

        if err.abs() > self.cfg.deadband_rad {
            // 0.8: undershoot a little; the next step finishes the turn.
            let angle = err.clamp(-self.cfg.max_step_rad, self.cfg.max_step_rad) * 0.8;
            out.dx = self.turn.counts_for(angle);
        }
        // Turn in place for a big error when the pawn can; otherwise (or
        // when it cannot) keep walking so the turn takes effect.
        out.forward = err.abs() <= self.cfg.turn_in_place_rad || self.standing_turn == Some(false);
        self.last = Some((pose, out.dx, out.forward));
        out
    }
}

/// Tuning for [`FaceController`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceConfig {
    /// Horizontal field of view assumed to turn pixels into angles (UE3's
    /// default 90 degrees); only the first step depends on it, since the
    /// gain is re-learned from the target's motion on screen.
    pub hfov: f64,
    /// Done when the target is within this fraction of the half width of
    /// the centre and vertically on screen.
    pub x_tolerance: f64,
    pub edge_margin_px: f64,
    /// Search step while the target is behind the camera, radians.
    pub search_step: f64,
}

impl Default for FaceConfig {
    fn default() -> Self {
        Self {
            hfov: FRAC_PI_2,
            x_tolerance: 0.25,
            edge_margin_px: 40.0,
            search_step: FRAC_PI_2,
        }
    }
}

/// What the face controller wants next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaceStep {
    Done,
    Look { dx: i32, dy: i32 },
}

/// Turn the camera until a world point sits near the screen centre, from
/// its projected pixel (and, while it is behind the camera, from a yaw
/// estimate when one is available).
#[derive(Debug, Clone)]
pub struct FaceController {
    pub cfg: FaceConfig,
    pub yaw: TurnModel,
    pub pitch: TurnModel,
    last: Option<(f64, f64, i32, i32)>,
    pub searches: u32,
}

impl FaceController {
    pub fn new(cfg: FaceConfig, yaw: TurnModel) -> Self {
        let pitch = TurnModel::new(-yaw.counts_per_rad.abs());
        Self {
            cfg,
            yaw,
            pitch,
            last: None,
            searches: 0,
        }
    }

    /// Angles (right, up) of a pixel off the view axis.
    fn pixel_angles(&self, screen: Screen, p: (f64, f64)) -> (f64, f64) {
        let (cx, cy) = screen.centre();
        let focal = cx / (self.cfg.hfov / 2.0).tan();
        (((p.0 - cx) / focal).atan(), ((cy - p.1) / focal).atan())
    }

    /// `pixel`: the projection (None when behind the camera).
    /// `yaw_hint`: radians the camera should turn, when known otherwise.
    pub fn step(
        &mut self,
        screen: Screen,
        pixel: Option<(f64, f64)>,
        yaw_hint: Option<f64>,
    ) -> FaceStep {
        let visible = pixel.filter(|p| p.0.is_finite() && p.1.is_finite());
        match visible {
            Some(p) => {
                let (ax, ay) = self.pixel_angles(screen, p);
                if let Some((px, py, dx, dy)) = self.last {
                    // The target is fixed: its angle moves opposite the camera.
                    self.yaw.observe(dx, px - ax);
                    self.pitch.observe(dy, py - ay);
                }
                let centred = screen.x_error(p).abs() <= self.cfg.x_tolerance
                    && screen.contains(p, self.cfg.edge_margin_px);
                if centred {
                    self.last = None;
                    return FaceStep::Done;
                }
                let dx = self.yaw.counts_for(ax * 0.9);
                let dy = if screen.contains((screen.centre().0, p.1), self.cfg.edge_margin_px) {
                    0
                } else {
                    self.pitch.counts_for(ay * 0.9)
                };
                self.last = Some((ax, ay, dx, dy));
                FaceStep::Look { dx, dy }
            }
            None => {
                self.last = None;
                self.searches += 1;
                let angle = match yaw_hint {
                    Some(h) if h.abs() > 0.05 => h.clamp(-PI, PI),
                    _ => self.cfg.search_step,
                };
                FaceStep::Look {
                    dx: self.yaw.counts_for(angle),
                    dy: 0,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut c = MoveController::new(MoveConfig::default(), TurnModel::default());
        let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0));
        assert!(matches!(s.outcome, Some(MoveOutcome::Arrived { .. })));
    }

    #[test]
    fn move_controller_turns_toward_and_walks() {
        let mut c = MoveController::new(MoveConfig::default(), TurnModel::new(400.0));
        // Target straight ahead: walk, no turn.
        let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(2000.0, 0.0, 0.0));
        assert!(s.forward);
        assert_eq!(s.dx, 0);
        // Target 90 degrees left of a fresh controller: turn in place.
        let mut c = MoveController::new(MoveConfig::default(), TurnModel::new(400.0));
        let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(0.0, 2000.0, 0.0));
        assert!(!s.forward);
        assert!(s.dx > 0);
    }

    /// Standing still and turning does not rotate the pawn: learn that and
    /// walk while turning instead of spinning the camera forever.
    #[test]
    fn move_controller_walks_while_turning_when_standing_turns_do_nothing() {
        let mut c = MoveController::new(MoveConfig::default(), TurnModel::new(400.0));
        let target = Vec3::new(0.0, 2000.0, 0.0);
        let s1 = c.step(0, pose(0.0, 0.0, 0.0), target);
        assert!(!s1.forward && s1.dx != 0);
        let s2 = c.step(100, pose(0.0, 0.0, 0.0), target);
        assert_eq!(c.standing_turn, Some(false));
        assert!(s2.forward);
    }

    #[test]
    fn move_controller_unsticks_then_gives_up() {
        let cfg = MoveConfig {
            max_unsticks: 2,
            ..MoveConfig::default()
        };
        let mut c = MoveController::new(cfg, TurnModel::new(400.0));
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
}
