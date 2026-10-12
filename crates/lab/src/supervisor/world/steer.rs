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

use std::f64::consts::{FRAC_PI_2, PI, TAU};

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
/// Rotator units in one radian (65536 per turn): the camera's `gain` is
/// rotator units per mouse count.
const UNITS_PER_RAD: f64 = 65536.0 / TAU;

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

/// Turn-rate model for the pawn's turn keys.
///
/// Live: about 1.92 rad/s (110 deg/s), the same standing and walking.
pub const DEFAULT_TURN_RATE: f64 = 1.92;
const RATE_BOUNDS: (f64, f64) = (0.2, 10.0);
/// A hold shorter than this, or a turn smaller than [`MIN_RATE_RAD`], teaches nothing.
const MIN_RATE_DT_MS: u64 = 50;
const MIN_RATE_RAD: f64 = 0.02;
/// Under this much yaw change over a held tick the pawn "did not turn".
const NO_TURN_RAD: f64 = 0.005;
const MIN_PULSE_MS: u64 = 20;
const MAX_PULSE_MS: u64 = 300;

/// A turn key, by what it does to the yaw once the sign is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnKey {
    Left,
    Right,
}

/// What to do with the turn keys this step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnCmd {
    /// Neither key down.
    Off,
    /// Keep this key down until the next step.
    Hold(TurnKey),
    /// Press this key for this many ms inside the step, then let go.
    Pulse(TurnKey, u64),
}

/// Learned turn rate and key sign.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnRate {
    /// Radians per second while a turn key is held.
    pub rad_s: f64,
    /// +1.0: the Right key raises the yaw (stock: E turns toward +Y).
    pub right_sign: f64,
    pub samples: u32,
    pub sign_flips: u32,
    wrong_run: u32,
}

impl Default for TurnRate {
    fn default() -> Self {
        Self::new(DEFAULT_TURN_RATE)
    }
}

impl TurnRate {
    pub fn new(rad_s: f64) -> Self {
        Self {
            rad_s,
            right_sign: 1.0,
            samples: 0,
            sign_flips: 0,
            wrong_run: 0,
        }
    }

    fn sign_of(&self, key: TurnKey) -> f64 {
        match key {
            TurnKey::Right => self.right_sign,
            TurnKey::Left => -self.right_sign,
        }
    }

    /// The key that turns toward a heading error `err` (positive = the yaw
    /// must rise).
    pub fn key_for(&self, err: f64) -> TurnKey {
        if err * self.right_sign > 0.0 {
            TurnKey::Right
        } else {
            TurnKey::Left
        }
    }

    /// Learn from `key` held for `dt_ms` while the yaw changed by `dyaw`.
    /// Two samples in a row that turned the wrong way flip the key sign.
    /// Returns whether the sample taught the rate.
    pub fn observe(&mut self, key: TurnKey, dyaw: f64, dt_ms: u64) -> bool {
        if dt_ms < MIN_RATE_DT_MS || dyaw.abs() < MIN_RATE_RAD {
            return false;
        }
        let along = dyaw * self.sign_of(key);
        if along < 0.0 {
            self.wrong_run += 1;
            if self.wrong_run >= 2 {
                self.right_sign = -self.right_sign;
                self.sign_flips += 1;
                self.wrong_run = 0;
            }
            return false;
        }
        self.wrong_run = 0;
        let r = (along / (dt_ms as f64 / 1000.0)).clamp(RATE_BOUNDS.0, RATE_BOUNDS.1);
        self.rad_s = if self.samples == 0 {
            r
        } else {
            0.5 * self.rad_s + 0.5 * r
        };
        self.samples += 1;
        true
    }

