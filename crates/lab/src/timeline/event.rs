//! The merged-timeline event model and the merge/order/window step
//! (ADR §5).
//!
//! Every event is projected onto **one axis: the server clock**. Server
//! packet-tap rows are already on it. Client events arrive on the dev-box
//! clock and are shifted by the estimated offset (see [`super::clock`])
//! before they are merged. The original client timestamp is preserved so
//! a reader can see the raw value and the correction that was applied.
//!
//! The merge itself is deliberately dumb: normalize, concatenate, stable
//! sort by the server-clock key, then clip to the window. Keeping it a
//! pure function over owned inputs is what makes it testable without a
//! live client or server.

use serde::Serialize;
use serde_json::Value;

use super::clock::ClockOffset;

/// Which ring an event came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    /// Local client event (bridge heartbeat today; hook hits / Lua prints
    /// / Mercury dispatch once the #686 event ring lands).
    Client,
    /// Server packet-tap row (decoded Mercury message).
    Server,
}

/// One event on the merged timeline.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub source: EventSource,
    /// The merge key: milliseconds on the **server** clock.
    pub server_ts_ms: i64,
    /// The original client-clock timestamp, when the event came from the
    /// client. `None` for server rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ts_ms: Option<i64>,
    /// Coarse category, e.g. `"bridge.heartbeat"`, `"mercury.<name>"`.
    pub kind: String,
    /// Packet direction for server rows (`"c2s"` / `"s2c"`); `None`
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// The untouched source payload, for drill-down.
    pub detail: Value,
}

impl TimelineEvent {
    /// A server-sourced event — already on the server clock.
    pub fn server(
        ts_ms: i64,
        kind: impl Into<String>,
        direction: Option<String>,
        detail: Value,
    ) -> Self {
        Self {
            source: EventSource::Server,
            server_ts_ms: ts_ms,
            client_ts_ms: None,
            kind: kind.into(),
            direction,
            detail,
        }
    }

    /// A client-sourced event, still on the client clock. Call
    /// [`TimelineEvent::projected`] to shift it onto the server axis.
    pub fn client(client_ts_ms: i64, kind: impl Into<String>, detail: Value) -> Self {
        Self {
            source: EventSource::Client,
            // Placeholder until projected; equals the client clock so an
            // un-projected event still sorts sanely.
            server_ts_ms: client_ts_ms,
            client_ts_ms: Some(client_ts_ms),
            kind: kind.into(),
            direction: None,
            detail,
        }
    }

    /// Shift a client event onto the server clock by the offset. A no-op
    /// for server events (they are already there).
    fn projected(mut self, offset: &ClockOffset) -> Self {
        if self.source == EventSource::Client {
            if let Some(client_ts) = self.client_ts_ms {
                self.server_ts_ms = client_ts + offset.offset_ms;
            }
        }
        self
    }
}

/// Inclusive time window on the **server** clock.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub start_ms: i64,
    pub end_ms: i64,
}

impl Window {
    fn contains(&self, ts: i64) -> bool {
        ts >= self.start_ms && ts <= self.end_ms
    }
}

/// Merge client and server events into one time-ordered slice.
///
/// - Client events are projected onto the server clock via `offset`.
/// - Everything is stable-sorted by `server_ts_ms`; ties keep input
///   order (server rows are passed before client events, so on an exact
///   tie the wire packet reads before the local observation, which is the
///   causal order you want when a packet arrival triggers a client log).
/// - `window`, when present, clips the result inclusively.
pub fn merge(
    server_events: Vec<TimelineEvent>,
    client_events: Vec<TimelineEvent>,
    offset: &ClockOffset,
    window: Option<Window>,
) -> Vec<TimelineEvent> {
    let mut merged: Vec<TimelineEvent> =
        Vec::with_capacity(server_events.len() + client_events.len());
    // Server first so ties resolve server-before-client (causal order).
    merged.extend(server_events);
    merged.extend(client_events.into_iter().map(|e| e.projected(offset)));

    // Stable sort keeps the server-before-client tie order above.
    merged.sort_by_key(|e| e.server_ts_ms);

    if let Some(w) = window {
        merged.retain(|e| w.contains(e.server_ts_ms));
    }
    merged
}

