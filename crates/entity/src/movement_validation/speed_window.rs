//! Per-entity sliding window for the speed warning (#1249): client packets
//! arrive jittered (25 ms then 175 ms), so one packet's distance/dt
//! overstates speed by up to 4x. The window sums distance and dt over at
//! least SPEED_WINDOW of recent packets.
use std::collections::VecDeque;
use std::time::Duration;

/// How much recent movement the speed warning averages over.
pub const SPEED_WINDOW: Duration = Duration::from_millis(250);
/// Moves shorter than this (world units) never warn (standing still,
/// rotating in place, float noise).
pub const MIN_SPEED_DISTANCE: f32 = 0.01;
/// Cap on samples kept, so a flood of zero-dt packets cannot grow the deque.
const MAX_SAMPLES: usize = 32;

#[derive(Debug, Default)]
pub(super) struct SpeedWindow {
    samples: VecDeque<(f32, f32)>, // (distance, dt_secs)
}

impl SpeedWindow {
    /// Add a sample and return (summed distance, summed dt) over the window.
    /// Drops the oldest samples while the rest still cover SPEED_WINDOW.
    pub(super) fn push(&mut self, distance: f32, dt_secs: f32) -> (f32, f32) {
        self.samples.push_back((distance, dt_secs.max(0.0)));
        while self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
        let window = SPEED_WINDOW.as_secs_f32();
        loop {
            let total: f32 = self.samples.iter().map(|s| s.1).sum();
            let Some(&(_, front_dt)) = self.samples.front() else {
                break;
            };
            if self.samples.len() > 1 && total - front_dt >= window {
                self.samples.pop_front();
            } else {
                break;
            }
        }
        self.samples
            .iter()
            .fold((0.0, 0.0), |(d, t), s| (d + s.0, t + s.1))
    }
}