    pub fn to_json(self) -> Value {
        json!({
            "rad_s": (self.rad_s * 100.0).round() / 100.0,
            "deg_s": (self.rad_s.to_degrees() * 10.0).round() / 10.0,
            "right_sign": self.right_sign,
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
    /// Heading errors above this turn standing still first.
    pub turn_in_place_rad: f64,
    /// Errors under this are corrected with a timed key pulse, above it a hold.
    pub pulse_below_rad: f64,
    /// Heading errors below this need no correction.
    pub deadband_rad: f64,
    /// Give up when walking has taken the avatar this far (metres) beyond the
    /// closest it has been to the goal: it is walking away, not snagged.
    pub diverge_m: f64,
    /// Give up after this many held-key ticks in a row with no yaw change.
    pub no_turn_ticks: u32,
}

impl Default for MoveConfig {
    fn default() -> Self {
        Self {
            arrival_m: 1.5,
            stuck_window_ms: 2500,
            stuck_progress_m: 0.5,
            max_unsticks: 3,
            turn_in_place_rad: 1.0,
            pulse_below_rad: 0.35,
            deadband_rad: 0.05,
            diverge_m: 6.0,
            no_turn_ticks: 10,
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
    Arrived {
        distance_m: f64,
    },
    Stuck {
        distance_m: f64,
        attempts: u32,
    },
    /// Farther from the goal than ever after walking: a wall deflecting the
    /// pawn, or a goal that is running away.
    Diverged {
        distance_m: f64,
        best_m: f64,
    },
    /// The turn keys were held for `ticks` ticks and the pawn's yaw never
    /// moved: rooted (dialog, cast, stun), or the bindings are not these keys.
    NoTurn {
        ticks: u32,
    },
}

/// What to do this step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveStep {
    pub turn: TurnCmd,
    /// Hold the forward key after this step.
    pub forward: bool,
    pub unstick: Option<Unstick>,
    pub outcome: Option<MoveOutcome>,
    pub distance_m: f64,
    pub heading_error: f64,
}

/// Walk toward a point: turn with the turn keys, hold forward, detect snags.
#[derive(Debug, Clone)]
pub struct MoveController {
    pub cfg: MoveConfig,
    pub rate: TurnRate,
    pub unsticks: u32,
    last: Option<(Pose, u64, TurnCmd, bool)>,
    no_turn: u32,
    best_m: f64,
    best_at: u64,
    min_m: f64,
}

impl MoveController {
    pub fn new(cfg: MoveConfig, rate: TurnRate) -> Self {
        Self {
            cfg,
            rate,
            unsticks: 0,
            last: None,
            no_turn: 0,
            best_m: f64::INFINITY,
            best_at: 0,
            min_m: f64::INFINITY,
        }
    }

    pub fn step(&mut self, now_ms: u64, pose: Pose, target: Vec3) -> MoveStep {
        let dist = horizontal_m(pose.pos, target);
        let err = yaw_error(pose.pos, pose.yaw, target);
        let mut out = MoveStep {
            turn: TurnCmd::Off,
            forward: false,
            unstick: None,
            outcome: None,
            distance_m: dist,
            heading_error: err,
        };

        // Learn from what the previous step's held key did.
        let was_forward = match self.last {
            Some((prev, t, cmd, fwd)) => {
                if let TurnCmd::Hold(k) = cmd {
                    let dyaw = wrap_pi(pose.yaw - prev.yaw);
                    self.rate.observe(k, dyaw, now_ms.saturating_sub(t));
                    self.no_turn = if dyaw.abs() < NO_TURN_RAD {
                        self.no_turn + 1
                    } else {
                        0
                    };
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
        if self.no_turn >= self.cfg.no_turn_ticks {
            out.outcome = Some(MoveOutcome::NoTurn {
                ticks: self.no_turn,
            });
            return out;
        }

        // Never closer than this: remembered across turns and unsticks.
        self.min_m = self.min_m.min(dist);
        if was_forward && dist > self.min_m + self.cfg.diverge_m {
            out.outcome = Some(MoveOutcome::Diverged {
                distance_m: dist,
                best_m: self.min_m,
            });
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
            let key = self.rate.key_for(err);
            out.turn = if err.abs() < self.cfg.pulse_below_rad {
                // 0.9: undershoot a little (the key has a short tail).
                let ms = (err.abs() / self.rate.rad_s * 1000.0 * 0.9).round() as u64;
                TurnCmd::Pulse(key, ms.clamp(MIN_PULSE_MS, MAX_PULSE_MS))
            } else {
                TurnCmd::Hold(key)
            };
        }
        out.forward = err.abs() <= self.cfg.turn_in_place_rad;
        self.last = Some((pose, now_ms, out.turn, out.forward));
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

    /// Start from the camera's own signed gains (`yaw_gain` / `pitch_gain` of
    /// `client_camera`'s readout, rotator units per count) instead of guesses.
    /// A target is fixed, so turning the camera one way moves it the other:
    /// yaw counts per radian is `U / yaw_gain` and pitch counts per radian is
    /// `-U / pitch_gain` (looking down raises the pitch offset and moves the
    /// target up the screen). An inverted axis has a negative gain, so the
    /// first look already goes the right way. A gain that is zero (settings
    /// not loaded yet) or not finite changes nothing, and a model that has
    /// already learned from a pixel keeps what it learned.
    pub fn seed_gains(&mut self, yaw_gain: Option<f64>, pitch_gain: Option<f64>) {
        let seed = |v: f64| v.signum() * v.abs().clamp(GAIN_BOUNDS.0, GAIN_BOUNDS.1);
        let usable = |g: &f64| g.is_finite() && g.abs() >= 1e-3;
        if let Some(g) = yaw_gain.filter(usable) {
            if self.yaw.samples == 0 {
                self.yaw = TurnModel::new(seed(UNITS_PER_RAD / g));
            }
        }
        if let Some(g) = pitch_gain.filter(usable) {
            if self.pitch.samples == 0 {
                self.pitch = TurnModel::new(seed(-UNITS_PER_RAD / g));
            }
        }
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
