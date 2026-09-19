//! Per-player ordered event journal.
//!
//! Most hard bugs from the 2026-09-18 playtest were **ordering** bugs across
//! systems that log independently: a cover edge one second before the step
//! that consumes it, a dialog replacing another 0.6 s later, a follow armed
//! during a ring transport, region hints that stop after a respawn. Each
//! system's own log was correct; the relationship between them had to be
//! rebuilt by hand from timestamps in ~40 scopes.
//!
//! The journal gives every notable per-player event one shared, strictly
//! increasing `seq` and one stable target (`player.journal`), so
//! `scope_name = 'player.journal' AND entity_id = N` *is* the cross-system
//! order. A short in-memory ring per player lets a later event report what
//! happened in between (deferred actions) and lets `.bug` attach the lead-up.
//!
//! `kind` values are a closed vocabulary — see [`kinds`].

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// Ring capacity per player.
pub(crate) const RING: usize = 64;

/// The closed `kind` vocabulary.
pub(crate) mod kinds {
    pub(crate) const WORLD_ENTER: &str = "world_enter";
    pub(crate) const REANCHOR: &str = "reanchor";
    pub(crate) const REGION_HINT: &str = "region_hint";
    /// A hint the server threw away. Paired with [`REGION_HINT`] so a `.bug`
    /// report can tell "the client never sent it" from "the server refused
    /// it" — the two look identical in-game (the door does nothing).
    pub(crate) const REGION_HINT_REFUSED: &str = "region_hint_refused";
    pub(crate) const COVER_EDGE: &str = "cover_edge";
    pub(crate) const STEP_ADVANCE: &str = "step_advance";
    pub(crate) const MISSION_COMPLETE: &str = "mission_complete";
    pub(crate) const DIALOG: &str = "dialog";
    pub(crate) const ACTION_LIST: &str = "action_list";
    pub(crate) const DEFERRED_SCHEDULED: &str = "deferred_scheduled";
    pub(crate) const DEFERRED_FIRED: &str = "deferred_fired";
    pub(crate) const DEATH: &str = "death";
    pub(crate) const RESPAWN: &str = "respawn";
    pub(crate) const KILL: &str = "kill";
    pub(crate) const TELEPORT: &str = "teleport";
    /// The player learned a stargate address (content grant). Answers
    /// "could they dial it yet?" from the bookmark alone — the client's
    /// DHD list and the server's dial gate are separate copies, and a
    /// player whose grant never landed sees a greyed-out destination with
    /// no error.
    pub(crate) const STARGATE_ADDRESS: &str = "stargate_address";
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    pub seq: u64,
    pub at: Instant,
    pub kind: &'static str,
    pub detail: String,
}

#[derive(Debug, Default)]
pub(crate) struct Journal {
    next_seq: u64,
    ring: VecDeque<Entry>,
}

impl Journal {
    pub(crate) fn push(&mut self, kind: &'static str, detail: String, at: Instant) -> u64 {
        self.next_seq += 1;
        if self.ring.len() == RING {
            self.ring.pop_front();
        }
        self.ring.push_back(Entry {
            seq: self.next_seq,
            at,
            kind,
            detail,
        });
        self.next_seq
    }

    /// Entries with `seq` strictly between the two bounds, oldest first.
    pub(crate) fn between(&self, after_seq: u64, before_seq: u64) -> Vec<&Entry> {
        self.ring
            .iter()
            .filter(|e| e.seq > after_seq && e.seq < before_seq)
            .collect()
    }

    pub(crate) fn tail(&self, n: usize) -> Vec<&Entry> {
        let skip = self.ring.len().saturating_sub(n);
        self.ring.iter().skip(skip).collect()
    }
}

static JOURNALS: LazyLock<Mutex<HashMap<u32, Journal>>> = LazyLock::new(Mutex::default);

fn with<R>(entity_id: u32, f: impl FnOnce(&mut Journal) -> R) -> R {
    let mut guard = JOURNALS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(guard.entry(entity_id).or_default())
}

/// Record one event for a player. Returns its `seq`.
pub(crate) fn note(entity_id: u32, kind: &'static str, detail: impl Into<String>) -> u64 {
    let detail = detail.into();
    let seq = with(entity_id, |j| j.push(kind, detail.clone(), Instant::now()));
    tracing::debug!(
        target: "player.journal",
        entity_id,
        seq,
        kind,
        detail = %detail,
        "player journal"
    );
    seq
}

/// `kind:detail` strings for everything journaled strictly between two seqs.
pub(crate) fn between(entity_id: u32, after_seq: u64, before_seq: u64) -> Vec<String> {
    with(entity_id, |j| {
        j.between(after_seq, before_seq)
            .into_iter()
            .map(|e| format!("#{} {}:{}", e.seq, e.kind, e.detail))
            .collect()
    })
}

/// The last `n` entries as `(seq, ms_ago, kind, detail)`, oldest first.
pub(crate) fn tail(entity_id: u32, n: usize) -> Vec<(u64, u64, &'static str, String)> {
    let now = Instant::now();
    with(entity_id, |j| {
        j.tail(n)
            .into_iter()
            .map(|e| {
                (
                    e.seq,
                    now.saturating_duration_since(e.at).as_millis() as u64,
                    e.kind,
                    e.detail.clone(),
                )
            })
            .collect()
    })
}

/// What a deferred action's firing looked like relative to its scheduling.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeferredReport {
    pub scheduled_seq: u64,
    pub fired_seq: u64,
    pub delay_ms: i32,
    /// How long after its deadline it actually fired (tick granularity).
    pub late_ms: u64,
    /// Everything journaled for this player between schedule and fire.
    pub between: Vec<String>,
}

