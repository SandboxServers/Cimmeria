//! The watch sampler's ring buffer (lab autopilot contract C6). Pure: it
//! knows nothing about game memory, so host tests drive it with frame
//! numbers alone.

use std::collections::VecDeque;

use serde::Serialize;

/// Bounds the contract puts on `period_frames`.
pub const PERIOD_MIN: u64 = 1;
pub const PERIOD_MAX: u64 = 60;
/// Bounds the contract puts on `cap`.
pub const CAP_MIN: usize = 16;
pub const CAP_MAX: usize = 4096;

/// One frame's reading. Every field after `ms` is absent when its read
/// failed, so a client that cannot be read yields `{i, f, ms}`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sample {
    /// Sample index, assigned by [`Ring::push`]; never reused.
    pub i: u64,
    /// Bridge frame (the heartbeat counter) the sample was taken in.
    pub f: u64,
    /// Milliseconds since the bridge started.
    pub ms: u64,
    /// Game clock, seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<f64>,
    /// Pawn location, client space.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<[f32; 3]>,
    /// Pawn pitch, yaw, roll in rotator units. Roll is always 0: the
    /// existing pose reader decodes pitch and yaw only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rot: Option<[i32; 3]>,
    /// Hover entity id as read (`hover_id` reinterpreted as `u32`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover: Option<u32>,
    /// Controller mode bits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,
    /// Controller button bits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<u32>,
    /// Slot count of the persistent level's Actors array (null slots
    /// included; streamed sub-levels are not counted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actors: Option<u32>,
}

impl Sample {
    /// A sample with only the stamp fields; `i` is set on push.
    pub fn bare(f: u64, ms: u64) -> Self {
        Self {
            i: 0,
            f,
            ms,
            t: None,
            pos: None,
            rot: None,
            hover: None,
            mode: None,
            buttons: None,
            actors: None,
        }
    }
}

/// Fixed-capacity sample ring.
#[derive(Debug)]
pub struct Ring {
    buf: VecDeque<Sample>,
    cap: usize,
    next_i: u64,
    dropped: u64,
    period: u64,
    last_frame: Option<u64>,
    /// False after `watch_stop`; the samples stay readable until a new
    /// start replaces the ring.
    pub running: bool,
    /// Frame `watch_start` ran in.
    pub started_frame: u64,
}

impl Ring {
    /// Clamp `period` to 1..=60 and `cap` to 16..=4096.
    pub fn new(period: u64, cap: usize, started_frame: u64) -> Self {
        let cap = cap.clamp(CAP_MIN, CAP_MAX);
        Self {
            buf: VecDeque::with_capacity(cap),
            cap,
            next_i: 0,
            dropped: 0,
            period: period.clamp(PERIOD_MIN, PERIOD_MAX),
            last_frame: None,
            running: true,
            started_frame,
        }
    }

    pub fn period(&self) -> u64 {
        self.period
    }

    pub fn cap(&self) -> usize {
        self.cap
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Samples taken so far, including the ones the ring overwrote.
    pub fn taken(&self) -> u64 {
        self.next_i
    }

    /// Is a sample owed at `frame`? The first frame always is.
    pub fn due(&self, frame: u64) -> bool {
        self.last_frame
            .is_none_or(|last| frame.saturating_sub(last) >= self.period)
    }

    /// Store `s`, giving it the next index. A full ring drops its oldest
    /// sample and counts it.
    pub fn push(&mut self, mut s: Sample) {
        s.i = self.next_i;
        self.next_i += 1;
        self.last_frame = Some(s.f);
        if self.buf.len() == self.cap {
            self.buf.pop_front();
            self.dropped += 1;
        }
        self.buf.push_back(s);
    }

    /// Up to `max` samples with index >= `since`, and the index to pass as
    /// `since` next time. Moves no cursor, so two readers do not disturb
    /// each other.
    pub fn read(&self, since: u64, max: usize) -> (Vec<Sample>, u64) {
        let out: Vec<Sample> = self
            .buf
            .iter()
            .filter(|s| s.i >= since)
            .take(max)
            .cloned()
            .collect();
        let next = out.last().map_or(self.next_i, |s| s.i + 1);
        (out, next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill(r: &mut Ring, n: u64) {
        for f in 0..n {
            r.push(Sample::bare(f, f * 16));
        }
    }

    #[test]
    fn ring_overwrites_and_counts_dropped() {
        let mut r = Ring::new(1, 16, 0);
        fill(&mut r, 20);
        let (all, _) = r.read(0, 1024);
        assert_eq!(all.len(), 16);
        assert_eq!(r.dropped(), 4);
        assert_eq!(all[0].i, 4);
        assert_eq!(r.taken(), 20);
    }

    #[test]
    fn read_since_moves_no_cursor() {
        let mut r = Ring::new(1, 16, 0);
        fill(&mut r, 10);
        let (a, next_a) = r.read(0, 5);
        let (b, next_b) = r.read(0, 5);
        assert_eq!(a, b);
        assert_eq!(a.len(), 5);
        assert_eq!((next_a, next_b), (5, 5));
        let (rest, next) = r.read(next_a, 1024);
        assert_eq!(rest.len(), 5);
        assert_eq!(next, 10);
        // Nothing new: the cursor stays where the ring is.
        assert_eq!(r.read(10, 5), (Vec::new(), 10));
    }

    #[test]
    fn period_skips_frames() {
        let mut r = Ring::new(3, 16, 0);
        for f in 0..10 {
            if r.due(f) {
                r.push(Sample::bare(f, 0));
            }
        }
        let frames: Vec<u64> = r.read(0, 100).0.iter().map(|s| s.f).collect();
        assert_eq!(frames, vec![0, 3, 6, 9]);
    }

    #[test]
    fn ring_params_are_clamped() {
        assert_eq!(Ring::new(0, 100, 0).period(), 1);
        assert_eq!(Ring::new(99, 100, 0).period(), 60);
        assert_eq!(Ring::new(1, 1, 0).cap(), 16);
        assert_eq!(Ring::new(1, 99_999, 0).cap(), 4096);
    }
}
