//! The consent gate, the queue lock, and the two queue calls that need no
//! session: startup recovery and an event queued before any session.
use super::{TelemetryEvent, TransportError};
use crate::game_telemetry::Identity;
use crate::storage::DesktopState;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use std::path::PathBuf;

/// Core's queue is drain-then-requeue, so two writers in one process could
/// lose an event between them. The state directory's owner lock already
/// keeps other processes out.
pub(super) static QUEUE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Core's queue file names, inside the state directory.
const QUEUE_FILES: [&str; 2] = ["telemetry-queue.jsonl", "telemetry-queue.dropped"];

/// Proof the player has opted in, read under the state lock and used after
/// it is released. The transport cannot be opened without one.
#[derive(Debug, Clone)]
pub struct Consent {
    pub(super) identity: Identity,
    pub(super) queue_dir: PathBuf,
}

impl Consent {
    /// `Ok(None)` when the player has not opted in to game telemetry.
    pub fn read(state: &DesktopState) -> Result<Option<Self>, TransportError> {
        let choice = state.game_telemetry().map_err(|_| TransportError::Choice)?;
        Ok(choice
            .opted_in
            .then_some(choice.identity)
            .flatten()
            .map(|identity| Self {
                identity,
                queue_dir: state.state_root().to_path_buf(),
            }))
    }
}

/// What [`recover_on_startup`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    /// No queue on disk.
    Empty,
    /// Opted in: this many events wait for the next session's first flush.
    Pending(u64),
    /// Opted out: the queue a previous session left was deleted unsent.
    Discarded,
    /// The choice could not be read: nothing was touched.
    Untouched,
}

/// Run once when the launcher opens its state directory. An opted-in
/// player's queue is checked (a line torn by a crash is dropped) and kept;
/// an opted-out player's is deleted, so it never ships after a later
/// opt-in. The choice is read now; the returned future holds no borrow of
/// `state`, so the state lock need not be held across it.
pub fn recover_on_startup(
    state: &DesktopState,
) -> impl std::future::Future<Output = Recovery> + Send + 'static {
    let root = state.state_root().to_path_buf();
    let opted_in = state.game_telemetry().map(|choice| choice.opted_in);
    async move {
        let _queue = QUEUE_LOCK.lock().await;
        recover_at(&root, opted_in)
    }
}

fn recover_at(
    root: &std::path::Path,
    opted_in: Result<bool, crate::storage::StorageError>,
) -> Recovery {
    match opted_in {
        Ok(true) => match cimmeria_launcher_core::telemetry::recover_pending_on_startup(root) {
            0 => Recovery::Empty,
            n => Recovery::Pending(n),
        },
        Ok(false) => {
            let mut removed = false;
            for name in QUEUE_FILES {
                match std::fs::remove_file(root.join(name)) {
                    Ok(()) => removed = true,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => tracing::warn!(file = name, "opted-out telemetry queue not deleted"),
                }
            }
            if removed {
                tracing::info!("telemetry queue from before the opt-out deleted unsent");
                Recovery::Discarded
            } else {
                Recovery::Empty
            }
        }
        Err(_) => Recovery::Untouched,
    }
}

/// Queue an event that has no session to go with (an install result):
/// it is sent by the next session's first flush. `seq` stays as given.
pub async fn queue_without_session(
    consent: &Consent,
    event: TelemetryEvent,
) -> Result<(), TransportError> {
    let _queue = QUEUE_LOCK.lock().await;
    DiskQueue::new(&consent.queue_dir)
        .enqueue(&event)
        .map_err(|_| TransportError::Queue)
}