static DEFERRED: LazyLock<Mutex<HashMap<(u32, i64, Instant), (u64, i32)>>> =
    LazyLock::new(Mutex::default);

/// First token of an action's `Debug` form -- its variant name.
pub(crate) fn action_kind(action: &impl std::fmt::Debug) -> String {
    let d = format!("{action:?}");
    d.split([' ', '{', '('])
        .next()
        .unwrap_or_default()
        .to_string()
}

/// A content action was queued to fire at `fire_at`.
pub(crate) fn deferred_scheduled(
    entity_id: u32,
    chain_id: i64,
    delay_ms: i32,
    fire_at: Instant,
    kind: &str,
) {
    let seq = note(
        entity_id,
        kinds::DEFERRED_SCHEDULED,
        format!("chain={chain_id} {kind} delay_ms={delay_ms}"),
    );
    DEFERRED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert((entity_id, chain_id, fire_at), (seq, delay_ms));
}

/// The queued action is firing now. `None` if it was never seen scheduled
/// (constructed directly, e.g. in tests).
pub(crate) fn deferred_fired(
    entity_id: u32,
    chain_id: i64,
    fire_at: Instant,
    kind: &str,
) -> Option<DeferredReport> {
    let (scheduled_seq, delay_ms) = DEFERRED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&(entity_id, chain_id, fire_at))?;
    let fired_seq = note(
        entity_id,
        kinds::DEFERRED_FIRED,
        format!("chain={chain_id} {kind} scheduled_seq={scheduled_seq}"),
    );
    Some(DeferredReport {
        scheduled_seq,
        fired_seq,
        delay_ms,
        late_ms: Instant::now()
            .saturating_duration_since(fire_at)
            .as_millis() as u64,
        between: between(entity_id, scheduled_seq, fired_seq),
    })
}

/// Drop a player's journal (entity ids are recycled).
pub(crate) fn forget(entity_id: u32) {
    JOURNALS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&entity_id);
    DEFERRED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .retain(|(eid, _, _), _| *eid != entity_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_is_strictly_increasing_and_between_is_exclusive() {
        let mut j = Journal::default();
        let t = Instant::now();
        let a = j.push(kinds::DEFERRED_SCHEDULED, "c1".into(), t);
        let b = j.push(kinds::COVER_EDGE, "entered 1381".into(), t);
        let c = j.push(kinds::DIALOG, "2516".into(), t);
        let d = j.push(kinds::DEFERRED_FIRED, "c1".into(), t);
        assert_eq!((a, b, c, d), (1, 2, 3, 4));
        let mid: Vec<u64> = j.between(a, d).into_iter().map(|e| e.seq).collect();
        assert_eq!(mid, vec![2, 3], "bounds themselves are excluded");
        assert!(j.between(d, d + 10).is_empty());
    }

    #[test]
    fn ring_is_bounded_and_keeps_the_newest() {
        let mut j = Journal::default();
        let t = Instant::now();
        for i in 0..(RING as u64 + 10) {
            j.push(kinds::REGION_HINT, i.to_string(), t);
        }
        assert_eq!(j.ring.len(), RING);
        let tail = j.tail(3);
        assert_eq!(tail.len(), 3);
        assert_eq!(
            tail[2].seq,
            RING as u64 + 10,
            "seq keeps counting past the ring"
        );
        assert_eq!(tail[0].seq, RING as u64 + 8);
    }

    #[test]
    fn note_logs_to_the_stable_target_with_seq() {
        let logs = crate::test_support::LogCapture::install();
        let s1 = note(4_100_001, kinds::DEATH, "killer=100034");
        let s2 = note(4_100_001, kinds::RESPAWN, "respawner=3");
        assert_eq!(s2, s1 + 1);
        let ev = logs
            .find_message(tracing::Level::DEBUG, "player journal")
            .expect("journal row");
        assert_eq!(ev.target, "player.journal");
        assert!(ev.has_field("kind", "death"));
        assert_eq!(between(4_100_001, s1 - 1, s2 + 1).len(), 2);
        forget(4_100_001);
        assert!(tail(4_100_001, 5).is_empty());
    }

    /// The dialog-5859 shape: something else happens to the player between a
    /// deferred action being scheduled and firing, and the fire report says so.
    #[test]
    fn deferred_report_lists_what_happened_in_between() {
        let eid = 4_100_002;
        let fire_at = Instant::now();
        deferred_scheduled(eid, 1172, 10_600, fire_at, "DisplayDialog");
        note(eid, kinds::DIALOG, "dialog=2516 chain=1161");
        note(eid, kinds::COVER_EDGE, "entered set=1381 crouched=false");
        let r = deferred_fired(eid, 1172, fire_at, "DisplayDialog").expect("was scheduled");
        assert_eq!(r.delay_ms, 10_600);
        assert_eq!(r.fired_seq, r.scheduled_seq + 3);
        assert_eq!(r.between.len(), 2);
        assert!(r.between[0].contains("dialog:dialog=2516"));
        // Fired once; a second fire for the same key is unknown.
        assert!(deferred_fired(eid, 1172, fire_at, "DisplayDialog").is_none());
        forget(eid);
    }

    #[test]
    fn action_kind_is_the_variant_name() {
        #[derive(Debug)]
        #[allow(dead_code)]
        enum A {
            AddItem { id: i32 },
            Unit,
            Tuple(i32),
        }
        assert_eq!(action_kind(&A::AddItem { id: 1 }), "AddItem");
        assert_eq!(action_kind(&A::Unit), "Unit");
        assert_eq!(action_kind(&A::Tuple(3)), "Tuple");
    }
}
