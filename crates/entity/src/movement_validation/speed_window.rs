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
///
/// Deliberate bound: a sample ages out only once the samples after it cover
/// SPEED_WINDOW of server-clock dt, so zero-dt packets (a burst released after
/// a lag spike) stay in the window. The sum of such a burst is at most
/// `MAX_SAMPLES` hops over the clamped 0.25 s divisor; the cap drops the
/// oldest hops beyond that rather than growing without bound. The check only
/// warns, it never rejects.
const MAX_SAMPLES: usize = 32;

/// One accepted move: its distance and the server-clock dt it covered.
#[derive(Debug, Clone, Copy)]
struct Sample {
    distance: f32,
    dt_secs: f32,
}

#[derive(Debug, Default)]
pub(super) struct SpeedWindow {
    samples: VecDeque<Sample>,
}

impl SpeedWindow {
    /// Add a sample and return (summed distance, summed dt) over the window.
    /// Drops the oldest samples while the rest still cover SPEED_WINDOW.
    /// A negative dt counts as zero.
    pub(super) fn push(&mut self, distance: f32, dt_secs: f32) -> (f32, f32) {
        self.samples.push_back(Sample {
            distance,
            dt_secs: dt_secs.max(0.0),
        });
        while self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
        let window = SPEED_WINDOW.as_secs_f32();
        let mut total: f32 = self.samples.iter().map(|s| s.dt_secs).sum();
        while let Some(front) = self.samples.front() {
            if self.samples.len() > 1 && total - front.dt_secs >= window {
                total -= front.dt_secs;
                self.samples.pop_front();
            } else {
                break;
            }
        }
        self.samples
            .iter()
            .fold((0.0, 0.0), |(d, t), s| (d + s.distance, t + s.dt_secs))
    }
}
