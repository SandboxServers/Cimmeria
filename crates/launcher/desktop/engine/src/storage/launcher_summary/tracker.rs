//! In-memory attempt tracking and the journal observer. The observer sees only
//! confirmed commits and does cheap, infallible work; reading result detail
//! needs the whole state and happens later, in `DesktopState::finalize_summaries`.
//! What a row may say about time is in `attempt.rs`.
//!
//! One known limit of the restart path. `reconcile` reports the journal's
//! terminal state as the outcome. If a process that never configured summaries
//! (an older launcher, a tool) reconciled the operation to a terminal in
//! between, that terminal is reported as observed although no worker saw it.
use super::{
    attempt::{Live, Pending},
    endpoint::SummaryEndpoint,
    queue::{Entry, Queue, Tracking},
    schema::{
        LauncherVersion, RetryCount, Summary, SummaryArch, SummaryErrorCode, SummaryOperation,
        SummaryOs, SummaryOutcome, SummaryPhase, TimedPhase,
    },
    SummaryConfig, SummaryFaults,
};
use crate::{Operation, OperationState, Snapshot};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Held by both `FileJournal` and `DesktopState`. Lock order: the state mutex
/// outside, this one inside. Poisoning is ignored; the data is disposable.
pub(in crate::storage) type Shared = Arc<Mutex<Tracker>>;

pub(super) fn lock(shared: &Shared) -> MutexGuard<'_, Tracker> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Wall clock, monotonic clock and id minting, injected so tests are exact.
pub(super) struct Sources {
    pub unix_s: Box<dyn Fn() -> u64 + Send>,
    pub monotonic: Box<dyn Fn() -> Duration + Send>,
    pub new_id: Box<dyn FnMut() -> Uuid + Send>,
}
impl Default for Sources {
    fn default() -> Self {
        let origin = Instant::now();
        Self {
            unix_s: Box::new(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |elapsed| elapsed.as_secs())
            }),
            monotonic: Box::new(move || origin.elapsed()),
            new_id: Box::new(Uuid::new_v4),
        }
    }
}

pub(super) struct Active {
    pub version: LauncherVersion,
    pub endpoint: SummaryEndpoint,
}

/// Why nothing may be tracked, recorded or sent, in the order it is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Gate {
    Open,
    NoEndpoint,
    NoConsent,
    Blocked,
    ReopenRequired,
}

pub(in crate::storage) struct Tracker {
    pub(super) root: PathBuf,
    /// `Some` only after `configure_summaries` ran with an endpoint.
    pub(super) active: Option<Active>,
    // Copies of state the observer cannot reach; refreshed by every entry point.
    pub(super) consent: bool,
    pub(super) reopen: bool,
    /// Set by an opt-out before its write; only a successful opt-in clears it.
    pub(super) export_blocked: bool,
    loaded: bool,
    pub(super) queue: Queue,
    live: Option<Live>,
    pending: Option<Pending>,
    /// The last queue write failed, so the file is behind the queue in memory.
    dirty: bool,
    pub(super) sources: Sources,
    pub(super) faults: SummaryFaults,
    /// Cancelled when consent is withdrawn, and then replaced. Every batch
    /// carries a clone: cancelling aborts the exporter's mint or POST in flight
    /// and ends its backoff wait.
    pub(super) cancel: CancellationToken,
    /// Wakes the exporter when a row is waiting.
    pub(super) wake: Arc<Notify>,
    /// Set once an exporter task was started for this state; there is only one.
    pub(super) exporting: bool,
    /// Cancelled when the state is dropped, which ends the exporter task.
    pub(super) shutdown: CancellationToken,
    #[cfg(test)]
    pub(super) panic_hook: bool,
    /// Fails every queue write while set, leaving the file as it is.
    #[cfg(test)]
    pub(super) store_fault: bool,
}
impl Drop for Tracker {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

// Eligibility: only the admission commit starts tracking. An attempt first seen
// in any later state (admitted while the gate was closed, before configuration,
// or by a previous process) stays untracked for its whole life.
fn is_admission(state: OperationState) -> bool {
    state == OperationState::Starting
}

impl Tracker {
    pub(super) fn new(root: &Path, consent: bool) -> Shared {
        Arc::new(Mutex::new(Self {
            root: root.to_path_buf(),
            active: None,
            consent,
            reopen: false,
            export_blocked: false,
            loaded: false,
            queue: Queue::empty(0),
            live: None,
            pending: None,
            dirty: false,
            sources: Sources::default(),
            faults: SummaryFaults::default(),
            cancel: CancellationToken::new(),
            wake: Arc::new(Notify::new()),
            exporting: false,
            shutdown: CancellationToken::new(),
            #[cfg(test)]
            panic_hook: false,
            #[cfg(test)]
            store_fault: false,
        }))
    }

