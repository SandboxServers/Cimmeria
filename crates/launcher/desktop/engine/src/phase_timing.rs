//! Bounded, path-free timing of native adoption and pre-game launch work.
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use uuid::Uuid;

static NEXT_PHASE: AtomicU64 = AtomicU64::new(1);

/// Closed stages; never use a file name, URL or renderer-provided label here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    AdoptionPreview,
    AdoptionInitialScan,
    AdoptionCleanupScan,
    AdoptionComparison,
    AdoptionIndex,
    AdoptionReplay,
    AdoptionReviewScan,
    AdoptionConfirmation,
    AdoptionConfirmationScan,
    AdoptionApply,
    AdoptionPublicationLock,
    AdoptionSetup,
    AdoptionFinalScan,
    PlayRequest,
    PlayDispatchQueue,
    PlayAdmission,
    PlayResources,
    PlayTelemetryMint,
    PlayClientSetup,
    PlayPreviousSession,
    PlayHelperStart,
    PlayHelperSpawn,
    PlayHelperHandshake,
}

impl Phase {
    fn code(self) -> &'static str {
        match self {
            Self::AdoptionPreview => "adoption_preview",
            Self::AdoptionInitialScan => "adoption_initial_scan",
            Self::AdoptionCleanupScan => "adoption_cleanup_scan",
            Self::AdoptionComparison => "adoption_comparison",
            Self::AdoptionIndex => "adoption_index",
            Self::AdoptionReplay => "adoption_replay",
            Self::AdoptionReviewScan => "adoption_review_scan",
            Self::AdoptionConfirmation => "adoption_confirmation",
            Self::AdoptionConfirmationScan => "adoption_confirmation_scan",
            Self::AdoptionApply => "adoption_apply",
            Self::AdoptionPublicationLock => "adoption_publication_lock",
            Self::AdoptionSetup => "adoption_setup",
            Self::AdoptionFinalScan => "adoption_final_scan",
            Self::PlayRequest => "play_request",
            Self::PlayDispatchQueue => "play_dispatch_queue",
            Self::PlayAdmission => "play_admission",
            Self::PlayResources => "play_resources",
            Self::PlayTelemetryMint => "play_telemetry_mint",
            Self::PlayClientSetup => "play_client_setup",
            Self::PlayPreviousSession => "play_previous_session",
            Self::PlayHelperStart => "play_helper_start",
            Self::PlayHelperSpawn => "play_helper_spawn",
            Self::PlayHelperHandshake => "play_helper_handshake",
        }
    }

    fn adoption(self) -> bool {
        self.code().starts_with("adoption_")
    }
}

/// Completion describes only this stage, never game login or world entry.
#[derive(Clone, Copy, Debug)]
pub enum Outcome {
    Succeeded,
    Failed,
    Unavailable,
    Cancelled,
}

impl Outcome {
    fn code(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Process-local correlation only. The durable UUID never enters a record.
#[derive(Clone, Copy)]
pub struct Context {
    operation: u64,
}

impl Context {
    /// Preview work has no admitted operation yet.
    pub fn preview() -> Self {
        Self::for_operation(Uuid::new_v4())
    }

    /// Resolve the existing operation to process-local numeric correlation;
    /// the UUID is never included in a timing record.
    pub fn for_operation(id: Uuid) -> Self {
        Self {
            operation: crate::operation_numbers::number(id),
        }
    }

    /// Emit a start and return a guard that emits the matching finish on drop.
    pub fn start(self, phase: Phase) -> Timer {
        let timer = Timer {
            context: self,
            phase,
            sequence: NEXT_PHASE.fetch_add(1, Ordering::Relaxed),
            started: Instant::now(),
            outcome: None,
            files: None,
            bytes: None,
        };
        timer.emit(false);
        timer
    }

    /// Time synchronous work, record its outcome, and return its result unchanged.
    pub fn measure<T, E>(self, phase: Phase, work: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        let timer = self.start(phase);
        timer.result(work())
    }
}

/// Emits a finish on early return, future cancellation, and panic too. No I/O,
/// networking or state lock: the existing tracing subscriber owns delivery.
pub struct Timer {
    context: Context,
    phase: Phase,
    sequence: u64,
    started: Instant,
    outcome: Option<Outcome>,
    files: Option<u64>,
    bytes: Option<u64>,
}

impl Timer {
    /// Attach successful scan counts to the finish; no per-file data is emitted.
    pub fn workload(&mut self, files: u64, bytes: u64) {
        self.files = Some(files);
        self.bytes = Some(bytes);
    }

    /// Set the explicit outcome and consume the guard, emitting its finish on drop.
    pub fn finish(mut self, outcome: Outcome) {
        self.outcome = Some(outcome);
    }

    /// Set success/error from the unchanged result and emit the finish on drop.
    pub fn result<T, E>(mut self, result: Result<T, E>) -> Result<T, E> {
        self.outcome = Some(if result.is_ok() {
            Outcome::Succeeded
        } else {
            Outcome::Failed
        });
        result
    }

    fn emit(&self, finished: bool) {
        let operation = self.context.operation;
        let phase = self.phase.code();
        let phase_sequence = self.sequence;
        let event = if finished {
            "phase_finished"
        } else {
            "phase_started"
        };
        let outcome = self.outcome.map_or_else(
            || {
                if std::thread::panicking() {
                    "panicked"
                } else {
                    "interrupted"
                }
            },
            Outcome::code,
        );
        let duration_ms =
            finished.then(|| u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX));
        macro_rules! record {
            ($target:literal) => {
                tracing::info!(target: $target, event, phase, operation, phase_sequence,
                    outcome = finished.then_some(outcome), duration_ms,
                    files = self.files, bytes = self.bytes, "native phase timing");
            };
        }
        if self.phase.adoption() {
            record!("adoption");
        } else {
            record!("launcher.launch");
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.emit(true);
    }
}

#[cfg(test)]
mod tests;
