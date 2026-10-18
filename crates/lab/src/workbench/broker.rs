//! Bounded client/server history with independent named cursors and exact eviction counts.

use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::context::CaptureContext;

const MAX_ROWS_PER_SOURCE: usize = 4096;
const MAX_CURSORS: usize = 64;
const MAX_ROW_BYTES: usize = 64 * 1024;
const HISTORY_MS: u64 = 300_000;
const HISTORY_AGE: Duration = Duration::from_secs(300);

/// The two independently bounded capture feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Client,
    Server,
}

/// Identity under which a cursor may read, including its server session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureKey {
    pub instance: String,
    pub client_pid: u32,
    pub server_session_id: Option<String>,
}

impl From<&CaptureContext> for CaptureKey {
    fn from(context: &CaptureContext) -> Self {
        Self {
            instance: context.instance.clone(),
            client_pid: context.client_pid,
            server_session_id: context.server_session_id.clone(),
        }
    }
}

/// Stable handle for a named subscriber within one broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CursorId(u64);

/// A subscription refusal that callers can surface without changing ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
    CaptureBusy,
    InvalidCursor,
    TooManyCursors,
}

impl CaptureError {
    /// Stable machine-readable refusal code.
    pub fn code(self) -> &'static str {
        match self {
            Self::CaptureBusy => "capture_busy",
            Self::InvalidCursor => "invalid_cursor",
            Self::TooManyCursors => "too_many_cursors",
        }
    }
}

/// A captured JSON row, with its original context and broker sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureRow {
    pub seq: u64,
    pub source: Source,
    pub context: CaptureContext,
    pub row: Value,
}

/// Cloned rows following one cursor and losses since its previous read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadBatch {
    pub rows: Vec<CaptureRow>,
    pub first_seq: Option<u64>,
    pub last_seq: Option<u64>,
    pub dropped: u64,
}

/// Rejected rows since this broker was constructed, grouped by reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectionCounts {
    /// Rows from a different client PID or server session while cursors hold this capture.
    pub identity: u64,
    /// Rows whose serialized JSON exceeds 64 KiB.
    pub oversized: u64,
    /// Rows already outside the 300-second monotonic history window on arrival.
    pub stale: u64,
}

struct ByteLimit(usize);