    /// The one gate. Every place that tracks, records or sends asks this.
    pub(super) fn gate(&self) -> Gate {
        if self.active.is_none() {
            Gate::NoEndpoint
        } else if !self.consent {
            Gate::NoConsent
        } else if self.export_blocked {
            Gate::Blocked
        } else if self.reopen {
            Gate::ReopenRequired
        } else {
            Gate::Open
        }
    }

    /// A failed queue write is counted and otherwise ignored.
    pub(super) fn store(&mut self) {
        self.store_checked();
    }

    pub(super) fn store_checked(&mut self) -> bool {
        #[cfg(test)]
        let stored = !self.store_fault && self.queue.store(&self.root).is_ok();
        #[cfg(not(test))]
        let stored = self.queue.store(&self.root).is_ok();
        if !stored {
            self.faults.queue_writes = self.faults.queue_writes.saturating_add(1);
        }
        self.dirty = !stored;
        stored
    }

    /// Tries a failed queue write again. Until it succeeds the file may still
    /// hold tracking for an attempt whose row exists only in memory, and a
    /// restart would then report that attempt a second time.
    pub(super) fn retry_store(&mut self) {
        if self.dirty && self.active.is_some() {
            self.store();
        }
    }

    pub(super) fn test_panic(&self) {
        #[cfg(test)]
        if self.panic_hook {
            panic!("injected launcher-summary fault");
        }
    }

    /// Called after `operation.json` was replaced successfully.
    pub(super) fn observe(&mut self, snapshot: &Snapshot) {
        self.test_panic();
        let Some(operation) = snapshot.operation.as_ref() else {
            return;
        };
        if self.active.is_none() {
            return;
        }
        let now = (self.sources.monotonic)();
        if self.tracks(operation.id) {
            match operation.state {
                OperationState::Starting | OperationState::CancelRequested => (),
                OperationState::Running => self.enter_phase(operation.id, TimedPhase::Running),
                OperationState::Succeeded => self.finish(SummaryOutcome::Succeeded, now),
                OperationState::Failed => self.finish(SummaryOutcome::Failed, now),
                OperationState::Cancelled => self.finish(SummaryOutcome::Cancelled, now),
                // Lost observation is its own outcome, never a success.
                OperationState::ReconciliationRequired => self.finish(SummaryOutcome::Unknown, now),
            }
            return;
        }
        // A finished or lost operation has nothing left to track.
        if operation.state.terminal() || operation.state == OperationState::ReconciliationRequired {
            return;
        }
        if !is_admission(operation.state) || self.gate() != Gate::Open {
            return;
        }
        let attempt_id = (self.sources.new_id)();
        self.live = Some(Live::admitted(
            operation.id,
            attempt_id,
            operation.kind.into(),
            now,
        ));
        // Persisted now so that a crash is later reported as `unknown`.
        self.queue.tracking = Some(Tracking {
            local_operation_id: operation.id,
            attempt_id,
            kind: operation.kind.into(),
        });
        self.store();
    }

    fn finish(&mut self, outcome: SummaryOutcome, now: Duration) {
        let Some(live) = self.live.take() else {
            return;
        };
        // An earlier end nobody finalized can no longer have its detail read.
        if let Some(earlier) = self.pending.take() {
            let code = (earlier.outcome == SummaryOutcome::Failed)
                .then_some(SummaryErrorCode::Unspecified);
            self.enqueue_terminal(earlier, code);
        }
        self.pending = Some(live.finish(outcome, now));
        self.wake.notify_one();
    }

    pub(super) fn take_pending(&mut self) -> Option<Pending> {
        self.pending.take()
    }

    /// One queue entry and the end of tracking, in the same queue write.
    pub(super) fn enqueue_terminal(&mut self, pending: Pending, code: Option<SummaryErrorCode>) {
        // A state that must be reopened is left alone. Tracking stays on disk,
        // so the next process reports this attempt, without timings.
        if self.gate() == Gate::ReopenRequired {
            return;
        }
        let tracked = self
            .queue
            .tracking
            .is_some_and(|tracking| tracking.local_operation_id == pending.local_id);
        if tracked {
            self.queue.tracking = None;
        }
        let version = self
            .active
            .as_ref()
            .filter(|_| self.gate() == Gate::Open)
            .map(|active| active.version);
        if let Some(launcher_version) = version {
            let entry = Entry {
                created_unix_s: (self.sources.unix_s)(),
                pre_admission: false,
                summary: Summary {
                    event_id: (self.sources.new_id)(),
                    attempt_id: pending.attempt_id,
                    operation: pending.operation,
                    phase: pending.phase,
                    outcome: pending.outcome,
                    error_code: code.filter(|_| pending.outcome == SummaryOutcome::Failed),
                    duration_ms: pending.duration_ms,
                    retry_count: RetryCount::ZERO,
                    phases: pending.phases,
                    launcher_version,
                    os: SummaryOs::current(),
                    arch: SummaryArch::current(),
                },
            };
            self.queue.push_admitted(entry);
        } else if !tracked {
            return;
        }
        self.store();
    }

