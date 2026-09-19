//! `lab_timeline` — the low-latency merged client+server timeline
//! (ADR §5, phase 6 / issue #689).
//!
//! Client events (dev-box clock) and server packet-tap rows (server
//! clock) are projected onto one axis and interleaved. SigNoz remains the
//! durable copy under the dev-session id; this tool is the fast local
//! read for the experiment loop.
//!
//! Layout (directory from day one — 5 siblings per the file-org rule):
//! - [`clock`] — NTP-shaped offset estimation (pure).
//! - [`event`] — the event model + merge/order/window step (pure).
//! - [`client_events`] — local client sources: the heartbeat ring today,
//!   the #686 event-ring seam for later.
//! - [`packet_tap`] — fetch server rows from `cimmeria-lab-mcp` over HTTP.
//! - this module — the orchestrator wired onto the supervisor.
//!
//! ## What it does today vs. what is gated on #686
//!
//! Today the only client signal is the bridge **heartbeat** ring, so the
//! client side of the timeline is liveness pings interleaved with decoded
//! server packets — already enough to answer "did the client's main
//! thread keep ticking while the server sent packet X". The full client
//! event ring (hook hits, Lua prints, Mercury dispatch) was #686, which
//! the owner stopped; [`client_events::drain_event_ring`] is the seam it
//! plugs into with no change to the offset/merge machinery.

pub mod client_events;
pub mod clock;
pub mod event;
pub mod packet_tap;

use serde_json::{json, Value};

use crate::supervisor::Supervisor;

use clock::{estimate_offset, ClockOffset};
use event::{lookback_window, merge, Window};
use packet_tap::{PacketTapClient, PacketTapConfig};

/// Default lookback window when the caller doesn't specify one.
const DEFAULT_WINDOW_MS: i64 = 60_000;
/// Default packet-tap row cap.
const DEFAULT_TAP_LIMIT: u32 = 1_000;

/// Arguments accepted by the `lab_timeline` tool.
#[derive(Debug, Default)]
pub struct TimelineArgs {
    /// Server session whose packet tap to read. Omit for a client-only
    /// timeline (heartbeat ring, no server rows).
    pub session_id: Option<String>,
    /// Lookback window in ms, ending at the newest event. Default 60_000.
    pub window_ms: Option<i64>,
    /// Max tap rows to request. Default 1_000.
    pub limit: Option<u32>,
    /// Explicit server-clock lower bound handed to the tap read.
    pub since_ms: Option<i64>,
}

/// The `lab_timeline` implementation. Holds the (optional) server-endpoint
/// config; the client side comes from the supervisor per call.
pub struct Timeline {
    tap: Option<PacketTapClient>,
}

impl Timeline {
    /// Build from the environment. When `CIMMERIA_LAB_MCP_URL` /
    /// `_TOKEN` are unset the server side is absent by design (fail-closed)
    /// and the tool returns a client-only timeline.
    pub fn from_env() -> Self {
        Self {
            tap: PacketTapConfig::from_env().map(PacketTapClient::new),
        }
    }

    /// Assemble the merged window.
    pub async fn build(
        &self,
        supervisor: &Supervisor,
        args: TimelineArgs,
    ) -> Result<Value, String> {
        let window_ms = args.window_ms.unwrap_or(DEFAULT_WINDOW_MS);
        let limit = args.limit.unwrap_or(DEFAULT_TAP_LIMIT);
        let mut notes: Vec<String> = Vec::new();

        // ---- Client side (dev-box clock) ----
        let mut client_events =
            client_events::heartbeat_events(&supervisor.heartbeat_samples().await);
        client_events.extend(client_events::drain_event_ring());
        if client_events.is_empty() {
            notes.push(
                "no client events yet (heartbeat ring empty); \
                 full client-event ring is gated on #686"
                    .to_string(),
            );
        }

        // ---- Server side (server clock) + offset ping ----
        let mut server_events = Vec::new();
        let mut offset = supervisor
            .cached_offset()
            .await
            .unwrap_or_else(ClockOffset::unestimated);

        match (&self.tap, &args.session_id) {
            (Some(client), Some(session_id)) => {
                let fetch = client.fetch(session_id, args.since_ms, limit).await?;
                if let Some(fresh) = fetch
                    .ping
                    .and_then(|p| estimate_offset(&[p], "packet_tap_newest_row"))
                {
                    // Re-pin the offset from this round trip and cache it
                    // (the ADR calls for a login-time estimate; until a
                    // server `lab` ping tool exists this opportunistic
                    // re-pin is the best available — see clock.rs).
                    supervisor.set_offset(fresh.clone()).await;
                    offset = fresh;
                }
                server_events = fetch.rows.into_iter().map(|r| r.into_event()).collect();
            }
            (None, _) => notes.push(
                "server packet tap unavailable (CIMMERIA_LAB_MCP_URL/_TOKEN unset); \
                 client-only timeline"
                    .to_string(),
            ),
            (Some(_), None) => notes.push(
                "no session_id given; server packet tap not read (client-only timeline)"
                    .to_string(),
            ),
        }

        if offset.method == "unestimated" {
            notes.push(
                "clock offset unestimated (no packet-tap ping yet); client and server \
                 rows sit on their own clocks"
                    .to_string(),
            );
        }

        // ---- Window + merge ----
        let window: Option<Window> =
            lookback_window(&server_events, &client_events, &offset, window_ms);
        let events = merge(server_events, client_events, &offset, window);

        Ok(json!({
            "offset": offset,
            "window": window.map(|w| json!({ "start_ms": w.start_ms, "end_ms": w.end_ms })),
            "event_count": events.len(),
            "events": events,
            "notes": notes,
            "durable_copy": "SigNoz, under the dev-session id (ADR §5)",
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_args_default_is_empty() {
        let a = TimelineArgs::default();
        assert!(a.session_id.is_none());
        assert!(a.window_ms.is_none());
    }

    // The offset + merge + parse logic is unit-tested in the submodules
    // against mocked inputs (clock.rs, event.rs, packet_tap.rs,
    // client_events.rs). `build` itself is a thin orchestrator over those
    // plus live I/O (supervisor bridge + HTTP), exercised at the colo
    // exit criterion (#689).
}
