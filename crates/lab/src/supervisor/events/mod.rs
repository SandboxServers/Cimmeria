//! Client events with a persistent read cursor.
//!
//! - [`store`] — the seq-numbered, non-draining history and named cursors.
//! - [`predicate`] — what a wait matches (kind, name, entity id, text,
//!   fields), pure and tested.
//! - [`lua_rings`] — the combat-text, chat and window-click rings the lab
//!   keeps inside the client's UI Lua.
//! - [`wait`] — `client_wait_event`.
//!
//! A *pump* moves everything new into the store: the bridge ring
//! (`events_read`: `cme.event`, `net.out`, `entity.*`, `cegui.log`,
//! `lua.print`, `hook.hit`, ...) and, when asked, the three Lua rings
//! (`combat.hit`, `chat.line`, `ui.click`). The supervisor is the bridge ring's only
//! drainer; `client_events_read` reads the store through its own cursor, so
//! it still returns each event once without stealing from waits.

#[cfg(test)]
pub mod fake_bridge;
#[cfg(test)]
mod flow_tests;
mod keeper;
pub mod lua_rings;
pub mod predicate;
pub mod store;
pub mod wait;

use serde_json::{json, Value};

use super::flows::ui_state::lua_results_of;
use super::{now_ms, Supervisor};
use store::StoreInner;

/// Events asked of the bridge ring per pump (its capacity).
pub const BRIDGE_DRAIN_MAX: u32 = 4096;
/// Cursor `client_events_read` keeps.
pub const EVENTS_READ_CURSOR: &str = "events_read";
/// Kinds the Lua rings produce.
pub const KIND_COMBAT: &str = "combat.hit";
pub const KIND_CHAT: &str = "chat.line";
/// A CEGUI window click (LR-10): `{ window, button, mouse, ui_time }`.
pub const KIND_CLICK: &str = "ui.click";

/// What one pump did.
#[derive(Debug, Clone, Default)]
pub struct PumpReport {
    pub bridge_events: usize,
    /// Filled by the caller that reads (`events_read`, `wait_event`) for its
    /// own range; a plain pump leaves it 0.
    pub bridge_dropped: u64,
    /// Every event the bridge ring has dropped since the lab started.
    pub bridge_dropped_total: u64,
    /// Plain-words reading of `bridge_dropped`, when it is not zero.
    pub bridge_note: Option<String>,
    pub combat: usize,
    pub chat: usize,
    pub click: usize,
    /// Click identities rejected in the current Lua epoch before ring insertion.
    pub click_rejected_total: u64,
    /// Malformed click lines rejected while parsing this Lua read.
    pub click_rejected_lines: u64,
    /// `(ring, status)` from the Lua install step; empty when not pumped.
    pub lua_install: Vec<(String, String)>,
    /// The Lua step failed (not in the world, a Lua error); the bridge
    /// events still landed.
    pub lua_error: Option<String>,
    /// The Lua state was rebuilt since the last pump (ring restarted).
    pub lua_epoch_changed: bool,
    pub head: u64,
}

impl PumpReport {
    pub fn to_json(&self) -> Value {
        json!({
            "bridge_events": self.bridge_events,
            "bridge_dropped": self.bridge_dropped,
            "bridge_dropped_total": self.bridge_dropped_total,
            "bridge_note": self.bridge_note,
            "combat": self.combat,
            "chat": self.chat,
            "click": self.click,
            "click_rejected_total": self.click_rejected_total,
            "click_rejected_lines": self.click_rejected_lines,
            "lua_install": self.lua_install.iter()
                .map(|(r, s)| json!({ "ring": r, "status": s }))
                .collect::<Vec<_>>(),
            "lua_error": self.lua_error,
            "lua_epoch_changed": self.lua_epoch_changed,
            "head": self.head,
        })
    }
}

/// Fold one `events_read` result into the store. Returns (events, dropped).
pub fn ingest_bridge(st: &mut StoreInner, result: &Value) -> (usize, u64) {
    let dropped = result.get("dropped").and_then(Value::as_u64).unwrap_or(0);
    st.record_drops(dropped);
    let mut n = 0;
    if let Some(evs) = result.get("events").and_then(Value::as_array) {
        for e in evs {
            let kind = e
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let ts = e
                .get("ts_ms")
                .and_then(Value::as_i64)
                .unwrap_or_else(now_ms);
            let fields = e.get("fields").cloned().unwrap_or(Value::Null);
            st.push(kind, ts, fields);
            n += 1;
        }
    }
    (n, dropped)
}

/// What one Lua ring read added to the store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RingIngest {
    /// Records pushed to the store this pump, per ring.
    pub combat: usize,
    pub chat: usize,
    /// `ui.click` records (LR-10).
    pub click: usize,
    /// The Lua state was rebuilt since the last read.
    pub epoch_changed: bool,
}

