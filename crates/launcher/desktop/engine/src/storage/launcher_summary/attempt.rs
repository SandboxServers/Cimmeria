//! One tracked attempt: its timed phases while this process watches it, and
//! its committed end while that waits for the error detail to be read.
//!
//! What a row may say about time is decided here, in `Live::finish`:
//!
//! - Entering a phase closes the one before it. A phase entered again adds to
//!   the same entry, so `download` and `extraction` are totals over every blob.
//! - A lost observation (`unknown`) has no total duration and no entry for the
//!   phase that was open: nobody saw either of them end. Phases that had
//!   already closed are kept.
//! - A launch has no total duration and no `running` entry. The journal commits
//!   `Running` before the game host is spawned and the terminal when the game
//!   process exits, so both would be the length of the play session. That is
//!   game activity, which launcher-summary consent does not cover. `starting`,
//!   the launcher's own preparation, is kept, and the row's `phase` still says
//!   where the attempt ended.
use super::{
    queue::Tracking,
    schema::{Millis, PhaseDuration, SummaryOperation, SummaryOutcome, SummaryPhase, TimedPhase},
};
use std::time::Duration;
use uuid::Uuid;

/// An attempt admitted by this process with the gate open.
pub(super) struct Live {
    pub local_id: Uuid,
    pub attempt_id: Uuid,
    pub operation: SummaryOperation,
    started: Duration,
    pub open: TimedPhase,
    opened: Duration,
    phases: Vec<(TimedPhase, Duration)>,
}
impl Live {
    /// Admission opens `starting`.
    pub fn admitted(
        local_id: Uuid,
        attempt_id: Uuid,
        operation: SummaryOperation,
        now: Duration,
    ) -> Self {
        Self {
            local_id,
            attempt_id,
            operation,
            started: now,
            open: TimedPhase::Starting,
            opened: now,
            phases: vec![(TimedPhase::Starting, Duration::ZERO)],
        }
    }

    pub fn enter(&mut self, phase: TimedPhase, now: Duration) {
        if phase == self.open {
            return;
        }
        self.close(now);
        self.open = phase;
        if !self.phases.iter().any(|(seen, _)| *seen == phase) {
            self.phases.push((phase, Duration::ZERO));
        }
    }

    fn close(&mut self, now: Duration) {
        let spent = now.saturating_sub(self.opened);
        if let Some((_, total)) = self.phases.iter_mut().find(|(seen, _)| *seen == self.open) {
            *total = total.saturating_add(spent);
        }
        self.opened = now;
    }

    pub fn finish(mut self, outcome: SummaryOutcome, now: Duration) -> Pending {
        let lost = outcome == SummaryOutcome::Unknown;
        let launch = self.operation == SummaryOperation::Launch;
        let open = self.open;
        if lost {
            // The open phase never ended where this process could see it.
            self.phases.retain(|(phase, _)| *phase != open);
        } else {
            self.close(now);
        }
        if launch {
            // The game session is not the launcher's to report.
            self.phases
                .retain(|(phase, _)| *phase != TimedPhase::Running);
        }
        let phases: Vec<PhaseDuration> = self
            .phases
            .into_iter()
            .map(|(phase, total)| PhaseDuration {
                phase,
                duration_ms: Millis::saturating(total),
            })
            .collect();
        Pending {
            local_id: self.local_id,
            attempt_id: self.attempt_id,
            operation: self.operation,
            outcome,
            phase: open.into(),
            duration_ms: (!lost && !launch)
                .then(|| Millis::saturating(now.saturating_sub(self.started))),
            phases: (!phases.is_empty()).then_some(phases),
        }
    }
}

/// A committed end of an attempt, waiting for its error detail to be read.
pub(super) struct Pending {
    pub local_id: Uuid,
    pub attempt_id: Uuid,
    pub operation: SummaryOperation,
    pub outcome: SummaryOutcome,
    pub phase: SummaryPhase,
    pub duration_ms: Option<Millis>,
    pub phases: Option<Vec<PhaseDuration>>,
}
impl Pending {
    /// The end of an attempt a previous process admitted. This process timed
    /// nothing of it, so the row carries no duration, no phases and no end phase.
    pub fn unobserved(tracking: Tracking, outcome: SummaryOutcome) -> Self {
        Self {
            local_id: tracking.local_operation_id,
            attempt_id: tracking.attempt_id,
            operation: tracking.kind,
            outcome,
            phase: SummaryPhase::None,
            duration_ms: None,
            phases: None,
        }
    }
}
