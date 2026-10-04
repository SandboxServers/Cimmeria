//! The exporter's seam. It takes a batch under the state lock, sends it with the
//! lock released, and comes back to apply the verdicts. `generation` and the gate
//! are checked again on every return, so a withdrawal in between applies nothing.
//!
//! Two counts are at-least-once approximations, not exact totals, by design in
//! v1. A batch holds copies of what was queued when it was taken:
//!
//! - `client_dropped` is cleared only by an answered `200`. A body the server
//!   took but whose answer was lost is sent again with the same counters, and
//!   the server counts them twice (summaries are deduplicated by `event_id`;
//!   counters are not). A row evicted by overflow while it is in flight is also
//!   counted as dropped although the server received it.
//! - `retry_count` of a pre-admission row is at least what was delivered. A
//!   repeat arriving while the row is in flight raises the queued copy, which
//!   the verdict then removes, so that repeat is never reported.
use super::{
    endpoint::SummaryEndpoint,
    schema::{
        DroppedCounts, LauncherVersion, Summary, SummaryRequest, SummaryResult, SCHEMA_VERSION,
    },
    tracker::{lock, Gate, Tracker},
};
use crate::storage::DesktopState;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// What one upload carries. The entries stay queued until a verdict removes them.
pub(super) struct Batch {
    pub generation: u64,
    /// Cancelled when consent is withdrawn. That aborts the exporter's mint or
    /// POST in flight and ends its backoff wait. Whether anything may still be
    /// sent or applied is decided under the lock, by the gate and `generation`.
    pub cancel: CancellationToken,
    /// The drop counters as sent; exactly these are subtracted on success.
    pub dropped: DroppedCounts,
    pub summaries: Vec<Summary>,
}
impl Batch {
    pub fn request(&self) -> SummaryRequest {
        SummaryRequest {
            schema_version: SCHEMA_VERSION,
            client_dropped: self.dropped,
            summaries: self.summaries.clone(),
        }
    }
    fn event_ids(&self) -> Vec<Uuid> {
        self.summaries
            .iter()
            .map(|summary| summary.event_id)
            .collect()
    }
}

pub(super) enum Take {
    Closed(Gate),
    Empty,
    Ready(Batch),
}

/// Fixed for the life of one configuration.
pub(super) struct ExportTarget {
    pub endpoint: SummaryEndpoint,
    pub launcher_version: LauncherVersion,
    /// Notified when a row is waiting.
    pub wake: Arc<Notify>,
    /// Cancelled when the state is dropped.
    pub shutdown: CancellationToken,
}

impl DesktopState {
    /// `None` while no endpoint is configured: there is nothing to start.
    pub(super) fn summary_export_target(&self) -> Option<ExportTarget> {
        let tracker = lock(&self.summaries);
        tracker.active.as_ref().map(|active| ExportTarget {
            endpoint: active.endpoint.clone(),
            launcher_version: active.version,
            wake: tracker.wake.clone(),
            shutdown: tracker.shutdown.clone(),
        })
    }

    /// Finalizes pending rows, expires old ones, then hands out the oldest.
    pub(super) fn summary_take_batch(&mut self) -> Take {
        self.summary_guarded(|state| match state.summary_sync() {
            Gate::Open => lock(&state.summaries).take_batch(),
            closed => Take::Closed(closed),
        })
        .unwrap_or(Take::Empty)
    }

    /// Whether a batch taken at `generation` may still be sent or applied.
    pub(super) fn summary_batch_current(&mut self, generation: u64) -> bool {
        self.summary_guarded(|state| {
            state.summary_sync() == Gate::Open
                && lock(&state.summaries).queue.generation == generation
        })
        .unwrap_or(false)
    }

    /// A `200` with one verdict per summary: every verdict removes its entry.
    /// Returns false, changing nothing, when the batch is no longer current.
    pub(super) fn summary_apply_results(
        &mut self,
        batch: &Batch,
        results: &[SummaryResult],
    ) -> bool {
        if results.len() != batch.summaries.len() || !self.summary_batch_current(batch.generation) {
            return false;
        }
        let rejected = results
            .iter()
            .filter(|result| **result == SummaryResult::Rejected)
            .count();
        self.summary_guarded(|state| {
            let mut tracker = lock(&state.summaries);
            let dropped = &mut tracker.queue.dropped;
            // Only what the server received is cleared; later drops stay counted.
            dropped.overflow = dropped.overflow.saturating_sub(batch.dropped.overflow);
            dropped.expired = dropped.expired.saturating_sub(batch.dropped.expired);
            dropped.rejected = dropped.rejected.saturating_sub(batch.dropped.rejected);
            tracker.remove_batch(batch, rejected);
        })
        .is_some()
    }

    /// A permanent refusal of the whole body (`400`, `413`): the batch is dropped
    /// and counted, and the counters it carried are sent again.
    pub(super) fn summary_reject_batch(&mut self, batch: &Batch) -> bool {
        if !self.summary_batch_current(batch.generation) {
            return false;
        }
        self.summary_guarded(|state| {
            lock(&state.summaries).remove_batch(batch, batch.summaries.len());
        })
        .is_some()
    }
}

impl Tracker {
    fn take_batch(&mut self) -> Take {
        if self.queue.expire((self.sources.unix_s)()) {
            self.store();
        }
        let summaries = self.queue.oldest_batch();
        if summaries.is_empty() {
            return Take::Empty;
        }
        Take::Ready(Batch {
            generation: self.queue.generation,
            cancel: self.cancel.clone(),
            dropped: self.queue.dropped,
            summaries,
        })
    }

    fn remove_batch(&mut self, batch: &Batch, rejected: usize) {
        self.queue.remove_events(&batch.event_ids());
        self.queue.dropped.rejected = self
            .queue
            .dropped
            .rejected
            .saturating_add(u16::try_from(rejected).unwrap_or(u16::MAX));
        self.store();
    }
}