/// Push one ring's records newer than its mark; returns the new mark and
/// how many were pushed.
fn ingest_ring(
    st: &mut StoreInner,
    kind: &str,
    mark: &store::LuaRingMark,
    epoch: &str,
    records: &[(u64, Value)],
    now: i64,
) -> (store::LuaRingMark, usize) {
    let after = mark.after(epoch);
    let mut seq_max = after;
    let mut n = 0;
    for (seq, fields) in records {
        if *seq > after {
            let mut f = fields.clone();
            f["ring_seq"] = json!(seq);
            st.push(kind, now, f);
            seq_max = seq_max.max(*seq);
            n += 1;
        }
    }
    (
        store::LuaRingMark {
            epoch: epoch.to_string(),
            seq: seq_max,
        },
        n,
    )
}

/// Fold one Lua ring read into the store, advancing the ring marks.
pub fn ingest_rings(
    st: &mut StoreInner,
    r: &lua_rings::RingRead,
    client_pid: Option<u32>,
) -> RingIngest {
    let epoch_changed = !st.combat_mark.epoch.is_empty() && st.combat_mark.epoch != r.epoch;
    let now = now_ms();
    let (combat_mark, chat_mark, click_mark) = (
        st.combat_mark.clone(),
        st.chat_mark.clone(),
        st.click_mark.clone(),
    );
    let (m, combat) = ingest_ring(st, KIND_COMBAT, &combat_mark, &r.epoch, &r.combat, now);
    st.combat_mark = m;
    let (m, chat) = ingest_ring(st, KIND_CHAT, &chat_mark, &r.epoch, &r.chat, now);
    st.chat_mark = m;
    let click_rows: Vec<_> = r
        .click
        .iter()
        .map(|(seq, fields)| {
            let mut fields = fields.clone();
            fields["client_pid"] = json!(client_pid.filter(|pid| *pid != 0));
            (*seq, fields)
        })
        .collect();
    let (m, click) = ingest_ring(st, KIND_CLICK, &click_mark, &r.epoch, &click_rows, now);
    st.click_mark = m;
    RingIngest {
        combat,
        chat,
        click,
        epoch_changed,
    }
}

/// Plain-words reading of `n` dropped bridge events.
pub fn drop_note(n: u64) -> String {
    format!(
        "the client's event ring overflowed and dropped {n} newest event(s) in the range read: cme.event, net.out, cegui.log and similar rows may be missing. chat.line, combat.hit and ui.click come from separate in-client rings and are not affected."
    )
}

impl Supervisor {
    /// Move new client events into the store. `lua` also installs/reads the
    /// combat, chat and click rings (one extra `lua_eval`). A bridge failure is an
    /// error; a Lua failure is reported in the result.
    pub async fn pump_events(&self, lua: bool) -> Result<PumpReport, String> {
        let mut rep = PumpReport::default();
        {
            let _drain = self.events.drain.lock().await;
            let drained = self
                .bridge_call("events_read", json!({ "max": BRIDGE_DRAIN_MAX }))
                .await?;
            let mut st = self.events.inner.lock().await;
            let (n, _) = ingest_bridge(&mut st, &drained);
            rep.bridge_events = n;
            rep.bridge_dropped_total = st.upstream_dropped;
        }
        if lua {
            let (combat_after, chat_after, click_after, epoch) = {
                let st = self.events.inner.lock().await;
                (
                    st.combat_mark.seq,
                    st.chat_mark.seq,
                    st.click_mark.seq,
                    st.combat_mark.epoch.clone(),
                )
            };
            // Ask from the stored marks (from 0 before the first read). A
            // rebuilt Lua state restarts its ring at 1, so a read with the
            // old marks could skip records; that case re-reads from 0.
            let chunk = if epoch.is_empty() {
                lua_rings::pump_chunk(0, 0, 0, lua_rings::READ_MAX)
            } else {
                lua_rings::pump_chunk(combat_after, chat_after, click_after, lua_rings::READ_MAX)
            };
            let pid_before = self.client_pid().await;
            match self.lua_ring_read(&chunk).await {
                Ok(mut read) => {
                    if !epoch.is_empty() && read.epoch != epoch {
                        // A new Lua state: its ring restarted at 1, so the
                        // marks we asked with may have skipped records.
                        if let Ok(full) = self
                            .lua_ring_read(&lua_rings::pump_chunk(0, 0, 0, lua_rings::READ_MAX))
                            .await
                        {
                            read = full;
                        }
                    }
                    let pid_after = self.client_pid().await;
                    let client_pid = if pid_before == pid_after {
                        pid_after
                    } else {
                        None
                    };
                    let mut st = self.events.inner.lock().await;
                    let got = ingest_rings(&mut st, &read, client_pid);
                    rep.combat = got.combat;
                    rep.chat = got.chat;
                    rep.click = got.click;
                    rep.click_rejected_total = read.click_rejected_total;
                    rep.click_rejected_lines = read.click_rejected_lines;
                    rep.lua_epoch_changed = got.epoch_changed;
                    rep.lua_install = read.install;
                }
                Err(e) => rep.lua_error = Some(e),
            }
        }
        rep.head = self.events.inner.lock().await.head();
        Ok(rep)
    }

