//! Client events with a persistent read cursor.
//!
//! - [`store`] — the seq-numbered, non-draining history and named cursors.
//! - [`predicate`] — what a wait matches (kind, name, entity id, text,
//!   fields), pure and tested.
//! - [`lua_rings`] — the combat-text and chat rings the lab keeps inside
//!   the client's UI Lua.
//! - [`wait`] — `client_wait_event`.
//!
//! A *pump* moves everything new into the store: the bridge ring
//! (`events_read`: `cme.event`, `net.out`, `entity.*`, `cegui.log`,
//! `lua.print`, `hook.hit`, ...) and, when asked, the two Lua rings
//! (`combat.hit`, `chat.line`). The supervisor is the bridge ring's only
//! drainer; `client_events_read` reads the store through its own cursor, so
//! it still returns each event once without stealing from waits.

#[cfg(test)]
pub mod fake_bridge;
#[cfg(test)]
mod flow_tests;
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

/// What one pump did.
#[derive(Debug, Clone, Default)]
pub struct PumpReport {
    pub bridge_events: usize,
    pub bridge_dropped: u64,
    pub combat: usize,
    pub chat: usize,
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
            "combat": self.combat,
            "chat": self.chat,
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
    st.upstream_dropped += dropped;
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

/// Fold one Lua ring read into the store, advancing the ring marks.
/// Returns (combat records, chat records, epoch changed).
pub fn ingest_rings(st: &mut StoreInner, r: &lua_rings::RingRead) -> (usize, usize, bool) {
    let changed = !st.combat_mark.epoch.is_empty() && st.combat_mark.epoch != r.epoch;
    let now = now_ms();
    let combat_after = st.combat_mark.after(&r.epoch);
    let chat_after = st.chat_mark.after(&r.epoch);
    let mut nc = 0;
    let mut combat_seq = combat_after;
    for (seq, fields) in &r.combat {
        if *seq > combat_after {
            let mut f = fields.clone();
            f["ring_seq"] = json!(seq);
            st.push(KIND_COMBAT, now, f);
            combat_seq = combat_seq.max(*seq);
            nc += 1;
        }
    }
    let mut nh = 0;
    let mut chat_seq = chat_after;
    for (seq, fields) in &r.chat {
        if *seq > chat_after {
            let mut f = fields.clone();
            f["ring_seq"] = json!(seq);
            st.push(KIND_CHAT, now, f);
            chat_seq = chat_seq.max(*seq);
            nh += 1;
        }
    }
    st.combat_mark = store::LuaRingMark {
        epoch: r.epoch.clone(),
        seq: combat_seq,
    };
    st.chat_mark = store::LuaRingMark {
        epoch: r.epoch.clone(),
        seq: chat_seq,
    };
    (nc, nh, changed)
}

impl Supervisor {
    /// Move new client events into the store. `lua` also installs/reads the
    /// combat and chat rings (one extra `lua_eval`). A bridge failure is an
    /// error; a Lua failure is reported in the result.
    pub async fn pump_events(&self, lua: bool) -> Result<PumpReport, String> {
        let mut rep = PumpReport::default();
        let drained = self
            .bridge_call("events_read", json!({ "max": BRIDGE_DRAIN_MAX }))
            .await?;
        {
            let mut st = self.events.inner.lock().await;
            let (n, d) = ingest_bridge(&mut st, &drained);
            rep.bridge_events = n;
            rep.bridge_dropped = d;
        }
        if lua {
            let (combat_after, chat_after, epoch) = {
                let st = self.events.inner.lock().await;
                (
                    st.combat_mark.seq,
                    st.chat_mark.seq,
                    st.combat_mark.epoch.clone(),
                )
            };
            // Ask from the stored marks (from 0 before the first read). A
            // rebuilt Lua state restarts its ring at 1, so a read with the
            // old marks could skip records; that case re-reads from 0.
            let chunk = if epoch.is_empty() {
                lua_rings::pump_chunk(0, 0, lua_rings::READ_MAX)
            } else {
                lua_rings::pump_chunk(combat_after, chat_after, lua_rings::READ_MAX)
            };
            match self.lua_ring_read(&chunk).await {
                Ok(mut read) => {
                    if !epoch.is_empty() && read.epoch != epoch {
                        // A new Lua state: its ring restarted at 1, so the
                        // marks we asked with may have skipped records.
                        if let Ok(full) = self
                            .lua_ring_read(&lua_rings::pump_chunk(0, 0, lua_rings::READ_MAX))
                            .await
                        {
                            read = full;
                        }
                    }
                    let mut st = self.events.inner.lock().await;
                    let (nc, nh, changed) = ingest_rings(&mut st, &read);
                    rep.combat = nc;
                    rep.chat = nh;
                    rep.lua_epoch_changed = changed;
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
        Ok(json!({
            "returned": slice.events.len(),
            "dropped": rep.bridge_dropped,
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

    #[test]
    fn ring_records_are_ingested_once() {
        let mut st = StoreInner::with_cap(16);
        assert_eq!(
            ingest_rings(&mut st, &read("e1", &[1, 2], &[1])),
            (2, 1, false)
        );
        // The same records again (a pump that re-read them) add nothing.
        assert_eq!(
            ingest_rings(&mut st, &read("e1", &[1, 2, 3], &[1])),
            (1, 0, false)
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
        ingest_rings(&mut st, &read("e1", &[1, 2, 3, 4, 5], &[]));
        let (nc, _, changed) = ingest_rings(&mut st, &read("e2", &[1, 2], &[]));
        assert!(changed);
        assert_eq!(nc, 2, "seqs 1 and 2 of the new ring are new events");
        assert_eq!(st.combat_mark.seq, 2);
    }
}
