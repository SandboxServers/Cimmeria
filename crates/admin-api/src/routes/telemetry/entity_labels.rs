//! Naming the entity IDs in replayed client rows (NT-40).
//!
//! An entity ID is a recycled slot, and a client row reaches the server
//! seconds to minutes after the client wrote it, so the ID is named by
//! asking the cell who held the slot *when the row was written*:
//! `SpaceManager::entity_label_at(space_id, entity_id, at)`.
//!
//! **These names are client-claimed.** The entity ID, the space and the
//! timestamp all come from the uploaded row, and the uploader is anyone who
//! minted a dev-session token. The clock mapping below corrects an honest
//! client's skew; it does not authenticate the row, and a client can pick
//! its fields to make a row name any entity the rings remember. Every
//! replayed `client.native` row says so (`names_source = "client_claimed"`).
//!
//! - **The time, on the server clock.** Lifetimes are stamped with the
//!   server's wall clock and the client's clock is its own. For each chunk
//!   the session observes an offset: the server's receive time minus the
//!   newest `ts_ms` in it. That is the true clock difference plus the
//!   chunk's upload delay, never less, so the session uses the smallest
//!   offset seen in the last [`OFFSET_WINDOW`]: an old sample ages out, so
//!   a clock step heals within the window instead of skewing the rest of
//!   the session. A row's server time is `ts_ms + offset`, capped at the
//!   receive time. Receive time alone would place every row of a buffered
//!   chunk after the slot reuses that happened while it waited.
//! - **One question per entity per chunk.** A chunk asks once for each
//!   (space, entity) pair, at the time of the first admitted row that
//!   mentions it. So the cell's work is bounded by the distinct entities in
//!   a chunk ([`ENTITY_LABEL_QUERY_CAP`]), not by the client's timestamps.
//! - **The space.** Most rows carry no `space_id`. The client is in one
//!   space at a time, and every entity it creates or brings into view
//!   reports the space (`client.entity.create`, `.enter`,
//!   `.entered_world`), so each row is placed in the last space the session
//!   reported, carried from chunk to chunk. A row before the first report
//!   stays unnamed.
//! - **The cell, at arm's length.** The upload routes are public and have
//!   no router state, so the ingest reaches the cell through one
//!   process-wide [`EntityLabelLink`], set at boot
//!   ([`connect_entity_labels`]). It is a small channel of its own (never
//!   the base->cell gameplay channel), read by the cell loop only when no
//!   gameplay message is waiting, and at most one request is in flight
//!   process-wide. A busy link, a full channel or no reply within
//!   [`CELL_REPLY_TIMEOUT`] replays the chunk unnamed at once; the ingest
//!   never waits for a turn.
//! - **Only what the budget admits.** The upload handler gates a chunk on
//!   the session budget first and names only the rows it will replay.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use tokio::sync::{mpsc, oneshot, Semaphore};

use cimmeria_services::cell::messages::{
    EntityLabelQuery, EntityLabelsReply, EntityLabelsRequest, ENTITY_LABEL_QUERY_CAP,
};

use super::dto::TelemetryEvent;

/// The `client.native` keys that hold an entity ID. Each pairs with the
/// same prefix ending in `_name` (Rule 6).
pub(super) const ENTITY_KEYS: [&str; 4] = ["entity_id", "target_id", "source_id", "pet_id"];

/// How long a chunk waits for the cell's answer before replaying unnamed.
/// The cell answers between ticks, well inside this.
pub(super) const CELL_REPLY_TIMEOUT: Duration = Duration::from_millis(500);

/// How far back the smallest observed clock offset is taken from.
pub(super) const OFFSET_WINDOW: Duration = Duration::from_secs(5 * 60);

/// Offset samples kept per session, at most; the oldest go first.
const OFFSET_SAMPLES: usize = 64;

/// Sessions whose clock and space are remembered. The least recently seen
/// is forgotten first; a forgotten session starts over, and its next rows
/// stay unnamed until it reports a space again.
const MAX_SESSIONS: usize = 1024;

