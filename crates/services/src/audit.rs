//! Login audit event types and ring buffer.
//!
//! Provides a [`LoginEvent`] struct emitted by auth handlers at every login
//! outcome, a [`LoginEventBuffer`] ring buffer for WebSocket history replay,
//! and an [`emit_login_event`] helper for constructing and broadcasting events.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::broadcast;

/// Maximum number of login events kept in the ring buffer.
const BUFFER_CAPACITY: usize = 100;

/// A single login audit event.
#[derive(Clone, Debug, Serialize)]
pub struct LoginEvent {
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Account name from the login request.
    pub account_name: String,
    /// Account ID (if credentials were valid).
    pub account_id: Option<u32>,
    /// Client IP address.
    pub ip_address: String,
    /// Login phase: `"credential_check"` or `"shard_selection"`.
    pub phase: String,
    /// Outcome: `"success"`, `"invalid_credentials"`, `"account_disabled"`, etc.
    pub outcome: String,
    /// Selected shard name (Phase 2 only).
    pub shard: Option<String>,
    /// Extra detail (error messages, etc.).
    pub detail: Option<String>,
}

/// Thread-safe ring buffer of recent login events.
///
/// Mirrors the [`LogBuffer`](crate::ws::broadcast_layer::LogBuffer) pattern.
#[derive(Clone)]
pub struct LoginEventBuffer {
    inner: Arc<Mutex<VecDeque<LoginEvent>>>,
}

impl LoginEventBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_CAPACITY))),
        }
    }

    /// Push an event, evicting the oldest if at capacity.
    pub fn push(&self, event: LoginEvent) {
        if let Ok(mut buf) = self.inner.lock() {
            if buf.len() >= BUFFER_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(event);
        }
    }

    /// Snapshot all buffered events (oldest first).
    pub fn snapshot(&self) -> Vec<LoginEvent> {
        self.inner
            .lock()
            .map(|buf| buf.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Build and broadcast a [`LoginEvent`].
///
/// Also pushes the event into the ring buffer for late-joining WebSocket
/// clients. Silently ignores send failures (no subscribers connected).
pub fn emit_login_event(
    tx: &broadcast::Sender<LoginEvent>,
    buffer: &LoginEventBuffer,
    account_name: &str,
    account_id: Option<u32>,
    ip_address: &str,
    phase: &str,
    outcome: &str,
    shard: Option<&str>,
    detail: Option<&str>,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let event = LoginEvent {
        timestamp_ms: now,
        account_name: account_name.to_string(),
        account_id,
        ip_address: ip_address.to_string(),
        phase: phase.to_string(),
        outcome: outcome.to_string(),
        shard: shard.map(|s| s.to_string()),
        detail: detail.map(|d| d.to_string()),
    };

    buffer.push(event.clone());
    let _ = tx.send(event);
}
