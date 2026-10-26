//! The launcher's in-memory hold queue (D-LTL1, D-LTL2). Forwarded events wait here
//! while the sender coalesces them, keeps its spacing or obeys the server's hold.
//! This is pure state: the caller passes every time in, and nothing here reads the
//! clock, touches the disk or logs. Nothing in the queue is ever written to disk.

use super::launcher_event::LauncherRecord;
use super::launcher_session::LAUNCHER_MAX_ROWS;
use super::transport::TelemetryEvent;
use serde_json::Value;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Most events held at once. The oldest are dropped first and counted as lost.
pub const QUEUE_CAP: usize = 500;
/// How long the oldest queued event waits for company before it is sent.
pub const COALESCE: Duration = Duration::from_secs(2);
/// The least time between two sends, which keeps the server's per-minute chunk quota.
pub const MIN_SPACING: Duration = Duration::from_secs(7);
/// A group this size sends without waiting for `COALESCE`.
pub const FILL: usize = 64;
/// Most events one `take` returns: one chunk.
pub const SEND_MAX: usize = LAUNCHER_MAX_ROWS;
/// The `LauncherRecord.target` of the exit flush sentinel.
pub const FLUSH_TARGET: &str = "launcher.flush";

/// True for the exit sentinel and for the `launch_finished` line of the launch flow.
/// Those records skip the coalescing wait.
pub fn is_flush_trigger(record: &LauncherRecord) -> bool {
    record.target == FLUSH_TARGET
        || (record.target == "client.launcher.launch"
            && record.fields.get("event").and_then(Value::as_str) == Some("launch_finished"))
}

/// Events waiting to be posted, with the timers that decide when they go.
#[derive(Debug, Default)]
pub struct LauncherQueue {
    events: VecDeque<TelemetryEvent>,
    /// When the oldest queued event arrived. Set exactly while the queue is not empty.
    oldest: Option<Instant>,
    /// When the last `take` ran.
    last_send: Option<Instant>,
    /// Set by `mark_urgent`: the next pump sends without waiting.
    urgent: bool,
    /// Events dropped at the cap since the last `take_lost`.
    lost: u64,
    /// The consent epoch the queued events were built under.
    epoch: Option<u64>,
}

impl LauncherQueue {
    /// An empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `events`, built under consent `epoch`. A different epoch clears the queue
    /// first. Those events were never sent, so they are not counted as lost.
    pub fn push(&mut self, events: Vec<TelemetryEvent>, epoch: u64, now: Instant) {
        if !self.events.is_empty() && self.epoch != Some(epoch) {
            self.clear();
        }
        self.epoch = Some(epoch);
        if !events.is_empty() {
            self.oldest.get_or_insert(now);
        }
        self.events.extend(events);
        self.trim();
    }

    /// Makes the queue due at once, skipping the coalescing wait and the spacing.
    pub fn mark_urgent(&mut self) {
        self.urgent = true;
    }

    /// True when a send may run now: the queue is not empty, no hold runs, and either
    /// the queue is urgent or both the spacing and the coalescing point have passed.
    pub fn due(&self, now: Instant, hold_left: Option<Duration>) -> bool {
        !self.events.is_empty()
            && hold_left.is_none()
            && (self.urgent || now >= self.spacing_at(now).max(self.ready_at(now)))
    }

    /// When the next `due` check can turn true. None when the queue is empty, the hold
    /// time while a hold runs, and zero when urgent.
    pub fn next_wake(&self, now: Instant, hold_left: Option<Duration>) -> Option<Duration> {
        if self.events.is_empty() {
            return None;
        }
        if let Some(hold) = hold_left {
            return Some(hold);
        }
        if self.urgent {
            return Some(Duration::ZERO);
        }
        let at = self.spacing_at(now).max(self.ready_at(now));
        Some(at.saturating_duration_since(now))
    }

    /// Drains up to `SEND_MAX` events from the front and records `now` as the last send.
    /// When that empties the queue, the urgent flag and the oldest-arrival time reset.
    pub fn take(&mut self, now: Instant) -> Vec<TelemetryEvent> {
        let n = self.events.len().min(SEND_MAX);
        let taken: Vec<TelemetryEvent> = self.events.drain(..n).collect();
        self.last_send = Some(now);
        if self.events.is_empty() {
            self.urgent = false;
            self.oldest = None;
        }
        taken
    }

    /// Puts `unsent` back at the front, in order, ahead of newer events, then drops the
    /// oldest past the cap. An empty `unsent` changes nothing.
    pub fn give_back(&mut self, unsent: Vec<TelemetryEvent>, now: Instant) {
        if unsent.is_empty() {
            return;
        }
        let mut newer = std::mem::take(&mut self.events);
        self.events = unsent.into();
        self.events.append(&mut newer);
        self.oldest.get_or_insert(now);
        self.trim();
    }

    /// Clears the queue, its urgent flag and its epoch, and returns how many events it
    /// cleared. Those events are not counted as lost. The send spacing and the lost
    /// count are kept, since neither depends on the events that were cleared.
    pub fn withdraw(&mut self) -> usize {
        let cleared = self.clear();
        self.epoch = None;
        cleared
    }

    /// Returns the events dropped at the cap since the last call, and resets the count.
    pub fn take_lost(&mut self) -> u64 {
        std::mem::take(&mut self.lost)
    }

    /// The consent epoch of the queued events, or None when nothing was pushed since the
    /// last withdrawal.
    pub fn epoch(&self) -> Option<u64> {
        self.epoch
    }

    /// Number of queued events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// True when no event is queued.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Empties the events, the oldest-arrival time and the urgent flag.
    fn clear(&mut self) -> usize {
        let cleared = self.events.len();
        self.events.clear();
        self.oldest = None;
        self.urgent = false;
        cleared
    }

    /// Drops the oldest events past the cap and counts them as lost.
    fn trim(&mut self) {
        let excess = self.events.len().saturating_sub(QUEUE_CAP);
        self.events.drain(..excess);
        self.lost += excess as u64;
    }

    /// The earliest moment the next send may run: `MIN_SPACING` after the last send.
    fn spacing_at(&self, now: Instant) -> Instant {
        self.last_send.map_or(now, |t| t + MIN_SPACING)
    }

    /// The moment the queued events are ready to go: now for a full group, otherwise
    /// the oldest arrival plus `COALESCE`.
    fn ready_at(&self, now: Instant) -> Instant {
        match self.oldest {
            Some(t) if self.events.len() < FILL => t + COALESCE,
            _ => now,
        }
    }
}