    pub(super) fn enter_phase(&mut self, operation_id: Uuid, phase: TimedPhase) {
        let now = (self.sources.monotonic)();
        if let Some(live) = self
            .live
            .as_mut()
            .filter(|live| live.local_id == operation_id)
        {
            live.enter(phase, now);
        }
    }

    pub(super) fn tracks(&self, operation_id: Uuid) -> bool {
        self.live
            .as_ref()
            .is_some_and(|live| live.local_id == operation_id)
    }

    #[cfg(test)]
    pub(super) fn open_phase(&self) -> Option<TimedPhase> {
        self.live.as_ref().map(|live| live.open)
    }

    pub(super) fn forget_attempt(&mut self) {
        self.live = None;
        self.pending = None;
    }

    /// Returns the end of an attempt a previous process left tracked, when the
    /// journal shows it; the caller reads its detail and enqueues it.
    pub(super) fn configure(
        &mut self,
        config: SummaryConfig,
        current: Option<&Operation>,
    ) -> Option<Pending> {
        let Some(endpoint) = config.endpoint else {
            // Inert: nothing is tracked, recorded or sent, and nothing waits on disk.
            self.active = None;
            self.forget_attempt();
            Queue::remove(&self.root);
            self.queue = Queue::empty(self.queue.generation.saturating_add(1));
            self.loaded = false;
            self.dirty = false;
            return None;
        };
        self.active = Some(Active {
            version: LauncherVersion::new(config.launcher_version),
            endpoint,
        });
        if !self.loaded {
            // The generation never goes back within one process run.
            let floor = self.queue.generation;
            self.queue = Queue::load(&self.root);
            self.queue.generation = self.queue.generation.max(floor);
            self.loaded = true;
        }
        if !self.consent {
            // Startup scrub: without consent nothing waits on disk either.
            Queue::remove(&self.root);
            self.queue = Queue::empty(self.queue.generation);
            self.forget_attempt();
            self.dirty = false;
            return None;
        }
        let expired = self.queue.expire((self.sources.unix_s)());
        let (pending, forgotten) = self.reconcile(current);
        if expired || forgotten {
            self.store();
        }
        pending
    }

    // Never invent a timing or a phase for an attempt this process did not watch.
    fn reconcile(&mut self, current: Option<&Operation>) -> (Option<Pending>, bool) {
        let Some(tracking) = self.queue.tracking else {
            return (None, false);
        };
        if self.tracks(tracking.local_operation_id) {
            return (None, false);
        }
        let operation = current.filter(|operation| operation.id == tracking.local_operation_id);
        let (Gate::Open, Some(operation)) = (self.gate(), operation) else {
            self.queue.tracking = None;
            return (None, true);
        };
        let outcome = match operation.state {
            OperationState::Succeeded => SummaryOutcome::Succeeded,
            OperationState::Failed => SummaryOutcome::Failed,
            OperationState::Cancelled => SummaryOutcome::Cancelled,
            _ => SummaryOutcome::Unknown,
        };
        (Some(Pending::unobserved(tracking, outcome)), false)
    }

    /// `current` is the journal's operation id; that attempt and the tracked one
    /// report through the journal, so a failure naming either is not counted twice.
    pub(super) fn pre_admission_failure(
        &mut self,
        request_operation_id: Option<Uuid>,
        current: Option<Uuid>,
        operation: SummaryOperation,
        phase: SummaryPhase,
        code: SummaryErrorCode,
    ) {
        if self.gate() != Gate::Open {
            return;
        }
        if request_operation_id.is_some_and(|id| Some(id) == current || self.tracks(id)) {
            return;
        }
        let Some(launcher_version) = self.active.as_ref().map(|active| active.version) else {
            return;
        };
        let now = (self.sources.unix_s)();
        let expired = self.queue.expire(now);
        match self.queue.repeat_pre_admission(operation, phase, code) {
            Some(false) if !expired => return,
            Some(_) => (),
            None => {
                self.queue.push_pre_admission(Entry {
                    created_unix_s: now,
                    pre_admission: true,
                    summary: Summary {
                        event_id: (self.sources.new_id)(),
                        attempt_id: (self.sources.new_id)(),
                        operation,
                        phase,
                        outcome: SummaryOutcome::Failed,
                        error_code: Some(code),
                        duration_ms: None,
                        retry_count: RetryCount::ZERO,
                        phases: None,
                        launcher_version,
                        os: SummaryOs::current(),
                        arch: SummaryArch::current(),
                    },
                });
                self.wake.notify_one();
            }
        }
        self.store();
    }
}
