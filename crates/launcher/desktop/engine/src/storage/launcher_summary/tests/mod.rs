//! Shared fixtures: a tempdir state root, clocks that move only when a test
//! moves them, and ids that count up. Only the `exporter` tests send anything,
//! and only to a loopback mock.
use super::{
    queue::{Queue, FILE},
    tracker::{lock, Gate, Sources},
    *,
};
use crate::{
    catalog::VerifiedRelease,
    client_setup::login_servers::default_servers,
    storage::{install_worker::fixtures, DesktopState},
    OperationKind,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

mod batch_seam;
mod consent;
pub(in crate::storage) mod exporter;
mod golden;
mod isolation;
mod journal;
mod launch_rows;
mod pre_admission;
mod queue_file;
mod restart;
mod timings;

/// Loopback and never contacted by a test that configures it.
pub(in crate::storage) const ENDPOINT: &str = "http://127.0.0.1:9";
pub(in crate::storage) const T0: u64 = 1_800_000_000;

/// The n-th id minted by a test clock.
pub(in crate::storage) fn minted(n: u64) -> Uuid {
    Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0000 | u128::from(n))
}

#[derive(Clone)]
pub(in crate::storage) struct Clock {
    unix_s: Arc<AtomicU64>,
    monotonic_ms: Arc<AtomicU64>,
}
impl Clock {
    /// Replaces the wall clock, the monotonic clock and the id source.
    pub(in crate::storage) fn install(state: &mut DesktopState) -> Self {
        Self::install_after(state, 0)
    }

    /// As `install`, minting ids from `minted(issued + 1)`: a second process
    /// must not repeat the ids of the first.
    fn install_after(state: &mut DesktopState, issued: u64) -> Self {
        let clock = Self {
            unix_s: Arc::new(AtomicU64::new(T0)),
            monotonic_ms: Arc::new(AtomicU64::new(0)),
        };
        let (unix_s, monotonic_ms) = (clock.unix_s.clone(), clock.monotonic_ms.clone());
        let mut next = issued;
        lock(&state.summaries).sources = Sources {
            unix_s: Box::new(move || unix_s.load(Ordering::SeqCst)),
            monotonic: Box::new(move || Duration::from_millis(monotonic_ms.load(Ordering::SeqCst))),
            new_id: Box::new(move || {
                next += 1;
                minted(next)
            }),
        };
        clock
    }
    pub(in crate::storage) fn advance_ms(&self, milliseconds: u64) {
        self.monotonic_ms.fetch_add(milliseconds, Ordering::SeqCst);
    }
    pub(in crate::storage) fn advance_s(&self, seconds: u64) {
        self.unix_s.fetch_add(seconds, Ordering::SeqCst);
    }
}

pub(in crate::storage) fn config(endpoint: Option<&str>) -> SummaryConfig {
    SummaryConfig {
        launcher_version: (0, 1, 0),
        endpoint: endpoint.map(|endpoint| SummaryEndpoint::parse(endpoint).unwrap()),
    }
}

/// Open, install the test clock and configure with the loopback endpoint.
pub(in crate::storage) fn configured(root: &Path) -> (DesktopState, Clock) {
    let mut state = DesktopState::open(root).unwrap();
    let clock = Clock::install(&mut state);
    state.configure_summaries(config(Some(ENDPOINT)));
    (state, clock)
}

/// A later process on the same state root. Its ids start at `minted(101)`.
fn reopened(root: &Path) -> (DesktopState, Clock) {
    let mut state = DesktopState::open(root).unwrap();
    let clock = Clock::install_after(&mut state, 100);
    state.configure_summaries(config(Some(ENDPOINT)));
    (state, clock)
}

/// A configured state root whose user has opted in. Layout: `<temp>/state`.
pub(in crate::storage) fn opted_in() -> (tempfile::TempDir, DesktopState, Clock) {
    let root = tempfile::tempdir().unwrap();
    let (mut state, clock) = configured(&root.path().join("state"));
    set_consent(&mut state, true);
    (root, state, clock)
}

pub(in crate::storage) fn set_consent(state: &mut DesktopState, consent: bool) {
    let preferences = state.preferences().clone();
    state
        .save_preferences(preferences.install_directory, consent, preferences.revision)
        .unwrap();
}

/// What is queued in memory, oldest first.
pub(in crate::storage) fn queued(state: &DesktopState) -> Vec<Summary> {
    lock(&state.summaries)
        .queue
        .entries
        .iter()
        .map(|entry| entry.summary.clone())
        .collect()
}

pub(in crate::storage) fn queue_bytes(state: &DesktopState) -> Option<Vec<u8>> {
    std::fs::read(state.state_root().join(FILE)).ok()
}

/// The timed phase the tracked attempt is in, if one is tracked.
pub(in crate::storage) fn open_phase(state: &DesktopState) -> Option<TimedPhase> {
    lock(&state.summaries).open_phase()
}

/// The exact ingest body the exporter would send now, if any.
pub(in crate::storage) fn upload_body(state: &mut DesktopState) -> Option<Vec<u8>> {
    match state.summary_take_batch() {
        batch::Take::Ready(batch) => Some(serde_json::to_vec(&batch.request()).unwrap()),
        _ => None,
    }
}

fn stored(state: &DesktopState) -> Queue {
    serde_json::from_slice(&queue_bytes(state).expect("queue file")).unwrap()
}

fn gate(state: &mut DesktopState) -> Gate {
    state.finalize_summaries();
    lock(&state.summaries).gate()
}

fn release() -> VerifiedRelease {
    fixtures::verified(&fixtures::archive(true))
}

/// Selects `<temp>/install` and admits a first install through the real path.
fn admit_install(state: &mut DesktopState) -> Uuid {
    if state.preferences().install_directory.is_none() {
        let preferences = state.preferences().clone();
        let directory = state.state_root().parent().unwrap().join("install");
        state
            .save_preferences(
                Some(directory),
                preferences.launcher_summary_consent,
                preferences.revision,
            )
            .unwrap();
    }
    let id = Uuid::new_v4();
    let (operation, preferences) = (
        state.operations().snapshot().revision,
        state.preferences().revision,
    );
    assert!(
        state
            .admit_install(id, operation, preferences, &release(), default_servers())
            .unwrap()
            .dispatch
    );
    id
}

fn observe(state: &mut DesktopState, id: Uuid, next: OperationState) {
    state.operations_mut().unwrap().observe(id, next).unwrap();
}

/// The install worker's own terminal sequence: result record, then the commit.
fn finish_install(state: &mut DesktopState, id: Uuid, outcome: install_worker::Outcome) {
    use install_worker::Outcome;
    state.prepare_install_result(id, outcome).unwrap();
    let terminal = match outcome {
        Outcome::ContentPrepared => OperationState::Succeeded,
        Outcome::Cancelled => OperationState::Cancelled,
        _ => OperationState::Failed,
    };
    observe(state, id, terminal);
}

/// Admits a journal operation of any kind without its workflow's files.
fn begin(state: &mut DesktopState, kind: OperationKind) -> Uuid {
    let id = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    assert!(
        state
            .operations_mut()
            .unwrap()
            .begin(id, kind, [3; 32], revision)
            .unwrap()
            .1
    );
    id
}
