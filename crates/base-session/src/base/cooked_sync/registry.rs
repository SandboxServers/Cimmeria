//! Per-session resync bookkeeping: which categories are queued or being
//! pushed, and what waits for the sync to finish.
//!
//! Keyed by client address, but an address can outlive its session (a
//! relog from the same port), so each entry also holds a `Weak` to the
//! session's `next_seq` counter, which every session allocates fresh. An
//! entry whose session is gone or replaced is stale: its task stops at its
//! next check and a new session gets a new entry.

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, LazyLock, Mutex, Weak};

use super::super::ConnectedClientState;

/// Work that must wait until the session's resync has finished (world
/// entry). Run on a spawned task once the last category is pushed; dropped
/// if the session goes away first.
pub type DeferredAction = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// One category to resync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncJob {
    pub category_id: u32,
    pub client_version: u32,
    pub server_version: u32,
}

pub(super) struct SessionSync {
    pub(super) session: Weak<AtomicU32>,
    pub(super) queue: VecDeque<SyncJob>,
    pub(super) active: Option<u32>,
    pub(super) deferred: Vec<DeferredAction>,
}

impl SessionSync {
    fn is_session(&self, token: &Arc<AtomicU32>) -> bool {
        self.session
            .upgrade()
            .is_some_and(|live| Arc::ptr_eq(&live, token))
    }
}

pub(super) static SYNCS: LazyLock<Mutex<HashMap<SocketAddr, SessionSync>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The session identity token for `addr`: its `next_seq` counter.
pub(super) fn session_token(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
) -> Option<Arc<AtomicU32>> {
    let clients = connected.lock().ok()?;
    clients.get(&addr).map(|c| Arc::clone(&c.next_seq))
}

/// What [`enqueue`] did with a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    /// No task was running for the session: the caller must spawn one.
    StartTask,
    /// A task is running and will take the job after the ones before it.
    Queued,
    /// The category is already being pushed or waiting: nothing to add.
    AlreadyPending,
    /// The session is not connected.
    NoSession,
}

pub(super) fn enqueue(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
    job: SyncJob,
) -> (EnqueueOutcome, Option<Arc<AtomicU32>>) {
    let Some(token) = session_token(connected, addr) else {
        return (EnqueueOutcome::NoSession, None);
    };
    let mut syncs = SYNCS.lock().unwrap_or_else(|p| p.into_inner());
    let fresh = || SessionSync {
        session: Arc::downgrade(&token),
        queue: VecDeque::new(),
        active: None,
        deferred: Vec::new(),
    };
    let entry = syncs.entry(addr).or_insert_with(fresh);
    if !entry.is_session(&token) {
        // A previous session's leftovers: its task stops on its own.
        *entry = fresh();
    }
    if entry.active == Some(job.category_id)
        || entry.queue.iter().any(|j| j.category_id == job.category_id)
    {
        return (EnqueueOutcome::AlreadyPending, Some(token));
    }
    let running = entry.active.is_some() || !entry.queue.is_empty();
    entry.queue.push_back(job);
    let outcome = if running {
        EnqueueOutcome::Queued
    } else {
        // Mark the entry busy before the task is spawned, so a second job
        // arriving first queues behind this one instead of spawning again.
        entry.active = Some(job.category_id);
        EnqueueOutcome::StartTask
    };
    (outcome, Some(token))
}

/// Whether a resync is queued or running for the session at `addr`.
pub fn is_syncing(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
) -> bool {
    let Some(token) = session_token(connected, addr) else {
        return false;
    };
    let syncs = SYNCS.lock().unwrap_or_else(|p| p.into_inner());
    syncs
        .get(&addr)
        .is_some_and(|s| s.is_session(&token) && (s.active.is_some() || !s.queue.is_empty()))
}

/// Hold `action` until the session's resync finishes. Hands it back when no
/// resync is running, so the caller runs it straight away.
pub fn defer_until_synced(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
    action: DeferredAction,
) -> Result<(), DeferredAction> {
    let Some(token) = session_token(connected, addr) else {
        return Err(action);
    };
    let mut syncs = SYNCS.lock().unwrap_or_else(|p| p.into_inner());
    match syncs.get_mut(&addr) {
        Some(s) if s.is_session(&token) && (s.active.is_some() || !s.queue.is_empty()) => {
            s.deferred.push(action);
            Ok(())
        }
        _ => Err(action),
    }
}

/// What the task gets back when it asks for its next job.
pub(super) enum NextJob {
    Job(SyncJob, usize),
    /// Every job is done: run these and stop.
    Done(Vec<DeferredAction>),
    /// The session is gone or replaced: these jobs were never pushed.
    SessionGone(Vec<SyncJob>),
}

/// Take the session's next job, or finish. Checks that `token` is still the
/// connected session's; if not, the queued jobs are handed back as lost.
pub(super) fn next_job(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
    token: &Arc<AtomicU32>,
) -> NextJob {
    let live = session_token(connected, addr).is_some_and(|t| Arc::ptr_eq(&t, token));
    if !live {
        return NextJob::SessionGone(abandon(addr, token));
    }
    let mut syncs = SYNCS.lock().unwrap_or_else(|p| p.into_inner());
    let Some(entry) = syncs.get_mut(&addr).filter(|e| e.is_session(token)) else {
        return NextJob::SessionGone(Vec::new());
    };
    match entry.queue.pop_front() {
        Some(job) => {
            entry.active = Some(job.category_id);
            NextJob::Job(job, entry.queue.len())
        }
        None => {
            let deferred = std::mem::take(&mut entry.deferred);
            syncs.remove(&addr);
            NextJob::Done(deferred)
        }
    }
}

/// Drop the session's entry after its task abandoned the sync, returning the
/// jobs that were still queued. Leaves a newer session's entry alone.
pub(super) fn abandon(addr: SocketAddr, token: &Arc<AtomicU32>) -> Vec<SyncJob> {
    let mut syncs = SYNCS.lock().unwrap_or_else(|p| p.into_inner());
    let owned = syncs.get(&addr).is_some_and(|s| {
        s.session
            .upgrade()
            .is_none_or(|live| Arc::ptr_eq(&live, token))
    });
    if !owned {
        return Vec::new();
    }
    syncs
        .remove(&addr)
        .map(|s| s.queue.into_iter().collect())
        .unwrap_or_default()
}
