//! Custom tracing layer that broadcasts log entries over a tokio channel.
//!
//! Created once in `main()` and added to the tracing subscriber. All log
//! events are serialised into [`LogEntry`] structs and sent to connected
//! WebSocket clients via a `broadcast::Sender`.
//!
//! A [`LogBuffer`] ring buffer retains the most recent entries so that new
//! WebSocket clients receive history on connect.
//!
//! # Memory bounds
//!
//! Some events carry text the server does not control, so entries and the
//! ring are bounded:
//!
//! - Each entry is truncated while it is built, before it is cloned or sent:
//!   the message to [`MAX_MESSAGE_BYTES`], every other string to
//!   [`MAX_FIELD_BYTES`], and the whole entry to [`MAX_ENTRY_BYTES`]. A cut
//!   lands on a UTF-8 char boundary and is followed by [`TRUNCATION_MARKER`].
//! - The ring holds at most [`BUFFER_CAPACITY`] entries and at most
//!   [`RING_BYTE_BUDGET`] bytes, evicting the oldest entries first.

use std::collections::VecDeque;
use std::fmt::{self, Write as _};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Maximum number of entries kept in the ring buffer for new clients.
pub const BUFFER_CAPACITY: usize = 500;

/// Total byte budget of the ring buffer (see [`LogEntry::approx_bytes`]).
/// 500 entries of ~4 KiB fit; larger entries push older ones out sooner.
pub const RING_BYTE_BUDGET: usize = 2 * 1024 * 1024;

/// Longest message kept, in bytes, before [`TRUNCATION_MARKER`].
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024;

/// Longest value kept for any other string (a field value, the target).
pub const MAX_FIELD_BYTES: usize = 1024;

/// Budget for all the text in one entry (message, field names and values).
/// Fields recorded after the budget runs out keep only the marker.
pub const MAX_ENTRY_BYTES: usize = 16 * 1024;

/// Appended to every string that was cut.
pub const TRUNCATION_MARKER: &str = "...[truncated]";

/// A single log entry forwarded to WebSocket clients.
#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Log level: TRACE, DEBUG, INFO, WARN, ERROR.
    pub level: String,
    /// Module target (e.g. `cimmeria_auth::auth`).
    pub target: String,
    /// The log message text.
    pub message: String,
    /// Structured key-value fields from the tracing event.
    pub fields: serde_json::Value,
}

impl LogEntry {
    /// Approximate memory held by this entry: the struct plus the allocated
    /// capacity of the text it owns. Non-string JSON values count as 8 bytes.
    /// It is the unit of the ring's byte budget; map node overhead is not
    /// counted, so it is not an allocator-exact figure.
    pub fn approx_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.level.capacity()
            + self.target.capacity()
            + self.message.capacity()
            + json_bytes(&self.fields)
    }
}

fn json_bytes(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::String(s) => s.capacity(),
        serde_json::Value::Array(items) => items.iter().map(json_bytes).sum(),
        serde_json::Value::Object(map) => {
            map.iter().map(|(k, v)| k.capacity() + json_bytes(v)).sum()
        }
        _ => 8,
    }
}

/// Ring contents with the running byte total, so a push never rescans.
struct Ring {
    entries: VecDeque<(LogEntry, usize)>,
    bytes: usize,
}

/// Thread-safe ring buffer of recent log entries.
///
/// Shared between the tracing layer (writer) and WebSocket handlers (reader).
/// Uses a `std::sync::Mutex` (not tokio) because the tracing layer runs in
/// a synchronous context.
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<Ring>>,
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Ring {
                entries: VecDeque::new(),
                bytes: 0,
            })),
        }
    }

    /// Push an entry, evicting the oldest until both the entry count and the
    /// byte budget hold.
    fn push(&self, entry: LogEntry) {
        let size = entry.approx_bytes();
        if let Ok(mut ring) = self.inner.lock() {
            while !ring.entries.is_empty()
                && (ring.entries.len() >= BUFFER_CAPACITY || ring.bytes + size > RING_BYTE_BUDGET)
            {
                if let Some((_, evicted)) = ring.entries.pop_front() {
                    ring.bytes -= evicted;
                }
            }
            ring.bytes += size;
            ring.entries.push_back((entry, size));
        }
    }

    /// Snapshot all buffered entries (oldest first).
    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.inner
            .lock()
            .map(|ring| ring.entries.iter().map(|(e, _)| e.clone()).collect())
            .unwrap_or_default()
    }

    /// The ring's running byte total.
    #[cfg(test)]
    fn total_bytes(&self) -> usize {
        self.inner.lock().map(|ring| ring.bytes).unwrap_or_default()
    }
}

