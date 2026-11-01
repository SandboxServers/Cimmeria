//! The walker's coast model: how far the pawn keeps moving after `W` goes up.
//!
//! `client_move_to` releases `W` early by the learned coast distance (the
//! lead), holds off while the pawn coasts in, and calls the walk arrived only
//! where the pawn settled. [`CoastGlue`] feeds the model from each release on
//! the final leg, and the model is kept between calls on the supervisor.

use serde_json::{json, Value};

use super::geometry::{horizontal_m, Vec3};

/// Coast time before any release has been measured, seconds.
pub const COAST_PRIOR_S: f64 = 0.12;
/// Weight of each later sample in the running coast time.
pub const COAST_ALPHA: f64 = 0.3;
/// No coast is believed longer than this, seconds.
pub const COAST_MAX_S: f64 = 0.6;
/// A release slower than this teaches nothing (metres per second).
pub const MIN_LEARN_SPEED_MPS: f64 = 0.5;
/// The shortest hold-off after a release, ms.
pub const MIN_HOLDOFF_MS: u64 = 300;

/// The learned coast time, seconds, and how many releases taught it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoastModel {
    pub coast_s: f64,
    pub samples: u32,
}

impl Default for CoastModel {
    fn default() -> Self {
        Self {
            coast_s: COAST_PRIOR_S,
            samples: 0,
        }
    }
}

impl CoastModel {
    /// How far ahead of the goal to release `W` at `speed_mps`, metres.
    pub fn lead_m(&self, speed_mps: f64) -> f64 {
        speed_mps.max(0.0) * self.coast_s
    }

    /// How long to wait after a release before judging arrival, ms.
    pub fn holdoff_ms(&self) -> u64 {
        MIN_HOLDOFF_MS.max((self.coast_s * 1000.0).ceil() as u64 + 100)
    }

    /// The pawn moved `coasted_m` after a release at `speed_mps`.
    pub fn learn(&mut self, speed_mps: f64, coasted_m: f64) {
        if speed_mps < MIN_LEARN_SPEED_MPS {
            return;
        }
        let obs = (coasted_m / speed_mps).clamp(0.0, COAST_MAX_S);
        if self.samples == 0 {
            self.coast_s = obs;
        } else {
            self.coast_s += COAST_ALPHA * (obs - self.coast_s);
        }
        self.samples += 1;
    }

    pub fn as_json(&self) -> Value {
        json!({ "coast_s": self.coast_s, "samples": self.samples })
    }
}

/// Horizontal speed from consecutive poses.
#[derive(Debug, Clone, Default)]
pub struct SpeedEst {
    last: Option<(Vec3, u64)>,
    pub mps: f64,
}

impl SpeedEst {
    /// Record a pose; an interval of zero milliseconds is ignored.
    pub fn observe(&mut self, pos: Vec3, now_ms: u64) {
        let Some((prev, t)) = self.last else {
            self.last = Some((pos, now_ms));
            return;
        };
        let dt_ms = now_ms.saturating_sub(t);
        if dt_ms == 0 {
            return;
        }
        self.mps = horizontal_m(prev, pos) / (dt_ms as f64 / 1000.0);
        self.last = Some((pos, now_ms));
    }
}

/// The final leg's coast bookkeeping: the model, the speed, the release.
#[derive(Debug, Clone)]
pub struct CoastGlue {
    pub model: CoastModel,
    speed: SpeedEst,
    /// The release point and the speed it was released at.
    release: Option<(Vec3, f64)>,
    /// The lead this leg used, metres.
    used_lead_m: f64,
    /// Coasted distance once the pawn settled; `Some` ends the predictions.
    settled_m: Option<f64>,
}

impl CoastGlue {
    pub fn new(model: CoastModel) -> Self {
        Self {
            model,
            speed: SpeedEst::default(),
            release: None,
            used_lead_m: 0.0,
            settled_m: None,
        }
    }

    /// The lead for the next step. Zero once the pawn has settled, so a
    /// second approach on this leg walks plainly.
    pub fn lead_m(&self) -> f64 {
        if self.settled_m.is_some() {
            return 0.0;
        }
        self.model.lead_m(self.speed.mps)
    }

    pub fn holdoff_ms(&self) -> u64 {
        self.model.holdoff_ms()
    }

    pub fn on_pose(&mut self, pos: Vec3, now_ms: u64) {
        self.speed.observe(pos, now_ms);
    }

    /// The walker just released `W` at `pos`.
    pub fn on_released(&mut self, pos: Vec3) {
        self.release = Some((pos, self.speed.mps));
        self.used_lead_m = self.lead_m();
    }

    /// The hold-off ended with the pawn at `pos`: learn the coast it made.
    pub fn on_settled(&mut self, pos: Vec3) {
        if let Some((at, speed_mps)) = self.release.take() {
            let coasted_m = horizontal_m(at, pos);
            self.model.learn(speed_mps, coasted_m);
            self.settled_m = Some(coasted_m);
        }
    }

    /// The final leg's `coast` report.
    pub fn leg_json(&self) -> Value {
        let mut v = self.model.as_json();
        v["lead_m"] = json!(self.used_lead_m);
        if let Some(m) = self.settled_m {
            v["settled_m"] = json!(m);
        }
        v
    }
}

/// The early release of one walker: the lead, the hold-off length, and when
/// the hold-off ends.
#[derive(Debug, Clone, Copy, Default)]
pub struct Lead {
    pub lead_m: f64,
    pub hold_ms: u64,
    until_ms: Option<u64>,
}

impl Lead {
    pub fn set(&mut self, lead_m: f64, hold_ms: u64) {
        self.lead_m = lead_m;
        self.hold_ms = hold_ms;
    }

    pub fn coasting(&self) -> bool {
        self.until_ms.is_some()
    }

    /// True while the hold-off runs. When it has ended, forget the lead so the
    /// walk judges arrival plainly.
    pub fn holding(&mut self, now_ms: u64) -> bool {
        match self.until_ms {
            Some(until) if now_ms < until => true,
            Some(_) => {
                self.until_ms = None;
                self.lead_m = 0.0;
                false
            }
            None => false,
        }
    }

    /// True when this step releases `W`: inside `arrival_m + lead_m`.
    pub fn release(&mut self, now_ms: u64, dist_m: f64, arrival_m: f64) -> bool {
        if self.lead_m > 0.0 && dist_m <= arrival_m + self.lead_m {
            self.until_ms = Some(now_ms + self.hold_ms);
            return true;
        }
        false
    }
}
