//! Local client-event sources for the merged timeline.
//!
//! ## What exists today (this branch)
//!
//! The only inbound client signal the bridge exposes right now is the
//! Tick-drain **heartbeat** counter (`crates/lab/src/client.rs`). The
//! supervisor already polls it (status + watchdog); the timeline just
//! needs a short history of those polls, so [`HeartbeatSample`]s are kept
//! in a bounded ring on the supervisor and drained here into
//! [`TimelineEvent`]s. Each carries the client-generated tick count and
//! the dev-box wall-clock time the supervisor observed it — and because
//! the client and the supervisor share the dev box, that observation time
//! *is* the client clock (to within the loopback RTT), which is exactly
//! the `ts_ms` the merge step expects on a client event.
//!
//! ## What plugs in later (#686, stopped)
//!
//! `client_events_read` was to drain the bridge's full local event ring —
//! hook hits, Lua prints, CEGUI log lines, and Mercury dispatch events
//! (ADR §3.3). That work was stopped by the owner. When it lands, its
//! rows arrive already stamped with the client's own `ts_ms`, so they
//! feed [`TimelineEvent::client`] the same way heartbeats do. The seam is
//! [`drain_event_ring`] below: wire the bridge `events_read` call there
//! and the merge/offset machinery is unchanged.

use serde_json::json;

use super::event::TimelineEvent;

/// One observed heartbeat: the client's Tick-drain counter and the
/// dev-box wall-clock time (ms since epoch) at observation.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatSample {
    pub tick_count: u64,
    pub observed_ms: i64,
}

/// Convert the supervisor's heartbeat ring into client timeline events.
///
/// Consecutive identical tick counts (the client's main thread wedged, or
/// simply polled faster than it ticked) are collapsed to the first
/// observation so the timeline isn't flooded with duplicate liveness
/// pings — a *change* in the counter is the informative event.
pub fn heartbeat_events(samples: &[HeartbeatSample]) -> Vec<TimelineEvent> {
    let mut out = Vec::new();
    let mut last_tick: Option<u64> = None;
    for s in samples {
        if last_tick == Some(s.tick_count) {
            continue;
        }
        last_tick = Some(s.tick_count);
        out.push(TimelineEvent::client(
            s.observed_ms,
            "bridge.heartbeat",
            json!({ "tick_count": s.tick_count }),
        ));
    }
    out
}

/// Seam for the #686 client-event ring. Returns nothing today; when
/// `client_events_read` lands, drain the bridge event ring here and map
/// each row to [`TimelineEvent::client`] with its client `ts_ms`.
///
/// Kept as a function (not a `todo!()`) so the timeline compiles and runs
/// with heartbeat-only client data, and so callers already plumb its
/// (empty) output through the merge — flipping it on is a one-site change.
pub fn drain_event_ring() -> Vec<TimelineEvent> {
    // TODO(#686): call the bridge `events_read` method and map rows here.
    // Each row carries the client's own ts_ms → TimelineEvent::client(...).
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_events_collapse_repeats_and_keep_advances() {
        let samples = vec![
            HeartbeatSample {
                tick_count: 10,
                observed_ms: 1000,
            },
            HeartbeatSample {
                tick_count: 10,
                observed_ms: 1100,
            }, // dup: dropped
            HeartbeatSample {
                tick_count: 11,
                observed_ms: 1200,
            }, // advance: kept
            HeartbeatSample {
                tick_count: 12,
                observed_ms: 1300,
            },
            HeartbeatSample {
                tick_count: 12,
                observed_ms: 1400,
            }, // dup: dropped
        ];
        let events = heartbeat_events(&samples);
        let ts: Vec<i64> = events.iter().filter_map(|e| e.client_ts_ms).collect();
        assert_eq!(ts, vec![1000, 1200, 1300]);
        assert!(events.iter().all(|e| e.kind == "bridge.heartbeat"));
    }

    #[test]
    fn empty_ring_yields_nothing() {
        assert!(heartbeat_events(&[]).is_empty());
        assert!(drain_event_ring().is_empty());
    }
}
