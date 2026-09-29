//! The supervisor-side event history: a bounded, seq-numbered copy of
//! everything the lab has pulled out of the client, with named cursors.
//!
//! The bridge's own ring (`events_read`) is drain-and-clear: whoever reads
//! first takes the events. Two readers (a `client_wait_event` and a
//! `client_events_read`, or two waits in a row) would steal from each
//! other. So the supervisor is the only drainer: every pull lands here with
//! a monotonically increasing `seq`, and each reader keeps a cursor (a seq)
//! instead of consuming. Seqs never repeat within one lab process, across
//! client restarts too, so a cursor can never point at a different event
//! than the one it was taken from.

use std::collections::{HashMap, VecDeque};

use serde_json::{json, Value};

/// Events kept. Older ones are evicted; a cursor behind the oldest kept
/// seq reads as a gap.
pub const STORE_CAP: usize = 8192;

/// One stored event.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredEvent {
    pub seq: u64,
    /// `cme.event`, `net.out`, `entity.enter`, `cegui.log`, `lua.print`,
    /// `hook.hit` (bridge ring), or `combat.hit` / `chat.line` (the lab's
    /// Lua rings, see [`super::lua_rings`]).
    pub kind: String,
    /// Client wall clock, ms since the epoch.
    pub ts_ms: i64,
    pub fields: Value,
}

impl StoredEvent {
    pub fn to_json(&self) -> Value {
        json!({
            "seq": self.seq,
            "kind": self.kind,
            "ts_ms": self.ts_ms,
            "fields": self.fields,
        })
    }
}

/// A read from a seq onwards.
#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    pub events: Vec<StoredEvent>,
    /// Events after the requested seq were evicted before this read: the
    /// reader missed some.
    pub gap: bool,
    /// Newest seq in the store at read time (0 when empty).
    pub head: u64,
}

/// Where the Lua rings were last read up to. `epoch` changes when the
/// client's Lua state is rebuilt (a relaunch or an interface reload), which
/// restarts the ring's own seqs at 1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LuaRingMark {
    pub epoch: String,
    pub seq: u64,
}

impl LuaRingMark {
    /// The ring seq to read after, given the epoch the client reports now.
    /// A new epoch means a fresh ring: read it from the start.
    pub fn after(&self, epoch: &str) -> u64 {
        if self.epoch == epoch {
            self.seq
        } else {
            0
        }
    }
}

#[derive(Debug, Default)]
pub struct StoreInner {
    next_seq: u64,
    buf: VecDeque<StoredEvent>,
    cap: usize,
    /// Events the bridge ring dropped before we drained it (ring full).
    pub upstream_dropped: u64,
    cursors: HashMap<String, u64>,
    pub combat_mark: LuaRingMark,
    pub chat_mark: LuaRingMark,
}

impl StoreInner {
    pub fn with_cap(cap: usize) -> Self {
        Self {
            next_seq: 1,
            cap: cap.max(1),
            ..Default::default()
        }
    }

    /// Newest seq handed out (0 before the first event).
    pub fn head(&self) -> u64 {
        self.next_seq - 1
    }

    /// Oldest seq still kept (the head + 1 when empty).
    pub fn oldest(&self) -> u64 {
        self.buf.front().map_or(self.next_seq, |e| e.seq)
    }

