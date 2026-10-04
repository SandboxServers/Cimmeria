//! Naming the entity IDs in replayed client rows (NT-40).
//!
//! An entity ID is a recycled slot, and a client row reaches the server
//! seconds to minutes after the client wrote it, so the ID is named by
//! asking the cell who held the slot *when the row was written*:
//! `SpaceManager::entity_label_at(space_id, entity_id, at)`. Three things
//! make that question answerable from a chunk of rows.
//!
//! - **The time, on the server clock.** Lifetimes are stamped with the
//!   server's wall clock and the client's clock is its own, so a row's
//!   `ts_ms` is never passed raw. Each session keeps a clock offset: for
//!   every chunk, the server's receive time minus the newest `ts_ms` in it,
//!   and the session keeps the smallest offset seen. An observed offset is
//!   the true clock difference plus that chunk's upload delay (buffering,
//!   retries, the network), never less, so the smallest one is the best
//!   estimate and only improves. The receive time alone would place every
//!   row of a chunk at the moment of upload, after the destroys and slot
//!   reuses that happened while the chunk sat in the DLL's queue; a raw
//!   client time would let skew (or the client) pick the occupant. A row's
//!   server time is `ts_ms + offset`, capped at the receive time. The rows
//!   of a session's first chunks may land a little late, by at most the
//!   smallest upload delay seen so far.
//! - **The space.** Most rows carry no `space_id`. The client is in one
//!   space at a time, and every entity it creates or brings into view
//!   reports the space (`client.entity.create`, `.enter`,
//!   `.entered_world`), so each row is placed in the last space the
//!   session reported, carried from chunk to chunk. A row before the first
//!   report stays unnamed.
//! - **The cell.** The cell loop owns the `SpaceManager`, and the upload
//!   routes are mounted on the admin listener and the public login port
//!   with no router state, so the ingest reaches the cell through one
//!   process-wide sender, set at boot ([`connect_entity_labels`]). Each
//!   chunk is one `BaseToCellMsg::EntityLabelsAt` round trip, deduplicated
//!   and capped, answered between ticks. No sender, a full channel, or no
//!   reply within [`CELL_REPLY_TIMEOUT`] replays the chunk unnamed.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use tokio::sync::{mpsc, oneshot};

use cimmeria_services::cell::messages::{
    BaseToCellMsg, EntityLabelQuery, EntityLabelsReply, ENTITY_LABEL_QUERY_CAP,
};

use super::dto::TelemetryEvent;

/// The `client.native` keys that hold an entity ID. Each pairs with the
/// same prefix ending in `_name` (Rule 6).
pub(super) const ENTITY_KEYS: [&str; 4] = ["entity_id", "target_id", "source_id", "pet_id"];

/// How long a chunk waits for the cell's answer before replaying unnamed.
/// The cell answers between ticks, well inside this.
pub(super) const CELL_REPLY_TIMEOUT: Duration = Duration::from_millis(500);

/// Sessions whose clock and space are remembered. The least recently seen
/// is forgotten first; a forgotten session starts over, and its next rows
/// stay unnamed until it reports a space again.
const MAX_SESSIONS: usize = 1024;

static CELL: OnceLock<mpsc::Sender<BaseToCellMsg>> = OnceLock::new();

/// Give the ingest the cell's channel, so replayed rows name their entity
/// IDs. Called once at boot, after the cell starts; later calls are
/// ignored. Until it is called, rows replay without entity names.
pub fn connect_entity_labels(cell_tx: mpsc::Sender<BaseToCellMsg>) {
    let _ = CELL.set(cell_tx);
}

/// The channel [`connect_entity_labels`] set, if any.
pub(super) fn cell() -> Option<&'static mpsc::Sender<BaseToCellMsg>> {
    CELL.get()
}

/// What a session's earlier chunks established.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SessionClock {
    /// Server minus client clock, in ms: the smallest offset observed.
    pub(super) offset_ms: Option<i64>,
    /// The last space the client reported.
    pub(super) space_id: Option<u32>,
    /// Server time of the last chunk, in ms, for eviction.
    pub(super) last_seen_ms: i64,
}

static CLOCKS: Mutex<Option<HashMap<String, SessionClock>>> = Mutex::new(None);

/// Run `f` on `sid`'s clock in the process-wide table, creating it if new.
/// Poisoning is ignored: the clock only names rows and must never take the
/// ingest down.
fn with_session_clock<R>(sid: &str, now_ms: i64, f: impl FnOnce(&mut SessionClock) -> R) -> R {
    let mut guard = CLOCKS.lock().unwrap_or_else(PoisonError::into_inner);
    let table = guard.get_or_insert_with(HashMap::new);
    if !table.contains_key(sid) && table.len() >= MAX_SESSIONS {
        let oldest = table
            .iter()
            .min_by_key(|(_, c)| c.last_seen_ms)
            .map(|(k, _)| k.clone());
        if let Some(oldest) = oldest {
            table.remove(&oldest);
        }
    }
    let clock = table.entry(sid.to_string()).or_default();
    clock.last_seen_ms = now_ms;
    f(clock)
}