/// Sessions remembered per install (the token's `sub`). One install minting
/// many sessions evicts its own oldest, not other installs'.
pub(super) const MAX_SESSIONS_PER_INSTALL: usize = 4;

/// The ingest's way to the cell: the label channel and the one permit that
/// keeps at most one request in flight.
#[derive(Debug, Clone)]
pub struct EntityLabelLink {
    tx: mpsc::Sender<EntityLabelsRequest>,
    in_flight: Arc<Semaphore>,
}

impl EntityLabelLink {
    /// A link over `tx` with one request in flight at a time.
    pub fn new(tx: mpsc::Sender<EntityLabelsRequest>) -> Self {
        Self {
            tx,
            in_flight: Arc::new(Semaphore::new(1)),
        }
    }
}

static LINK: OnceLock<EntityLabelLink> = OnceLock::new();

/// Give the ingest the cell's entity-label channel, so replayed rows name
/// their entity IDs. Called once at boot, after the cell starts; later calls
/// are ignored. Until it is called, rows replay without entity names.
pub fn connect_entity_labels(tx: mpsc::Sender<EntityLabelsRequest>) {
    let _ = LINK.set(EntityLabelLink::new(tx));
}

/// The link [`connect_entity_labels`] set, if any.
pub(super) fn link() -> Option<&'static EntityLabelLink> {
    LINK.get()
}

/// What a session's earlier chunks established.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SessionClock {
    /// `(receive ms, observed offset ms)` per recent chunk, oldest first.
    samples: VecDeque<(i64, i64)>,
    /// The last space the client reported.
    pub(super) space_id: Option<u32>,
    /// The install the session belongs to.
    install_id: String,
    /// Server time of the last chunk, in ms, for eviction.
    last_seen_ms: i64,
}

impl SessionClock {
    /// A fresh clock whose session already reported `space_id`.
    #[cfg(test)]
    pub(super) fn in_space(space_id: u32) -> Self {
        Self {
            space_id: Some(space_id),
            ..Self::default()
        }
    }

    /// Record a chunk received at `recv_ms` whose newest row is `newest`,
    /// and forget samples older than [`OFFSET_WINDOW`].
    fn observe(&mut self, recv_ms: i64, newest: i64) {
        self.samples
            .push_back((recv_ms, recv_ms.saturating_sub(newest)));
        let window = i64::try_from(OFFSET_WINDOW.as_millis()).unwrap_or(i64::MAX);
        while self.samples.len() > OFFSET_SAMPLES
            || self
                .samples
                .front()
                .is_some_and(|(at, _)| recv_ms.saturating_sub(*at) > window)
        {
            self.samples.pop_front();
        }
    }

    /// The smallest offset in the window.
    pub(super) fn offset_ms(&self) -> Option<i64> {
        self.samples.iter().map(|(_, o)| *o).min()
    }
}

/// The process-wide clocks, by session.
#[derive(Debug, Default)]
pub(super) struct SessionClocks(HashMap<String, SessionClock>);

impl SessionClocks {
    /// `sid`'s clock, created if new. A new session first evicts its
    /// install's least recently seen session when the install is at
    /// [`MAX_SESSIONS_PER_INSTALL`], then the least recently seen of all
    /// when the table is at [`MAX_SESSIONS`].
    pub(super) fn entry(&mut self, sid: &str, install_id: &str, now_ms: i64) -> &mut SessionClock {
        if !self.0.contains_key(sid) {
            let install_count = self
                .0
                .values()
                .filter(|c| c.install_id == install_id)
                .count();
            if install_count >= MAX_SESSIONS_PER_INSTALL {
                self.evict_oldest(|c| c.install_id == install_id);
            }
            if self.0.len() >= MAX_SESSIONS {
                self.evict_oldest(|_| true);
            }
        }
        let clock = self
            .0
            .entry(sid.to_string())
            .or_insert_with(|| SessionClock {
                install_id: install_id.to_string(),
                ..SessionClock::default()
            });
        clock.last_seen_ms = now_ms;
        clock
    }

