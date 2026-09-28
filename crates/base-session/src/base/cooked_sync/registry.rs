//! Per-session resync bookkeeping: the queued categories, the served-miss
//! queue, the miss rate limiter, and the world entry waiting on the held
//! categories.
//!
//! Keyed by client address, but an address can outlive its session (a
//! relog from the same port), so each entry also holds a `Weak` to the
//! session's `next_seq` counter, which every session allocates fresh. An
//! entry whose session is gone or replaced is stale: its task stops at its
//! next check, a new session gets a new entry, and dead entries are pruned
//! whenever the registry is touched.

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, LazyLock, Mutex, Weak};
use std::time::Instant;

use super::super::ConnectedClientState;
use super::order::{is_held, rank};
use super::{MISS_BURST, MISS_QUEUE_CAP, MISS_RATE_PER_SEC};

/// Work that must wait until the session's held categories are resynced
/// (world entry). Run on a spawned task once the last held category is
/// pushed; dropped if the session goes away first.
pub type DeferredAction = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// One category to resync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncJob {
    pub category_id: u32,
    pub client_version: u32,
    pub server_version: u32,
}

/// One entry the client asked for (`elementDataRequest`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Miss {
    pub(crate) category_id: u32,
    pub(crate) key: u32,
    pub(crate) requested_at: Instant,
}

/// Token bucket for `elementDataRequest`: [`MISS_BURST`] at once, refilled
/// at [`MISS_RATE_PER_SEC`].
#[derive(Debug)]
struct MissLimiter {
    tokens: f64,
    last: Instant,
}

impl MissLimiter {
    fn new(now: Instant) -> Self {
        Self {
            tokens: MISS_BURST as f64,
            last: now,
        }
    }

    fn take(&mut self, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + elapsed * MISS_RATE_PER_SEC as f64).min(MISS_BURST as f64);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

pub(super) struct SessionSync {
    session: Weak<AtomicU32>,
    queue: VecDeque<SyncJob>,
    active: Option<u32>,
    task_running: bool,
    misses: VecDeque<Miss>,
    deferred: Vec<DeferredAction>,
    limiter: MissLimiter,
    /// When each refusal reason was last logged, and how many were
    /// suppressed since (the WARN is throttled per session and reason).
    pub(super) refusals: HashMap<&'static str, (Instant, u32)>,
}

impl SessionSync {
    fn new(token: &Arc<AtomicU32>, now: Instant) -> Self {
        Self {
            session: Arc::downgrade(token),
            queue: VecDeque::new(),
            active: None,
            task_running: false,
            misses: VecDeque::new(),
            deferred: Vec::new(),
            limiter: MissLimiter::new(now),
            refusals: HashMap::new(),
        }
    }

    fn is_session(&self, token: &Arc<AtomicU32>) -> bool {
        self.session
            .upgrade()
            .is_some_and(|live| Arc::ptr_eq(&live, token))
    }

    fn holds_world_entry(&self) -> bool {
        self.active.is_some_and(is_held) || self.queue.iter().any(|j| is_held(j.category_id))
    }

    fn busy(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty() || !self.misses.is_empty()
    }
}

pub(super) static SYNCS: LazyLock<Mutex<HashMap<SocketAddr, SessionSync>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn lock() -> std::sync::MutexGuard<'static, HashMap<SocketAddr, SessionSync>> {
    SYNCS.lock().unwrap_or_else(|p| p.into_inner())
}

/// The session identity token for `addr`: its `next_seq` counter.
pub(super) fn session_token(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
) -> Option<Arc<AtomicU32>> {
    let clients = connected.lock().ok()?;
    clients.get(&addr).map(|c| Arc::clone(&c.next_seq))
}

