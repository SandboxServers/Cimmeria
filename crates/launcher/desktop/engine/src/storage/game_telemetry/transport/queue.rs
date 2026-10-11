//! The consent gate, the flush turn, and the queue calls that need no
//! session: startup recovery, discarding on a change of choice, and an
//! event queued before any session.
use super::{TelemetryEvent, TransportError};
use crate::game_telemetry::Identity;
use crate::storage::DesktopState;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
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

/// How many times the queue in this directory has been discarded since the
/// process started. A change of choice discards it, so a [`Consent`] whose
/// count is no longer current belongs to a choice the player has since
/// changed: a transport opened on it must stop, though the game that
/// session started keeps running. In memory only: a consent never outlives
/// the process.
fn discards(queue_dir: &Path) -> Arc<AtomicU64> {
    static COUNTS: Mutex<BTreeMap<PathBuf, Arc<AtomicU64>>> = Mutex::new(BTreeMap::new());
    COUNTS
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
    /// [`discards`] when this was read.
    pub(super) discards: u64,
}

impl Consent {
    /// `Ok(None)` when the player has not opted in to game telemetry.
    pub fn read(state: &DesktopState) -> Result<Option<Self>, TransportError> {
        let choice = state.game_telemetry().map_err(|_| TransportError::Choice)?;
        let queue_dir = state.state_root().to_path_buf();
        // Under the state lock, as every change of choice is.
        let discards = discards(&queue_dir).load(Ordering::SeqCst);
        Ok(choice
            .opted_in
            .then_some(choice.identity)
            .flatten()
            .map(|identity| Self {
                identity,
                queue_dir,
                discards,
            }))
    }

    /// False once the choice this consent was read under has changed. No
    /// lock and no disk read, so it is cheap enough to ask before every
    /// queue write.
    pub fn current(&self) -> bool {
        discards(&self.queue_dir).load(Ordering::SeqCst) == self.discards
    }

    /// After a write to the queue: if the choice changed while it ran, the
    /// write may have re-created a queue the change deleted, so delete it
    /// again. The change bumps the count before it deletes, so either the
    /// change's delete or this one comes after the write.
    ///
    /// The delete does not count as a change of choice: counting it would
    /// stop a session opened on a newer consent (after an opt-out and an
    /// opt-in). It does take that session's queued events with it, rather
    /// than let this stale write ship under the newer consent.
    pub(super) fn undo_if_withdrawn(&self) -> Result<(), TransportError> {
        if self.current() {
            return Ok(());
        }
        if DiskQueue::new(&self.queue_dir).discard().is_err() {
            tracing::warn!("withdrawn game telemetry write not deleted");
        }
        Err(TransportError::NotOptedIn)
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

/// [`recover_on_startup`] on the calling thread, for a caller that holds the
/// state lock and may not be on the async runtime (the launcher opening its
/// state directory). Run under the lock, it cannot race a change of choice.
pub fn recover_now(state: &DesktopState) -> Recovery {
    recover_at(
        state.state_root(),
        state.game_telemetry().map(|choice| choice.opted_in),
    )
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
    // First, so a transport that writes after this sees it and deletes
    // again (`Consent::undo_if_withdrawn`).
    discards(root).fetch_add(1, Ordering::SeqCst);
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
    if !consent.current() {
        return Err(TransportError::NotOptedIn);
    }
    queue_now(consent, &event)?;
    consent.undo_if_withdrawn()
}

/// [`queue_without_session`] for callers that hold the state lock and
/// cannot await: core's queue locks its own file only for the append.
pub(crate) fn queue_now(consent: &Consent, event: &TelemetryEvent) -> Result<(), TransportError> {
    DiskQueue::new(&consent.queue_dir)
        .enqueue(event)
        .map_err(|_| TransportError::Queue)
}