/// Compute a lookback window ending at the newest event across both
/// rings. Returns `None` when there are no events at all (empty
/// timeline), so the caller can short-circuit.
pub fn lookback_window(
    server_events: &[TimelineEvent],
    client_events: &[TimelineEvent],
    offset: &ClockOffset,
    lookback_ms: i64,
) -> Option<Window> {
    let latest_server = server_events.iter().map(|e| e.server_ts_ms).max();
    // Client events are still on the client clock here; project their max.
    let latest_client = client_events
        .iter()
        .filter_map(|e| e.client_ts_ms)
        .max()
        .map(|c| c + offset.offset_ms);
    let end = latest_server.into_iter().chain(latest_client).max()?;
    Some(Window {
        start_ms: end - lookback_ms,
        end_ms: end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn srv(ts: i64, kind: &str) -> TimelineEvent {
        TimelineEvent::server(ts, kind, Some("s2c".into()), json!({}))
    }
    fn cli(ts: i64, kind: &str) -> TimelineEvent {
        TimelineEvent::client(ts, kind, json!({}))
    }

    #[test]
    fn out_of_order_inputs_come_back_sorted() {
        let offset = ClockOffset::unestimated();
        let server = vec![srv(300, "a"), srv(100, "b")];
        let client = vec![cli(200, "c"), cli(50, "d")];
        let merged = merge(server, client, &offset, None);
        let order: Vec<i64> = merged.iter().map(|e| e.server_ts_ms).collect();
        assert_eq!(order, vec![50, 100, 200, 300]);
    }

    #[test]
    fn offset_is_applied_to_client_events_only() {
        // Server is 1000ms ahead of the client clock.
        let offset = ClockOffset {
            offset_ms: 1000,
            rtt_ms: 10,
            sample_count: 1,
            method: "test".into(),
        };
        let server = vec![srv(2000, "packet")];
        let client = vec![cli(1000, "heartbeat")]; // client-clock 1000 → server 2000
        let merged = merge(server, client, &offset, None);
        // Both now at 2000; server before client on the tie.
        assert_eq!(merged[0].source, EventSource::Server);
        assert_eq!(merged[1].source, EventSource::Client);
        assert_eq!(merged[1].server_ts_ms, 2000);
        // Original client timestamp preserved.
        assert_eq!(merged[1].client_ts_ms, Some(1000));
    }

    #[test]
    fn window_clips_inclusively() {
        let offset = ClockOffset::unestimated();
        let server = vec![
            srv(90, "before"),
            srv(100, "edge"),
            srv(150, "in"),
            srv(201, "after"),
        ];
        let w = Window {
            start_ms: 100,
            end_ms: 200,
        };
        let merged = merge(server, vec![], &offset, Some(w));
        let kinds: Vec<&str> = merged.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, vec!["edge", "in"]);
    }

    #[test]
    fn empty_inputs_yield_empty_window_and_merge() {
        let offset = ClockOffset::unestimated();
        assert!(lookback_window(&[], &[], &offset, 60_000).is_none());
        assert!(merge(vec![], vec![], &offset, None).is_empty());
    }

    #[test]
    fn lookback_window_spans_offset_projected_client_max() {
        let offset = ClockOffset {
            offset_ms: 5000,
            rtt_ms: 5,
            sample_count: 1,
            method: "test".into(),
        };
        // Client max is 1000 (client clock) → 6000 server. Server max 4000.
        let window = lookback_window(&[srv(4000, "s")], &[cli(1000, "c")], &offset, 2000).unwrap();
        assert_eq!(window.end_ms, 6000);
        assert_eq!(window.start_ms, 4000);
    }
}
