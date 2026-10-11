//! The consent gate, the flush turn, and the queue calls that need no
//! session: startup recovery, discarding on a change of choice, and an
//! event queued before any session.
use super::{TelemetryEvent, TransportError};
use crate::game_telemetry::Identity;
use crate::storage::DesktopState;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// One flush or bundle upload at a time per queue directory, across every
/// transport in the process: two flushes would send the same in-flight
/// chunk. Core's queue locks its own files, so enqueues never wait on this;
/// the state directory's owner lock keeps other processes out.
pub(super) fn flush_turn(queue_dir: &Path) -> Arc<tokio::sync::Mutex<()>> {
    static TURNS: Mutex<BTreeMap<PathBuf, Arc<tokio::sync::Mutex<()>>>> =
        Mutex::new(BTreeMap::new());
    TURNS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(queue_dir.to_path_buf())
        .or_default()
        .clone()
}

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
    async move { recover_at(&root, opted_in) }
}

fn recover_at(root: &Path, opted_in: Result<bool, crate::storage::StorageError>) -> Recovery {
    match opted_in {
        Ok(true) => match cimmeria_launcher_core::telemetry::recover_pending_on_startup(root) {
            0 => Recovery::Empty,
            n => Recovery::Pending(n),
        },
        Ok(false) => {
            if discard_queue(root) {
                Recovery::Discarded
            } else {
                Recovery::Empty
            }
        }
        Err(_) => Recovery::Untouched,
    }
}

/// Delete the queue, in-flight chunk and dropped-lines counter unsent.
/// Called whenever the game-telemetry choice changes (nothing may be queued
/// while opted out, so an opt-in starts empty too) and at startup for an
/// opted-out player. Returns whether anything was deleted; a failure is
/// logged and retried at the next change or startup.
pub(crate) fn discard_queue(root: &Path) -> bool {
    match DiskQueue::new(root).discard() {
        Ok(true) => {
            tracing::info!("game telemetry queue deleted unsent");
            true
        }
        Ok(false) => false,
        Err(_) => {
            tracing::warn!("game telemetry queue not deleted");
            false
        }
    }
}

/// Queue an event that has no session to go with (an install result):
/// it is sent by the next session's first flush. `seq` stays as given.
pub async fn queue_without_session(
    consent: &Consent,
    event: TelemetryEvent,
) -> Result<(), TransportError> {
    DiskQueue::new(&consent.queue_dir)
        .enqueue(&event)
        .map_err(|_| TransportError::Queue)
}
