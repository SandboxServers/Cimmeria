//! The launcher's forwarder: the in-memory queue and the at most one send each pump
//! may make. Consent is read again before every send (D-LTL2): a withdrawn target or
//! a new consent epoch drops the queue unsent. Nothing here touches the disk or logs.
use super::launcher_event::LauncherRecord;
use super::launcher_queue::{is_flush_trigger, LauncherQueue, FLUSH_TARGET};
use super::launcher_session::{LauncherSender, LauncherTarget, SendOutcome};
use super::transport::TelemetryEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Bumped by `set_game_telemetry` on every change of choice.
static CONSENT_EPOCH: AtomicU64 = AtomicU64::new(0);

/// The consent epoch now. Events built under an older epoch are withdrawn unsent.
pub fn consent_epoch() -> u64 {
    CONSENT_EPOCH.load(Ordering::SeqCst)
}

/// Starts a new consent epoch. Called once the new choice is stored.
pub(crate) fn consent_changed() {
    CONSENT_EPOCH.fetch_add(1, Ordering::SeqCst);
}

/// What the forwarder reads from the host. Each call takes the state lock only to read.
pub trait ForwardSource {
    /// `launcher_batch`'s events for `records`, or None (no module, or not opted in).
    fn events(&self, records: &[LauncherRecord]) -> Option<Vec<TelemetryEvent>>;
    /// `launcher_target()` now, or None.
    fn target(&self) -> Option<LauncherTarget>;
    /// `consent_epoch()` now.
    fn epoch(&self) -> u64;
}

/// What one pump that sent, or tried to send, did. The shell logs it through health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pumped {
    /// How the send ended.
    pub outcome: SendOutcome,
    /// Events the server accepted.
    pub sent_events: u64,
    /// Events given up on by this pump, including those dropped at the cap.
    pub lost_events: u64,
}

/// The queue, the sender and the exit-flush state of one launcher session.
pub struct Forwarder {
    queue: LauncherQueue,
    sender: LauncherSender,
    /// A flush sentinel arrived and no finished send has ended the flush yet.
    flush_pending: bool,
    /// Set when a flush ends; `take_flush_done` reads it once.
    flush_done: bool,
}

impl Forwarder {
    /// A forwarder with an empty queue that sends through `sender`.
    pub fn new(sender: LauncherSender) -> Self {
        Self {
            queue: LauncherQueue::new(),
            sender,
            flush_pending: false,
            flush_done: false,
        }
    }

    /// Queues the events of `records`. The epoch is read first, before anything else
    /// (D1): a toggle during `events` must not let events built while opted in carry
    /// the newer epoch. A flush sentinel only sets the pending flush and is not built.
    /// A flush trigger urges a send only when the queue holds events (D4).
    pub fn on_records<S: ForwardSource>(
        &mut self,
        source: &S,
        records: Vec<LauncherRecord>,
        now: Instant,
    ) {
        let epoch = source.epoch();
        let mut flush_seen = false;
        let mut built = Vec::with_capacity(records.len());
        for record in records {
            flush_seen |= is_flush_trigger(&record);
            if record.target == FLUSH_TARGET {
                self.flush_pending = true;
            } else {
                built.push(record);
            }
        }
        if !built.is_empty() {
            if let Some(events) = source.events(&built) {
                self.queue.push(events, epoch, now);
            }
        }
        if flush_seen && !self.queue.is_empty() {
            self.queue.mark_urgent();
        }
    }

    /// When the next `pump` may send: `Some(ZERO)` while a flush waits on queued events,
    /// otherwise the queue's own wake time under the sender's hold.
    pub fn next_wake(&self, now: Instant) -> Option<Duration> {
        if self.flush_pending && !self.queue.is_empty() {
            return Some(Duration::ZERO);
        }
        self.queue.next_wake(now, self.sender.hold_remaining())
    }

    /// At most one send. Consent comes first, so an opt-out is honoured even under a
    /// hold. Returns None when nothing was sent.
    pub async fn pump<S: ForwardSource>(&mut self, source: &S, now: Instant) -> Option<Pumped> {
        if self.flush_pending && !self.queue.is_empty() {
            self.queue.mark_urgent();
        }
        // The target is read before the epoch; either one withdraws the queue.
        let target = source.target();
        let epoch = source.epoch();
        let stale = !self.queue.is_empty() && self.queue.epoch() != Some(epoch);
        let target = match target {
            Some(target) if !stale => target,
            _ => {
                self.queue.withdraw();
                self.finish_flush();
                return None;
            }
        };
        let hold = self.sender.hold_remaining();
        if !self.queue.due(now, hold) {
            self.finish_flush();
            return None;
        }
        let batch = self.queue.take(now);
        let report = self.sender.send_report(&target, &batch).await;
        let retry = report
            .retry_from
            .map_or_else(Vec::new, |i| batch[i..].to_vec());
        let retried = retry.len();
        self.queue.give_back(retry, now);
        let lost = (batch.len() - report.posted - retried) as u64 + self.queue.take_lost();
        if self.queue.is_empty() || report.outcome != SendOutcome::Sent {
            self.finish_flush();
        }
        Some(Pumped {
            outcome: report.outcome,
            sent_events: report.posted as u64,
            lost_events: lost,
        })
    }

    /// True once since the last call when a flush finished (sent, failed, held or
    /// nothing to send).
    pub fn take_flush_done(&mut self) -> bool {
        std::mem::take(&mut self.flush_done)
    }

    /// Ends a pending flush and marks it done. Does nothing when no flush is pending.
    fn finish_flush(&mut self) {
        if self.flush_pending {
            self.flush_pending = false;
            self.flush_done = true;
        }
    }

    /// The sender this forwarder posts through.
    #[cfg(test)]
    pub(crate) fn sender(&self) -> &LauncherSender {
        &self.sender
    }

    /// Number of events waiting in the queue.
    #[cfg(test)]
    pub(crate) fn queued(&self) -> usize {
        self.queue.len()
    }
}
