//! `client_wait_event`: wait for a client event that satisfies a predicate,
//! reading the store through a named cursor.
//!
//! Cursor rules (the part that stops sequential waits racing):
//!
//! - A wait scans events *after* its start seq: `since_seq` when given,
//!   else the named cursor, else (a cursor never used) the store head after
//!   a first pump, so a fresh cursor does not match stale history.
//! - A met wait moves the cursor to its last matched seq, so the next wait
//!   on that cursor sees only later events, including ones that arrived
//!   while the caller was busy between the two calls.
//! - A timed-out wait leaves the cursor where it was: nothing it scanned is
//!   lost to a later wait with a different predicate.
//! - `arm` just puts the cursor at the head (after a pump) and returns:
//!   arm, act, then wait, and nothing the action caused can be missed.
//!
//! A timeout is `met: false`, never an error. Bridge failures are errors.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::predicate::{glob, EventPredicate};
use super::store::StoredEvent;
use super::{KIND_CHAT, KIND_COMBAT};
use crate::supervisor::flows::widgets;
use crate::supervisor::Supervisor;

pub const DEFAULT_CURSOR: &str = "wait";
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
pub const MAX_TIMEOUT_MS: u64 = 600_000;
pub const DEFAULT_POLL_MS: u64 = 300;
/// Store events scanned per poll.
const SCAN_MAX: usize = 8192;

/// One wait.
#[derive(Debug, Clone)]
pub struct WaitRequest {
    pub predicate: EventPredicate,
    /// Also met when this CEGUI window is visible (level-triggered).
    pub window: Option<String>,
    pub cursor: String,
    pub since_seq: Option<u64>,
    /// Matches needed (default 1).
    pub count: usize,
    pub arm: bool,
    pub timeout: Duration,
    pub poll: Duration,
}

impl WaitRequest {
    /// Whether the Lua rings need pumping: only when the predicate could
    /// match a `combat.hit` or `chat.line` event. Saves one `lua_eval` per
    /// poll on pure bridge-event waits.
    pub fn needs_lua(&self) -> bool {
        match &self.predicate.kind {
            None => true,
            Some(k) => glob(k, KIND_COMBAT) || glob(k, KIND_CHAT),
        }
    }
}

/// Pick the matches out of a scanned batch. Returns the matches (up to
/// `want`) and the last seq scanned.
pub fn scan<'a>(
    events: &'a [StoredEvent],
    p: &EventPredicate,
    want: usize,
) -> (Vec<&'a StoredEvent>, Option<u64>) {
    let mut hits = Vec::new();
    let mut last = None;
    for e in events {
        last = Some(e.seq);
        if p.matches(e) {
            hits.push(e);
            if hits.len() >= want {
                break;
            }
        }
    }
    (hits, last)
}

