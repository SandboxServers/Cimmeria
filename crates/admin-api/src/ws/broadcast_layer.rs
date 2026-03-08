//! Custom tracing layer that broadcasts log entries over a tokio channel.
//!
//! Created once in `main()` and added to the tracing subscriber. All log
//! events are serialised into [`LogEntry`] structs and sent to connected
//! WebSocket clients via a `broadcast::Sender`.

use serde::Serialize;
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// A single log entry forwarded to WebSocket clients.
#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Log level: TRACE, DEBUG, INFO, WARN, ERROR.
    pub level: String,
    /// Module target (e.g. `cimmeria_services::auth`).
    pub target: String,
    /// The log message text.
    pub message: String,
    /// Structured key-value fields from the tracing event.
    pub fields: serde_json::Value,
}

/// Tracing layer that sends every event to a broadcast channel.
///
/// # Important
///
/// This layer must **never** call `tracing::*` macros internally, or it will
/// cause infinite recursion.
pub struct BroadcastLayer {
    tx: broadcast::Sender<LogEntry>,
}

impl BroadcastLayer {
    pub fn new(tx: broadcast::Sender<LogEntry>) -> Self {
        Self { tx }
    }
}

impl<S> Layer<S> for BroadcastLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let entry = LogEntry {
            timestamp_ms: now,
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.message,
            fields: serde_json::Value::Object(visitor.fields),
        };

        // Ignore send errors — means no subscribers are connected.
        let _ = self.tx.send(entry);
    }
}

/// Visitor that extracts the `message` field and all other key-value pairs.
#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: serde_json::Map<String, serde_json::Value>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(format!("{:?}", value)),
            );
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
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
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Bool(value),
        );
    }
}