/// The session's entry, created (or replaced, if a previous session's) on
/// demand. Also prunes entries whose session is gone and whose task is not
/// running.
fn entry<'a>(
    syncs: &'a mut HashMap<SocketAddr, SessionSync>,
    addr: SocketAddr,
    token: &Arc<AtomicU32>,
    now: Instant,
) -> &'a mut SessionSync {
    syncs.retain(|a, s| *a == addr || s.task_running || s.session.strong_count() > 0);
    let e = syncs
        .entry(addr)
        .or_insert_with(|| SessionSync::new(token, now));
    if !e.is_session(token) {
        // A previous session's leftovers: its task stops on its own.
        *e = SessionSync::new(token, now);
    }
    e
}

/// What [`enqueue`] did with a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    /// No task was running for the session: the caller must spawn one.
    StartTask,
    /// A task is running and will take the job in rank order.
    Queued,
    /// The category is already being pushed or waiting: nothing to add.
    AlreadyPending,
    /// The session is not connected.
    NoSession,
}

/// Queue `job` in rank order (held categories first, TextStrings last).
pub(super) fn enqueue(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
    job: SyncJob,
) -> (EnqueueOutcome, Option<Arc<AtomicU32>>) {
    let Some(token) = session_token(connected, addr) else {
        return (EnqueueOutcome::NoSession, None);
    };
    let mut syncs = lock();
    let e = entry(&mut syncs, addr, &token, Instant::now());
    if e.active == Some(job.category_id) || e.queue.iter().any(|j| j.category_id == job.category_id)
    {
        return (EnqueueOutcome::AlreadyPending, Some(token));
    }
    let at = e
        .queue
        .iter()
        .position(|j| rank(j.category_id) > rank(job.category_id))
        .unwrap_or(e.queue.len());
    e.queue.insert(at, job);
    let outcome = if e.task_running {
        EnqueueOutcome::Queued
    } else {
        e.task_running = true;
        EnqueueOutcome::StartTask
    };
    (outcome, Some(token))
}

/// Why a miss was not queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissRefusal {
    UnknownCategory,
    UnknownKey,
    RateLimited,
    QueueFull,
    NoSession,
}

impl MissRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::UnknownCategory => "unknown_category",
            Self::UnknownKey => "unknown_key",
            Self::RateLimited => "rate_limited",
            Self::QueueFull => "queue_full",
            Self::NoSession => "no_session",
        }
    }
}

/// What [`queue_miss`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissOutcome {
    /// Queued; the caller must spawn the task if `start_task`.
    Queued { start_task: bool },
    /// The same entry is already waiting to be sent.
    Duplicate,
    /// Refused; `log` says whether this refusal should be logged now
    /// (throttled), with the count suppressed since the last one.
    Refused { why: MissRefusal, log: Option<u32> },
}

/// Seconds between two WARNs for the same refusal reason on one session.
const REFUSAL_LOG_INTERVAL_SECS: u64 = 5;

pub(super) fn refuse(
    addr: SocketAddr,
    token: &Arc<AtomicU32>,
    why: MissRefusal,
    now: Instant,
) -> MissOutcome {
    let mut syncs = lock();
    let e = entry(&mut syncs, addr, token, now);
    let slot = e.refusals.entry(why.reason()).or_insert((now, u32::MAX));
    let log = if slot.1 == u32::MAX
        || now.saturating_duration_since(slot.0).as_secs() >= REFUSAL_LOG_INTERVAL_SECS
    {
        let suppressed = if slot.1 == u32::MAX { 0 } else { slot.1 };
        *slot = (now, 0);
        Some(suppressed)
    } else {
        slot.1 += 1;
        None
    };
    MissOutcome::Refused { why, log }
}