impl Supervisor {
    pub async fn wait_event(&self, req: WaitRequest) -> Result<Value, String> {
        let t0 = Instant::now();
        let lua = req.needs_lua();
        let want = req.count.max(1);
        // Where this wait starts.
        let (start, cursor_before) = {
            let explicit = req.since_seq;
            let known = self.events.inner.lock().await.cursor(&req.cursor);
            match (explicit, known, req.arm) {
                (Some(s), _, false) => (s, known),
                (None, Some(c), false) => (c, known),
                // Fresh cursor or arm: pump first so the start is "now".
                _ => {
                    let rep = self.pump_events(lua).await?;
                    (rep.head, known)
                }
            }
        };
        if req.arm {
            self.events
                .inner
                .lock()
                .await
                .reset_cursor(&req.cursor, start);
            return Ok(json!({
                "armed": true,
                "cursor": { "name": req.cursor, "before": cursor_before, "seq": start },
                "elapsed_ms": t0.elapsed().as_millis() as u64,
            }));
        }
        let window_cond = req.window.as_deref().map(widgets::visible);
        let mut scan_from = start;
        let mut matched: Vec<StoredEvent> = Vec::new();
        let mut polls = 0u32;
        let mut scanned = 0usize;
        let mut gap = false;
        let mut first_match_ms = None;
        loop {
            polls += 1;
            let rep = self.pump_events(lua).await?;
            {
                let st = self.events.inner.lock().await;
                let slice = st.since(scan_from, SCAN_MAX);
                gap |= slice.gap;
                scanned += slice.events.len();
                let (hits, last) = scan(&slice.events, &req.predicate, want - matched.len());
                if !hits.is_empty() && first_match_ms.is_none() {
                    first_match_ms = Some(t0.elapsed().as_millis() as u64);
                }
                matched.extend(hits.into_iter().cloned());
                // Resume after the last match when we stopped early, else
                // after everything scanned.
                scan_from = if matched.len() >= want {
                    matched.last().map_or(scan_from, |e| e.seq)
                } else {
                    last.unwrap_or(scan_from)
                };
            }
            let mut by_window = false;
            if matched.len() < want {
                if let Some(cond) = &window_cond {
                    let r = self
                        .lua_results(&format!("return tostring({cond})"))
                        .await
                        .unwrap_or_default();
                    by_window = r.first().map(String::as_str) == Some("true");
                }
            }
            let met = matched.len() >= want || by_window;
            if met || t0.elapsed() >= req.timeout {
                let cursor_after = if met {
                    let mut st = self.events.inner.lock().await;
                    if let Some(last) = matched.last() {
                        st.set_cursor(&req.cursor, last.seq);
                    } else {
                        // Met by the window with no event: the cursor
                        // exists (forward-only, so this never rewinds it).
                        st.set_cursor(&req.cursor, start);
                    }
                    st.cursor(&req.cursor)
                } else {
                    // Timeout: leave the cursor; create it at the start
                    // when this was its first use.
                    let mut st = self.events.inner.lock().await;
                    if st.cursor(&req.cursor).is_none() {
                        st.set_cursor(&req.cursor, start);
                    }
                    st.cursor(&req.cursor)
                };
                let matched_by = if matched.len() >= want {
                    "event"
                } else if by_window {
                    "window"
                } else {
                    "none"
                };
                return Ok(json!({
                    "met": met,
                    "matched_by": matched_by,
                    "matched": matched.iter().map(StoredEvent::to_json).collect::<Vec<_>>(),
                    "count": { "wanted": want, "got": matched.len() },
                    "elapsed_ms": t0.elapsed().as_millis() as u64,
                    "first_match_ms": first_match_ms,
                    "polls": polls,
                    "scanned": scanned,
                    "gap": gap,
                    "cursor": {
                        "name": req.cursor,
                        "before": cursor_before,
                        "start": start,
                        "after": cursor_after,
                    },
                    "last_pump": rep.to_json(),
                }));
            }
            tokio::time::sleep(req.poll).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::events::store::StoreInner;

    fn filled() -> StoreInner {
        let mut st = StoreInner::with_cap(64);
        st.push(
            "cme.event",
            1,
            json!({ "event": "Event_NetIn_onTimerUpdate" }),
        );
        st.push("net.out", 2, json!({ "method": "useAbility" }));
        st.push("cme.event", 3, json!({ "event": "Event_NetIn_onSequence" }));
        st.push("cme.event", 4, json!({ "event": "Event_NetIn_onSequence" }));
        st
    }

    fn named(n: &str) -> EventPredicate {
        EventPredicate {
            name: Some(n.into()),
            ..Default::default()
        }
    }

    #[test]
    fn scan_stops_at_the_wanted_count_and_reports_progress() {
        let st = filled();
        let batch = st.since(0, 100).events;
        let (hits, last) = scan(&batch, &named("*onSequence"), 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].seq, 3);
        // Stopped at the match, so the last scanned seq is the match.
        assert_eq!(last, Some(3));
        let (hits, last) = scan(&batch, &named("*onSequence"), 5);
        assert_eq!(hits.len(), 2);
        assert_eq!(last, Some(4));
        let (none, last) = scan(&batch, &named("*onErrorCode"), 1);
        assert!(none.is_empty());
        assert_eq!(last, Some(4));
    }

    /// Two waits in a row on one cursor: the second starts after the
    /// first's match and still finds the later event, even though both were
    /// already in the store when the first wait ran (no drain race).
    #[test]
    fn sequential_waits_on_one_cursor_see_successive_matches() {
        let mut st = filled();
        let p = named("*onSequence");
        let first = st.since(0, 100).events;
        let (h1, _) = scan(&first, &p, 1);
        let seq1 = h1[0].seq;
        st.set_cursor(DEFAULT_CURSOR, seq1);
        let after = st.cursor(DEFAULT_CURSOR).unwrap();
        let second = st.since(after, 100).events;
        let (h2, _) = scan(&second, &p, 1);
        assert_eq!(h2[0].seq, 4);
    }

    /// A timed-out wait leaves the cursor, so a different predicate still
    /// sees what the first one scanned past.
    #[test]
    fn a_timeout_does_not_consume() {
        let mut st = filled();
        st.set_cursor(DEFAULT_CURSOR, 0);
        let batch = st.since(0, 100).events;
        let (miss, _) = scan(&batch, &named("*onErrorCode"), 1);
        assert!(miss.is_empty());
        // (the wait would not move the cursor here)
        let again = st.since(st.cursor(DEFAULT_CURSOR).unwrap(), 100).events;
        let (hit, _) = scan(&again, &named("useAbility"), 1);
        assert_eq!(hit[0].seq, 2);
    }

    #[test]
    fn lua_pumping_is_skipped_for_bridge_only_kinds() {
        let mk = |kind: Option<&str>| WaitRequest {
            predicate: EventPredicate {
                kind: kind.map(str::to_string),
                ..Default::default()
            },
            window: None,
            cursor: DEFAULT_CURSOR.into(),
            since_seq: None,
            count: 1,
            arm: false,
            timeout: Duration::from_millis(1),
            poll: Duration::from_millis(1),
        };
        assert!(mk(None).needs_lua());
        assert!(mk(Some("combat.*")).needs_lua());
        assert!(mk(Some("chat.line")).needs_lua());
        assert!(!mk(Some("cme.event")).needs_lua());
        assert!(!mk(Some("net.out")).needs_lua());
    }
}