/// Where and when one row happened, on the server's terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RowPlace {
    space_id: u32,
    at: SystemTime,
}

/// The entity names for one chunk's rows, resolved by the cell.
#[derive(Debug, Default)]
pub(super) struct EntityLabels {
    /// Per row (by index in the chunk): its place, when it has one.
    places: Vec<Option<RowPlace>>,
    /// Each distinct (space, entity, time) asked about, and its answer.
    labels: HashMap<EntityLabelQuery, &'static str>,
}

impl EntityLabels {
    /// No names: what a replay without the cell uses.
    pub(super) fn none() -> Self {
        Self::default()
    }

    /// The label of `entity_id` as row `row` saw it, if the cell knew it.
    pub(super) fn label(&self, row: usize, entity_id: u32) -> Option<&'static str> {
        let place = (*self.places.get(row)?)?;
        let query = EntityLabelQuery {
            space_id: place.space_id,
            entity_id,
            at: place.at,
        };
        self.labels.get(&query).copied()
    }
}

/// A chunk's places and the queries they need, before the cell answers.
#[derive(Debug, Default)]
pub(super) struct NamingPlan {
    places: Vec<Option<RowPlace>>,
    queries: Vec<EntityLabelQuery>,
}

impl NamingPlan {
    /// Place every `client.native` row of a chunk received at `recv`, and
    /// list the entity IDs to ask about, deduplicated and capped. `clock` is
    /// the session's state from earlier chunks, updated here.
    pub(super) fn build(
        events: &[TelemetryEvent],
        recv: SystemTime,
        clock: &mut SessionClock,
    ) -> Self {
        let recv_ms = millis(recv);
        let newest = events
            .iter()
            .filter_map(|ev| native(ev).map(|(ts, _)| ts))
            .max();
        if let Some(newest) = newest {
            let observed = recv_ms.saturating_sub(newest);
            clock.offset_ms = Some(clock.offset_ms.map_or(observed, |o| o.min(observed)));
        }
        let mut plan = Self::default();
        let mut seen = HashSet::new();
        for ev in events {
            let Some((ts_ms, fields)) = native(ev) else {
                plan.places.push(None);
                continue;
            };
            if let Some(space) = fields.get("space_id").and_then(as_u32) {
                clock.space_id = Some(space);
            }
            let at_ms = clock
                .offset_ms
                .map(|o| ts_ms.saturating_add(o).min(recv_ms));
            let place = match (clock.space_id, at_ms.and_then(from_millis)) {
                (Some(space_id), Some(at)) => Some(RowPlace { space_id, at }),
                _ => None,
            };
            plan.places.push(place);
            let Some(place) = place else { continue };
            for id in ENTITY_KEYS
                .iter()
                .filter_map(|k| fields.get(*k).and_then(entity_id))
            {
                let query = EntityLabelQuery {
                    space_id: place.space_id,
                    entity_id: id,
                    at: place.at,
                };
                if plan.queries.len() < ENTITY_LABEL_QUERY_CAP && seen.insert(query) {
                    plan.queries.push(query);
                }
            }
        }
        plan
    }

    /// The queries the cell will be asked, in order.
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
            .filter_map(|(q, l)| Some((q, l?)))
            .collect();
        EntityLabels {
            places: self.places,
            labels,
        }
    }
}

/// Plan a chunk for session `sid` against its stored clock, and resolve it
/// through `cell`. Rows replay unnamed when the cell can't answer.
pub(super) async fn name_chunk(
    sid: &str,
    events: &[TelemetryEvent],
    recv: SystemTime,
    cell: Option<&mpsc::Sender<BaseToCellMsg>>,
) -> EntityLabels {
    let plan = with_session_clock(sid, millis(recv), |clock| {
        NamingPlan::build(events, recv, clock)
    });
    resolve(plan, cell).await
}

/// Ask the cell about `plan`'s queries.
pub(super) async fn resolve(
    plan: NamingPlan,
    cell: Option<&mpsc::Sender<BaseToCellMsg>>,
) -> EntityLabels {
    let (Some(cell), false) = (cell, plan.queries.is_empty()) else {
        return plan.answered(Vec::new());
    };
    let (reply_tx, reply_rx) = oneshot::channel();
    let queries = plan.queries.clone();
    let asked = queries.len();
    // try_send: a cell backed up with game traffic is not delayed further by
    // telemetry; the chunk replays unnamed instead.
    let reason = match cell.try_send(BaseToCellMsg::EntityLabelsAt { queries, reply_tx }) {
        Err(_) => "cell_channel_unavailable",
        Ok(()) => match tokio::time::timeout(CELL_REPLY_TIMEOUT, reply_rx).await {
            Ok(Ok(labels)) => return plan.answered(labels),
            Ok(Err(_)) => "cell_dropped_reply",
            Err(_) => "cell_reply_timeout",
        },
    };
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