    async fn lua_ring_read(&self, chunk: &str) -> Result<lua_rings::RingRead, String> {
        let v = self
            .bridge_call("lua_eval", json!({ "chunk": chunk }))
            .await?;
        let lines = lua_results_of(&v)?;
        Ok(lua_rings::parse_ring_read(&lines))
    }

    /// The store's newest seq, without pumping.
    pub async fn event_head(&self) -> u64 {
        self.events.inner.lock().await.head()
    }

    /// Read a bounded event-history slice without moving any named cursor.
    pub async fn trace_snapshot(&self, after: u64, max: usize) -> (store::Slice, u64) {
        let st = self.events.inner.lock().await;
        (
            st.since(after, max.min(store::STORE_CAP)),
            st.upstream_dropped,
        )
    }

    /// `client_events_read`: pump the bridge ring, then return what the
    /// `events_read` cursor has not seen yet, and advance it. Each event is
    /// returned once to this reader, as before, but no other reader loses
    /// it. `max` bounds the batch; the rest stays for the next call.
    pub async fn events_read(&self, max: Option<u32>) -> Result<Value, String> {
        let rep = self.pump_events(false).await?;
        let max = max.unwrap_or(512).clamp(1, store::STORE_CAP as u32) as usize;
        let mut st = self.events.inner.lock().await;
        let after = st.cursor(EVENTS_READ_CURSOR).unwrap_or(0);
        let slice = st.since(after, max);
        if let Some(last) = slice.events.last() {
            st.set_cursor(EVENTS_READ_CURSOR, last.seq);
        }
        // Drops that fell inside what this read covered, whoever pumped them.
        let upto = if slice.events.len() >= max {
            slice.events.last().map_or(slice.head, |e| e.seq)
        } else {
            slice.head
        };
        let dropped = st.take_drops_up_to(EVENTS_READ_CURSOR, upto);
        Ok(json!({
            "returned": slice.events.len(),
            "dropped": dropped,
            "dropped_total": rep.bridge_dropped_total,
            "bridge_note": (dropped > 0).then(|| drop_note(dropped)),
            "gap": slice.gap,
            "head": slice.head,
            "cursors": st.cursors(),
            "events": slice.events.iter().map(store::StoredEvent::to_json).collect::<Vec<_>>(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    #[tokio::test]
    async fn trace_snapshot_is_bounded_reports_eviction_and_leaves_reader_cursor() {
        let sup = super::fake_bridge::supervisor(Arc::new(|_, _| {
            Err("bridge must not be called by a passive trace read".into())
        }))
        .await;
        {
            let mut st = sup.events.inner.lock().await;
            *st = StoreInner::with_cap(2);
            st.push("ui.click", 1, json!({}));
            st.push("net.out", 2, json!({}));
            st.push("ability.recv", 3, json!({}));
            st.record_drops(2);
            st.set_cursor(EVENTS_READ_CURSOR, 1);
        }
        let (slice, dropped) = sup.trace_snapshot(0, 1).await;
        assert!(slice.gap);
        assert_eq!(dropped, 2);
        assert_eq!(slice.events.len(), 1);
        assert_eq!(slice.events[0].seq, 2);
        let st = sup.events.inner.lock().await;
        assert_eq!(st.cursor(EVENTS_READ_CURSOR), Some(1));
        assert_eq!(st.since(1, 2).events.len(), 2);
    }

    #[test]
    fn bridge_results_land_in_order_with_their_fields() {
        let mut st = StoreInner::with_cap(16);
        let r = json!({
            "returned": 2, "dropped": 3,
            "events": [
                { "kind": "cme.event", "ts_ms": 10, "fields": { "event": "Event_NetIn_onSequence" } },
                { "kind": "net.out", "ts_ms": 11, "fields": { "method": "useAbility" } },
            ]
        });
        assert_eq!(ingest_bridge(&mut st, &r), (2, 3));
        assert_eq!(st.upstream_dropped, 3);
        assert_eq!(st.last_drop_id(), 1);
        let all = st.since(0, 10);
        assert_eq!(all.events[0].kind, "cme.event");
        assert_eq!(all.events[1].fields["method"], "useAbility");
        assert_eq!(all.events[1].ts_ms, 11);
    }

    fn read(epoch: &str, combat: &[u64], chat: &[u64]) -> lua_rings::RingRead {
        lua_rings::RingRead {
            epoch: epoch.into(),
            combat: combat.iter().map(|s| (*s, json!({ "n": s }))).collect(),
            chat: chat.iter().map(|s| (*s, json!({ "text": "x" }))).collect(),
            ..Default::default()
        }
    }

    fn counts(g: RingIngest) -> (usize, usize, usize, bool) {
        (g.combat, g.chat, g.click, g.epoch_changed)
    }

    #[test]
    fn ring_records_are_ingested_once() {
        let mut st = StoreInner::with_cap(16);
        assert_eq!(
            counts(ingest_rings(&mut st, &read("e1", &[1, 2], &[1]), Some(412))),
            (2, 1, 0, false)
        );
        // The same records again (a pump that re-read them) add nothing.
        assert_eq!(
            counts(ingest_rings(
                &mut st,
                &read("e1", &[1, 2, 3], &[1]),
                Some(412)
            )),
            (1, 0, 0, false)
        );
        let kinds: Vec<String> = st.since(0, 10).events.into_iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![KIND_COMBAT, KIND_COMBAT, KIND_CHAT, KIND_COMBAT]
        );
        assert_eq!(st.since(3, 1).events[0].fields["ring_seq"], 3);
    }

    #[test]
    fn a_new_epoch_takes_the_restarted_ring_from_seq_one() {
        let mut st = StoreInner::with_cap(16);
        ingest_rings(&mut st, &read("e1", &[1, 2, 3, 4, 5], &[]), Some(412));
        let got = ingest_rings(&mut st, &read("e2", &[1, 2], &[]), Some(412));
        assert!(got.epoch_changed);
        assert_eq!(got.combat, 2, "seqs 1 and 2 of the new ring are new events");
        assert_eq!(st.combat_mark.seq, 2);
    }

    fn click(seq: u64, window: &str, button: &str) -> (u64, Value) {
        (
            seq,
            json!({ "window": window, "button": button, "mouse": "left", "ui_time": 1 }),
        )
    }

    /// LR-10: clicks land as `ui.click` with their fields, once, on their
    /// own mark, and a rebuilt Lua state rereads the click ring from 1.
    #[test]
    fn click_records_land_as_ui_click_once() {
        let mut st = StoreInner::with_cap(16);
        let mut r = read("e1", &[], &[]);
        r.click = vec![
            click(1, "TutorialWin", "TutorialWin__auto_closebutton__"),
            click(2, "ChatWin", "ChatWin_Tab2"),
        ];
        assert_eq!(
            counts(ingest_rings(&mut st, &r, Some(412))),
            (0, 0, 2, false)
        );
        let evs = st.since(0, 10).events;
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].kind, KIND_CLICK);
        assert_eq!(evs[0].fields["window"], "TutorialWin");
        assert_eq!(evs[0].fields["button"], "TutorialWin__auto_closebutton__");
        assert_eq!(evs[0].fields["ring_seq"], 1);
        assert_eq!(evs[0].fields["client_pid"], 412);
        assert_eq!(st.click_mark.seq, 2);
        // Re-read with one more: only the new one lands; the other marks
        // are untouched by click traffic.
        r.click
            .push(click(3, "BagWin", "BagWin__auto_closebutton__"));
        assert_eq!(
            counts(ingest_rings(&mut st, &r, Some(412))),
            (0, 0, 1, false)
        );
        assert_eq!(st.combat_mark.seq, 0);
        // A new Lua state: its click ring restarted at 1.
        let mut fresh = read("e2", &[], &[]);
        fresh.click = vec![click(1, "TutorialWin", "TutorialWin__auto_closebutton__")];
        assert_eq!(
            counts(ingest_rings(&mut st, &fresh, Some(412))),
            (0, 0, 1, true)
        );
        assert_eq!(
            st.click_mark,
            store::LuaRingMark {
                epoch: "e2".into(),
                seq: 1
            }
        );
    }

    #[test]
    fn click_pid_is_fixed_at_ingest_across_relog() {
        let mut st = StoreInner::with_cap(16);
        let mut old = read("e1", &[], &[]);
        old.click = vec![click(1, "Win", "Button")];
        ingest_rings(&mut st, &old, Some(412));
        let mut fresh = read("e2", &[], &[]);
        fresh.click = vec![click(1, "Win", "Button")];
        ingest_rings(&mut st, &fresh, Some(913));
        let events = st.since(0, 10).events;
        assert_eq!(events[0].fields["client_pid"], 412);
        assert_eq!(events[1].fields["client_pid"], 913);
    }

    #[test]
    fn pump_report_exposes_click_rejections() {
        let report = PumpReport {
            click_rejected_total: 2,
            click_rejected_lines: 1,
            ..Default::default()
        };
        let value = report.to_json();
        assert_eq!(value["click_rejected_total"], 2);
        assert_eq!(value["click_rejected_lines"], 1);
    }
}