    /// Append one event and return its seq.
    pub fn push(&mut self, kind: impl Into<String>, ts_ms: i64, fields: Value) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.buf.push_back(StoredEvent {
            seq,
            kind: kind.into(),
            ts_ms,
            fields,
        });
        while self.buf.len() > self.cap {
            self.buf.pop_front();
        }
        seq
    }

    /// Events with `seq > after`, oldest first, at most `max`.
    pub fn since(&self, after: u64, max: usize) -> Slice {
        // A gap: the first event the reader wants (after + 1) is gone.
        let gap = after + 1 < self.oldest() && after < self.head();
        let events = self
            .buf
            .iter()
            .filter(|e| e.seq > after)
            .take(max)
            .cloned()
            .collect();
        Slice {
            events,
            gap,
            head: self.head(),
        }
    }

    /// A named cursor's position; `None` when it was never set.
    pub fn cursor(&self, name: &str) -> Option<u64> {
        self.cursors.get(name).copied()
    }

    /// Move a named cursor. Cursors only move forward: a stale writer
    /// (a slow wait finishing after a newer one) never rewinds it.
    pub fn set_cursor(&mut self, name: &str, seq: u64) {
        let c = self.cursors.entry(name.to_string()).or_insert(0);
        *c = (*c).max(seq);
    }

    /// Put a named cursor at an exact seq, backwards included (an explicit
    /// `since_seq` from the caller).
    pub fn reset_cursor(&mut self, name: &str, seq: u64) {
        self.cursors.insert(name.to_string(), seq);
    }

    /// Every cursor, for status reads.
    pub fn cursors(&self) -> Value {
        let mut names: Vec<_> = self.cursors.iter().collect();
        names.sort();
        json!(names
            .into_iter()
            .map(|(k, v)| json!({ "name": k, "seq": v }))
            .collect::<Vec<_>>())
    }
}

/// The shared store: a tokio mutex so pumps and readers on different MCP
/// requests serialize.
#[derive(Debug)]
pub struct EventStore {
    pub inner: tokio::sync::Mutex<StoreInner>,
}

impl Default for EventStore {
    fn default() -> Self {
        Self {
            inner: tokio::sync::Mutex::new(StoreInner::with_cap(STORE_CAP)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(cap: usize) -> StoreInner {
        StoreInner::with_cap(cap)
    }

    #[test]
    fn seqs_start_at_one_and_increase() {
        let mut s = store(8);
        assert_eq!(s.head(), 0);
        assert_eq!(s.push("a", 1, json!({})), 1);
        assert_eq!(s.push("b", 2, json!({})), 2);
        assert_eq!(s.head(), 2);
        let all = s.since(0, 100);
        assert_eq!(
            all.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(!all.gap);
    }

    /// Reading does not consume: two readers at the same cursor see the
    /// same events (the drain-and-clear race this store exists to end).
    #[test]
    fn reads_do_not_consume() {
        let mut s = store(8);
        s.push("a", 1, json!({}));
        s.push("b", 2, json!({}));
        assert_eq!(s.since(0, 10).events.len(), 2);
        assert_eq!(s.since(0, 10).events.len(), 2);
        assert_eq!(s.since(1, 10).events[0].kind, "b");
        assert!(s.since(2, 10).events.is_empty());
    }

    #[test]
    fn eviction_reports_a_gap_only_to_readers_behind_it() {
        let mut s = store(3);
        for i in 0..5 {
            s.push("k", i, json!({}));
        }
        // Kept: 3, 4, 5.
        assert_eq!(s.oldest(), 3);
        let behind = s.since(1, 10);
        assert!(behind.gap, "seq 2 was evicted before the reader got it");
        assert_eq!(behind.events[0].seq, 3);
        // A reader at 2 wants 3 onwards: nothing missed.
        assert!(!s.since(2, 10).gap);
        // A reader at the head is never in a gap.
        assert!(!s.since(5, 10).gap);
    }

    #[test]
    fn max_bounds_a_read() {
        let mut s = store(10);
        for i in 0..6 {
            s.push("k", i, json!({}));
        }
        let r = s.since(0, 2);
        assert_eq!(r.events.len(), 2);
        assert_eq!(r.head, 6);
    }

    #[test]
    fn cursors_only_move_forward_unless_reset() {
        let mut s = store(4);
        assert_eq!(s.cursor("wait"), None);
        s.set_cursor("wait", 5);
        s.set_cursor("wait", 3);
        assert_eq!(s.cursor("wait"), Some(5));
        s.reset_cursor("wait", 2);
        assert_eq!(s.cursor("wait"), Some(2));
        // Cursors are independent.
        s.set_cursor("combat", 9);
        assert_eq!(s.cursor("wait"), Some(2));
        assert_eq!(s.cursors()[0]["name"], "combat");
    }

    #[test]
    fn a_new_lua_epoch_rereads_the_ring_from_the_start() {
        let mark = LuaRingMark {
            epoch: "e1".into(),
            seq: 40,
        };
        assert_eq!(mark.after("e1"), 40);
        assert_eq!(mark.after("e2"), 0);
    }
}