impl Write for ByteLimit {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_ROW_BYTES.saturating_sub(self.0) {
            return Err(io::Error::other("capture row exceeds byte limit"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl ReadBatch {
    fn empty() -> Self {
        Self {
            rows: Vec::new(),
            first_seq: None,
            last_seq: None,
            dropped: 0,
        }
    }
}

struct StoredRow {
    value: CaptureRow,
    inserted: Instant,
}

struct Cursor {
    name: String,
    after: u64,
    dropped: u64,
}

struct BrokerState {
    key: Option<CaptureKey>,
    next_seq: u64,
    next_cursor_id: u64,
    evicted: u64,
    rejections: RejectionCounts,
    latest_monotonic_ms: u64,
    client: VecDeque<StoredRow>,
    server: VecDeque<StoredRow>,
    cursors: HashMap<CursorId, Cursor>,
}

impl BrokerState {
    fn reset(&mut self, key: CaptureKey) {
        self.key = Some(key);
        self.next_seq = 1;
        self.evicted = 0;
        self.latest_monotonic_ms = 0;
        self.client.clear();
        self.server.clear();
    }

    fn evict(&mut self, source: Source, capacity: usize, now_ms: u64, now: Instant) {
        let queue = match source {
            Source::Client => &mut self.client,
            Source::Server => &mut self.server,
        };
        let mut removed = Vec::new();
        queue.retain(|stored| {
            let too_old = now_ms.saturating_sub(stored.value.context.monotonic_ms) > HISTORY_MS
                || now.duration_since(stored.inserted) > HISTORY_AGE;
            if too_old {
                removed.push(stored.value.seq);
            }
            !too_old
        });
        while queue.len() > capacity {
            removed.push(queue.pop_front().expect("over capacity").value.seq);
        }
        self.evicted += removed.len() as u64;
        for cursor in self.cursors.values_mut() {
            cursor.dropped += removed.iter().filter(|&&seq| seq > cursor.after).count() as u64;
        }
    }

    fn expire(&mut self, capacity: usize, now: Instant) {
        self.evict(Source::Client, capacity, self.latest_monotonic_ms, now);
        self.evict(Source::Server, capacity, self.latest_monotonic_ms, now);
    }
}

/// Pure in-process broker; no tap, bridge, or HTTP ownership lives here.
pub struct CaptureBroker {
    capacity: usize,
    state: Mutex<BrokerState>,
}

impl CaptureBroker {
    /// Construct a broker capped at 4096 rows per source, even if asked for more.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.clamp(1, MAX_ROWS_PER_SOURCE),
            state: Mutex::new(BrokerState {
                key: None,
                next_seq: 1,
                next_cursor_id: 1,
                evicted: 0,
                rejections: RejectionCounts::default(),
                latest_monotonic_ms: 0,
                client: VecDeque::new(),
                server: VecDeque::new(),
                cursors: HashMap::new(),
            }),
        }
    }

    /// Subscribe by name. Repeating a name returns its existing cursor.
    pub fn subscribe(&self, key: CaptureKey, cursor: String) -> Result<CursorId, CaptureError> {
        if cursor.trim().is_empty() || cursor.len() > 128 {
            return Err(CaptureError::InvalidCursor);
        }
        let mut state = self.state.lock().unwrap();
        if state.key.as_ref().is_some_and(|active| active != &key) {
            if !state.cursors.is_empty() {
                return Err(CaptureError::CaptureBusy);
            }
            state.reset(key);
        } else if state.key.is_none() {
            state.key = Some(key);
        }
        if let Some((&id, _)) = state.cursors.iter().find(|(_, c)| c.name == cursor) {
            return Ok(id);
        }
        if state.cursors.len() >= MAX_CURSORS {
            return Err(CaptureError::TooManyCursors);
        }
        let id = CursorId(state.next_cursor_id);
        state.next_cursor_id += 1;
        let dropped = state.evicted;
        state.cursors.insert(
            id,
            Cursor {
                name: cursor,
                after: 0,
                dropped,
            },
        );
        Ok(id)
    }

    /// Append a row only when its identity matches this broker's active capture.
    pub fn push(&self, source: Source, context: &CaptureContext, row: Value) {
        let mut state = self.state.lock().unwrap();
        let key = CaptureKey::from(context);
        if state.key.as_ref().is_some_and(|active| active != &key) {
            if !state.cursors.is_empty() {
                state.rejections.identity += 1;
                return;
            }
            state.reset(key);
        } else if state.key.is_none() {
            state.key = Some(key);
        }
        if serde_json::to_writer(ByteLimit(0), &row).is_err() {
            state.rejections.oversized += 1;
            return;
        }
        let now = Instant::now();
        if state
            .latest_monotonic_ms
            .saturating_sub(context.monotonic_ms)
            > HISTORY_MS
        {
            state.rejections.stale += 1;
            return;
        }
        state.latest_monotonic_ms = state.latest_monotonic_ms.max(context.monotonic_ms);
        state.expire(self.capacity, now);
        let seq = state.next_seq;
        state.next_seq += 1;
        let stored = StoredRow {
            value: CaptureRow {
                seq,
                source,
                context: context.clone(),
                row,
            },
            inserted: now,
        };
        match source {
            Source::Client => state.client.push_back(stored),
            Source::Server => state.server.push_back(stored),
        }
        let latest_ms = state.latest_monotonic_ms;
        state.evict(source, self.capacity, latest_ms, now);
    }

    /// Return lifetime counts for rows refused before entering the history.
    pub fn rejections(&self) -> RejectionCounts {
        self.state.lock().unwrap().rejections
    }

    /// Read a bounded clone of rows after this cursor, advancing it independently.
    pub fn read(&self, id: CursorId, max: usize) -> ReadBatch {
        let mut state = self.state.lock().unwrap();
        state.expire(self.capacity, Instant::now());
        let Some(cursor) = state.cursors.get(&id) else {
            return ReadBatch::empty();
        };
        let after = cursor.after;
        let dropped = cursor.dropped;
        let mut rows: Vec<_> = state
            .client
            .iter()
            .chain(state.server.iter())
            .filter(|stored| stored.value.seq > after)
            .map(|stored| stored.value.clone())
            .collect();
        rows.sort_unstable_by_key(|row| row.seq);
        rows.truncate(max.min(self.capacity * 2));
        let first_seq = rows.first().map(|row| row.seq);
        let last_seq = rows.last().map(|row| row.seq);
        let cursor = state.cursors.get_mut(&id).expect("cursor held by lock");
        if let Some(seq) = last_seq {
            cursor.after = seq;
        }
        cursor.dropped = 0;
        ReadBatch {
            rows,
            first_seq,
            last_seq,
            dropped,
        }
    }

    /// Remove a subscriber; a later capture identity can then take ownership.
    pub fn release(&self, id: CursorId) {
        self.state.lock().unwrap().cursors.remove(&id);
    }
}

#[cfg(test)]
#[path = "broker_tests.rs"]
mod tests;