    fn evict_oldest(&mut self, which: impl Fn(&SessionClock) -> bool) {
        let oldest = self
            .0
            .iter()
            .filter(|(_, c)| which(c))
            .min_by_key(|(_, c)| c.last_seen_ms)
            .map(|(k, _)| k.clone());
        if let Some(oldest) = oldest {
            self.0.remove(&oldest);
        }
    }

    #[cfg(test)]
    pub(super) fn contains(&self, sid: &str) -> bool {
        self.0.contains_key(sid)
    }
}

static CLOCKS: Mutex<Option<SessionClocks>> = Mutex::new(None);

/// Run `f` on `sid`'s clock in the process-wide table. Poisoning is
/// ignored: the clock only names rows and must never take the ingest down.
fn with_session_clock<R>(
    sid: &str,
    install_id: &str,
    now_ms: i64,
    f: impl FnOnce(&mut SessionClock) -> R,
) -> R {
    let mut guard = CLOCKS.lock().unwrap_or_else(PoisonError::into_inner);
    f(guard
        .get_or_insert_with(SessionClocks::default)
        .entry(sid, install_id, now_ms))
}

/// The entity names for one chunk's rows, resolved by the cell.
#[derive(Debug, Default)]
pub(super) struct EntityLabels {
    /// Per row (by index in the chunk): the space it was placed in.
    spaces: Vec<Option<u32>>,
    /// The cell's answer for each (space, entity) pair it knew.
    labels: HashMap<(u32, u32), &'static str>,
}

impl EntityLabels {
    /// No names: what a replay without the cell uses.
    pub(super) fn none() -> Self {
        Self::default()
    }

    /// The label of `entity_id` as row `row` saw it, if the cell knew it.
    pub(super) fn label(&self, row: usize, entity_id: u32) -> Option<&'static str> {
        let space = (*self.spaces.get(row)?)?;
        self.labels.get(&(space, entity_id)).copied()
    }
}

/// A chunk's row spaces and the questions they need, before the cell
/// answers.
#[derive(Debug, Default)]
pub(super) struct NamingPlan {
    spaces: Vec<Option<u32>>,
    queries: Vec<EntityLabelQuery>,
}

impl NamingPlan {
    /// Place the `client.native` rows of a chunk received at `recv`, and
    /// list one question per (space, entity) pair among the rows `admitted`
    /// says will replay, at the time of the pair's first such row, capped
    /// at [`ENTITY_LABEL_QUERY_CAP`]. `clock` is the session's state from
    /// earlier chunks, updated here from every row.
    pub(super) fn build(
        events: &[TelemetryEvent],
        admitted: &[bool],
        recv: SystemTime,
        clock: &mut SessionClock,
    ) -> Self {
        let recv_ms = millis(recv);
        if let Some(newest) = events
            .iter()
            .filter_map(|ev| native(ev).map(|(ts, _)| ts))
            .max()
        {
            clock.observe(recv_ms, newest);
        }
        let offset = clock.offset_ms();
        let mut plan = Self::default();
        let mut asked = HashSet::new();
        for (row, ev) in events.iter().enumerate() {
            let Some((ts_ms, fields)) = native(ev) else {
                plan.spaces.push(None);
                continue;
            };
            if let Some(space) = fields.get("space_id").and_then(as_u32) {
                clock.space_id = Some(space);
            }
            plan.spaces.push(clock.space_id);
            let (Some(space_id), Some(offset), true) = (
                clock.space_id,
                offset,
                admitted.get(row).copied().unwrap_or(false),
            ) else {
                continue;
            };
            let Some(at) = from_millis(ts_ms.saturating_add(offset).min(recv_ms)) else {
                continue;
            };
            for entity_id in ENTITY_KEYS
                .iter()
                .filter_map(|k| fields.get(*k).and_then(entity_id))
            {
                if plan.queries.len() >= ENTITY_LABEL_QUERY_CAP {
                    break;
                }
                if asked.insert((space_id, entity_id)) {
                    plan.queries.push(EntityLabelQuery {
                        space_id,
                        entity_id,
                        at,
                    });
                }
            }
        }
        plan
    }