/// Queue a valid miss for the session, ahead of its background stream.
pub(super) fn queue_miss(
    token: &Arc<AtomicU32>,
    addr: SocketAddr,
    category_id: u32,
    key: u32,
    now: Instant,
) -> MissOutcome {
    let mut syncs = lock();
    let e = entry(&mut syncs, addr, token, now);
    if e.misses
        .iter()
        .any(|m| m.category_id == category_id && m.key == key)
    {
        return MissOutcome::Duplicate;
    }
    let why = if e.misses.len() >= MISS_QUEUE_CAP {
        Some(MissRefusal::QueueFull)
    } else if !e.limiter.take(now) {
        Some(MissRefusal::RateLimited)
    } else {
        None
    };
    if let Some(why) = why {
        drop(syncs);
        return refuse(addr, token, why, now);
    }
    e.misses.push_back(Miss {
        category_id,
        key,
        requested_at: now,
    });
    let start_task = !e.task_running;
    e.task_running = true;
    MissOutcome::Queued { start_task }
}

/// Whether a resync or a served miss is queued or running for `addr`.
pub fn is_syncing(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
) -> bool {
    let Some(token) = session_token(connected, addr) else {
        return false;
    };
    lock()
        .get(&addr)
        .is_some_and(|s| s.is_session(&token) && s.busy())
}

/// Whether world entry must wait: a held category (see
/// [`super::order::HELD_CATEGORIES`]) is queued or being pushed.
pub fn holds_world_entry(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
) -> bool {
    let Some(token) = session_token(connected, addr) else {
        return false;
    };
    lock()
        .get(&addr)
        .is_some_and(|s| s.is_session(&token) && s.holds_world_entry())
}

/// Hold `action` until the session's held categories are resynced. Hands
/// it back when none is pending, so the caller runs it straight away.
pub fn defer_until_synced(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
    action: DeferredAction,
) -> Result<(), DeferredAction> {
    let Some(token) = session_token(connected, addr) else {
        return Err(action);
    };
    let mut syncs = lock();
    match syncs.get_mut(&addr) {
        Some(s) if s.is_session(&token) && s.holds_world_entry() => {
            s.deferred.push(action);
            Ok(())
        }
        _ => Err(action),
    }
}

/// The task's next piece of work, in priority order.
pub(super) enum Next {
    /// A served miss: send this entry now.
    Miss(Miss),
    /// Keep pushing the category already in progress.
    Continue,
    /// Start this category.
    Start(SyncJob),
    /// Nothing left: the task stops (the entry stays for the rate limiter).
    Idle,
    /// The session is gone or replaced.
    Gone,
}

/// Decide the task's next step. `in_progress` is whether it is part-way
/// through a category.
pub(super) fn next(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
    token: &Arc<AtomicU32>,
    in_progress: bool,
) -> Next {
    let live = session_token(connected, addr).is_some_and(|t| Arc::ptr_eq(&t, token));
    let mut syncs = lock();
    let Some(e) = syncs.get_mut(&addr).filter(|e| e.is_session(token)) else {
        return Next::Gone;
    };
    if !live {
        return Next::Gone;
    }
    if let Some(miss) = e.misses.pop_front() {
        return Next::Miss(miss);
    }
    if in_progress {
        return Next::Continue;
    }
    match e.queue.pop_front() {
        Some(job) => {
            e.active = Some(job.category_id);
            Next::Start(job)
        }
        None => {
            e.active = None;
            e.task_running = false;
            Next::Idle
        }
    }
}

/// Record that the task finished its current category. Returns the world
/// entry to release if that was the last held category.
pub(super) fn finish_job(addr: SocketAddr, token: &Arc<AtomicU32>) -> Vec<DeferredAction> {
    let mut syncs = lock();
    let Some(e) = syncs.get_mut(&addr).filter(|e| e.is_session(token)) else {
        return Vec::new();
    };
    e.active = None;
    if e.holds_world_entry() {
        Vec::new()
    } else {
        std::mem::take(&mut e.deferred)
    }
}

/// Drop the session's entry after its task gave up, returning the jobs
/// that were still queued. Leaves a newer session's entry alone.
pub(super) fn abandon(addr: SocketAddr, token: &Arc<AtomicU32>) -> Vec<SyncJob> {
    let mut syncs = lock();
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