/// Tracing layer that sends every event to a broadcast channel and ring buffer.
///
/// # Important
///
/// This layer must **never** call `tracing::*` macros internally, or it will
/// cause infinite recursion.
pub struct BroadcastLayer {
    tx: broadcast::Sender<LogEntry>,
    buffer: LogBuffer,
}

impl BroadcastLayer {
    pub fn new(tx: broadcast::Sender<LogEntry>, buffer: LogBuffer) -> Self {
        Self { tx, buffer }
    }
}

impl<S> Layer<S> for BroadcastLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let entry = build_entry(event);

        // Only clone for the channel when a WebSocket client is listening;
        // the ring (also read by the lab's log tail) keeps the original.
        if self.tx.receiver_count() > 0 {
            // Ignore errors (the last receiver may have just gone).
            let _ = self.tx.send(entry.clone());
        }
        self.buffer.push(entry);
    }
}

/// Build a bounded [`LogEntry`] from a tracing event.
fn build_entry(event: &tracing::Event<'_>) -> LogEntry {
    let metadata = event.metadata();

    let mut visitor = FieldVisitor::default();
    event.record(&mut visitor);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    LogEntry {
        timestamp_ms: now,
        level: metadata.level().to_string(),
        target: truncate_str(metadata.target(), MAX_FIELD_BYTES),
        message: visitor.message,
        fields: serde_json::Value::Object(visitor.fields),
    }
}

/// Largest index `<= max` that is a char boundary of `s`.
fn floor_char_boundary(s: &str, max: usize) -> usize {
    if max >= s.len() {
        return s.len();
    }
    let mut cut = max;
    while !s.is_char_boundary(cut) {
        cut -= 1;
    }
    cut
}

/// Copy at most `limit` bytes of `s`, adding the marker when it was cut.
fn truncate_str(s: &str, limit: usize) -> String {
    if s.len() <= limit {
        return s.to_owned();
    }
    let cut = floor_char_boundary(s, limit);
    let mut out = String::with_capacity(cut + TRUNCATION_MARKER.len());
    out.push_str(&s[..cut]);
    out.push_str(TRUNCATION_MARKER);
    out
}

/// `fmt::Write` sink that stops at `limit` bytes. Once full it returns
/// `fmt::Error`, which aborts the `Debug` formatting, so an oversized value
/// is never formatted in full.
struct BoundedWriter {
    buf: String,
    limit: usize,
    truncated: bool,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            buf: String::new(),
            limit,
            truncated: false,
        }
    }

    fn finish(mut self) -> String {
        if self.truncated {
            self.buf.push_str(TRUNCATION_MARKER);
        }
        // The buffer grew by doubling, so it can hold up to twice its length.
        // Hand back a tight allocation: the ring budgets by capacity, and
        // slack here would let the budget hold half the entries it could.
        self.buf.shrink_to_fit();
        self.buf
    }
}

impl fmt::Write for BoundedWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.truncated {
            return Err(fmt::Error);
        }
        let room = self.limit - self.buf.len();
        if s.len() <= room {
            self.buf.push_str(s);
            return Ok(());
        }
        let cut = floor_char_boundary(s, room);
        self.buf.push_str(&s[..cut]);
        self.truncated = true;
        Err(fmt::Error)
    }
}

/// Format `value` with `{:?}`, keeping at most `limit` bytes.
fn bounded_debug(value: &dyn fmt::Debug, limit: usize) -> String {
    let mut w = BoundedWriter::new(limit);
    // An error here only means the writer filled up.
    let _ = write!(w, "{value:?}");
    w.finish()
}

/// Visitor that extracts the `message` field and all other key-value pairs,
/// truncating text as it goes against a per-entry budget.
struct FieldVisitor {
    message: String,
    fields: serde_json::Map<String, serde_json::Value>,
    /// Text bytes still allowed in this entry.
    remaining: usize,
}

impl Default for FieldVisitor {
    fn default() -> Self {
        Self {
            message: String::new(),
            fields: serde_json::Map::new(),
            remaining: MAX_ENTRY_BYTES,
        }
    }
}

impl FieldVisitor {
    /// Byte limit for the next string: its own cap, within what the entry
    /// has left once the field name is paid for.
    fn limit_for(&mut self, field: &Field) -> usize {
        if field.name() == "message" {
            MAX_MESSAGE_BYTES.min(self.remaining)
        } else {
            self.remaining = self.remaining.saturating_sub(field.name().len());
            MAX_FIELD_BYTES.min(self.remaining)
        }
    }

    fn store(&mut self, field: &Field, text: String) {
        self.remaining = self.remaining.saturating_sub(text.len());
        if field.name() == "message" {
            self.message = text;
        } else {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::String(text));
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let limit = self.limit_for(field);
        let text = bounded_debug(value, limit);
        self.store(field, text);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let limit = self.limit_for(field);
        let text = truncate_str(value, limit);
        self.store(field, text);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }
}

#[cfg(test)]
mod tests;