    /// The questions the cell will be asked, in order.
    #[cfg(test)]
    pub(super) fn queries(&self) -> &[EntityLabelQuery] {
        &self.queries
    }

    /// The plan with the cell's answers: `labels[i]` answers `queries[i]`.
    fn answered(self, labels: EntityLabelsReply) -> EntityLabels {
        let labels = self
            .queries
            .into_iter()
            .zip(labels)
            .filter_map(|(q, l)| Some(((q.space_id, q.entity_id), l?)))
            .collect();
        EntityLabels {
            spaces: self.spaces,
            labels,
        }
    }
}

/// Plan a chunk for session `sid` of install `install_id` against its
/// stored clock, and resolve the rows `admitted` marks through `link`.
pub(super) async fn name_chunk(
    sid: &str,
    install_id: &str,
    events: &[TelemetryEvent],
    admitted: &[bool],
    recv: SystemTime,
    link: Option<&EntityLabelLink>,
) -> EntityLabels {
    let plan = with_session_clock(sid, install_id, millis(recv), |clock| {
        NamingPlan::build(events, admitted, recv, clock)
    });
    resolve(plan, link).await
}

/// Ask the cell about `plan`'s questions, if the link is free right now.
pub(super) async fn resolve(plan: NamingPlan, link: Option<&EntityLabelLink>) -> EntityLabels {
    let (Some(link), false) = (link, plan.queries.is_empty()) else {
        return plan.answered(Vec::new());
    };
    let asked = plan.queries.len();
    // One request in flight process-wide, whatever the upload concurrency:
    // a chunk that finds it taken replays unnamed rather than queueing.
    let Ok(_permit) = link.in_flight.try_acquire() else {
        return unnamed(plan, "label_link_busy", asked);
    };
    let (reply_tx, reply_rx) = oneshot::channel();
    let request = EntityLabelsRequest {
        queries: plan.queries.clone(),
        reply_tx,
    };
    if link.tx.try_send(request).is_err() {
        return unnamed(plan, "label_channel_full", asked);
    }
    match tokio::time::timeout(CELL_REPLY_TIMEOUT, reply_rx).await {
        Ok(Ok(labels)) => plan.answered(labels),
        // The cell checks `is_closed` before answering, so a request left
        // behind by a timeout costs it nothing.
        Ok(Err(_)) => unnamed(plan, "cell_dropped_reply", asked),
        Err(_) => unnamed(plan, "cell_reply_timeout", asked),
    }
}

fn unnamed(plan: NamingPlan, reason: &'static str, asked: usize) -> EntityLabels {
    tracing::debug!(
        target: "launcher.ingest",
        event = "entity_labels_unavailable",
        reason,
        asked,
        "client rows replayed without entity names"
    );
    plan.answered(Vec::new())
}

/// `(ts_ms, fields)` of a `client.native` row; `None` for other rows.
fn native(ev: &TelemetryEvent) -> Option<(i64, &Map<String, Value>)> {
    match ev {
        TelemetryEvent::ClientNative(e) => Some((e.ts_ms, &e.fields)),
        _ => None,
    }
}

fn as_u32(v: &Value) -> Option<u32> {
    v.as_u64().and_then(|n| u32::try_from(n).ok())
}

/// An entity ID field: a positive number that fits the slot type. The DLL
/// writes 0 or a negative ID for "no entity".
pub(super) fn entity_id(v: &Value) -> Option<u32> {
    as_u32(v).filter(|&n| n != 0)
}

fn millis(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn from_millis(ms: i64) -> Option<SystemTime> {
    let ms = u64::try_from(ms).ok()?;
    UNIX_EPOCH.checked_add(Duration::from_millis(ms))
}
