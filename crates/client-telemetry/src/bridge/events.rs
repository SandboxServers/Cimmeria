//! `client_events_read` — a local ring of client-side events the
//! supervisor drains on demand (issue #686 scope 5).
//!
//! The ring is a low-latency, loopback-only mirror of what the DLL also
//! ships to SigNoz through the existing telemetry queue. It collects:
//!
//! - **hook hits** — from dynamic logging hooks ([`super::dynamic_hooks`]);
//! - **Lua prints** — captured `print` output from `lua_eval`
//!   ([`super::lua_eval`]);
//! - **Mercury dispatch events** — pushed best-effort from the existing
//!   `mercury_dispatch` detour under the `lab-bridge` feature.
//!
//! CEGUI log lines are a documented gap: the ring accepts them via
//! [`push`], but the CEGUI log tee is not yet wired to it (a follow-up).
//!
//! # Threading
//!
//! Producers run on **arbitrary game threads** (a hook can fire on the
//! network thread; Mercury dispatch is on the net thread). [`push`] is a
//! bounded-channel `try_send` — never blocks, drops + counts on a full
//! ring, exactly like the telemetry producer. The consumer
//! ([`drain`]) runs on the main thread from the dispatch drain.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use serde::Serialize;
use serde_json::Value;

use super::dispatch::{RpcResponse, INVALID_PARAMS};

/// Local ring capacity. Generous — the supervisor drains frequently and
/// events are small. Overflow drops the newest and bumps [`DROPPED`].
pub const RING_CAPACITY: usize = 4096;

/// Default max events returned by one `events_read` when unspecified.
pub const DEFAULT_DRAIN: usize = 512;

/// One local event.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LabEvent {
    /// Event kind, e.g. `hook.hit`, `lua.print`, `mercury.dispatch`.
    pub kind: String,
    /// Client wall-clock time, ms since epoch.
    pub ts_ms: i64,
    /// Event-specific fields.
    pub fields: Value,
}

/// Count of events dropped because the ring was full since the last
/// drain reset. Reported by `events_read` so the agent knows it missed
/// some.
static DROPPED: AtomicU64 = AtomicU64::new(0);

/// The ring, created on first use.
static RING: OnceLock<(Sender<LabEvent>, Receiver<LabEvent>)> = OnceLock::new();

fn ring() -> &'static (Sender<LabEvent>, Receiver<LabEvent>) {
    RING.get_or_init(|| bounded(RING_CAPACITY))
}

/// Push an event onto the local ring. Best-effort and non-blocking —
/// safe to call from any thread and any hook detour. Drops + counts on
/// a full ring rather than blocking a game thread.
pub fn push(kind: impl Into<String>, ts_ms: i64, fields: Value) {
    let ev = LabEvent {
        kind: kind.into(),
        ts_ms,
        fields,
    };
    match ring().0.try_send(ev) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Drain up to `max` events from the ring, oldest first. Returns the
/// events and the dropped-since-last-drain count (reset to 0 here).
///
/// **Main thread only** (called from the dispatch drain).
pub fn drain(max: usize) -> (Vec<LabEvent>, u64) {
    let rx = &ring().1;
    let mut out = Vec::new();
    while out.len() < max {
        match rx.try_recv() {
            Ok(ev) => out.push(ev),
            Err(_) => break,
        }
    }
    let dropped = DROPPED.swap(0, Ordering::Relaxed);
    (out, dropped)
}

/// Params for `events_read`.
#[derive(Debug, serde::Deserialize)]
struct EventsReadParams {
    #[serde(default)]
    max: Option<usize>,
}

/// Route an `events_read` request.
pub fn dispatch_read(id: Value, params: &Value) -> RpcResponse {
    let p: EventsReadParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("events_read params: {e}"))
        }
    };
    let max = p.max.unwrap_or(DEFAULT_DRAIN).min(RING_CAPACITY);
    let (events, dropped) = drain(max);
    RpcResponse::ok(
        id,
        serde_json::json!({
            "returned": events.len(),
            "dropped": dropped,
            "events": events,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Push then drain preserves order and clears the ring. (Runs
    /// against the process-global ring; drain-to-empty first so the
    /// assertion is deterministic under any test ordering.)
    #[test]
    fn push_then_drain_round_trips_in_order() {
        let _ = drain(RING_CAPACITY); // clear
        push("hook.hit", 1, json!({ "a": 1 }));
        push("lua.print", 2, json!({ "line": "hi" }));
        let (evs, _dropped) = drain(RING_CAPACITY);
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].kind, "hook.hit");
        assert_eq!(evs[1].kind, "lua.print");
        assert_eq!(evs[1].fields["line"], "hi");
        // Ring is now empty.
        assert_eq!(drain(RING_CAPACITY).0.len(), 0);
    }

    /// `max` bounds the drain; the rest stays for the next read.
    #[test]
    fn drain_respects_max() {
        let _ = drain(RING_CAPACITY);
        for i in 0..5 {
            push("k", i, json!({}));
        }
        let (first, _) = drain(2);
        assert_eq!(first.len(), 2);
        let (rest, _) = drain(RING_CAPACITY);
        assert_eq!(rest.len(), 3);
    }

    #[test]
    fn dispatch_read_reports_shape() {
        let _ = drain(RING_CAPACITY);
        push("mercury.dispatch", 7, json!({ "msg_id": 42 }));
        let r = dispatch_read(json!(1), &json!({ "max": 10 }));
        assert_eq!(r.id, json!(1));
        let result = r.result.expect("ok");
        assert_eq!(result["returned"], 1);
        assert_eq!(result["events"][0]["kind"], "mercury.dispatch");
    }
}
