//! LR-02: the row segmenter, which cuts a recording into rows at anchors
//! (D-LR4). An anchor is a call the person made on purpose (interact, use
//! an item or ability, move an item, or enter a region that changed
//! something); everything up to the next anchor belongs to its row.

use serde_json::Value;

use super::decode::{decode_call, Call};
use super::log::{DbSnapshot, RecordLog, UiClick};

/// What kind of call opened a row.
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorKind {
    /// `interact`: the person used an NPC or object.
    Interact,
    /// `useItem`: the person used an inventory item.
    UseItem,
    /// `moveItem`: the person moved, equipped or split an item.
    MoveItem,
    /// `useAbility`: the person fired an ability.
    UseAbility,
    /// A region entry after which the DB snapshot changed.
    Region,
}

/// The call that opened a row.
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    /// Which call it was; the row's tables pick their steps from this.
    pub kind: AnchorKind,
    /// The tick (`seq`) the call was recorded in; the row's first tick.
    pub tick: u32,
    /// The call as decoded; an `interact(0)` stays 0 here.
    pub call: Call,
    /// The entity the call acts on: an `interact(0)` resolved to the current
    /// target (None when nothing was targeted yet), a `useItem` or
    /// `useAbility` target of 0 as None; None for a move or a region.
    pub target: Option<u32>,
}

/// One row of the draft, before any table turns it into steps.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedRow {
    pub anchor: Anchor,
    /// Ticks covered: the anchor's tick to the tick before the next anchor (or the last tick).
    pub first_tick: u32,
    pub last_tick: u32,
    /// Calls after the anchor in the row (dialog choices, others), with their
    /// tick. Movement is never kept (D-LR4): not a `gmGotoXYZ`, nor a region
    /// call that did not anchor a row. `setTargetID`, chat and noise are not
    /// kept either.
    pub calls: Vec<(u32, Call)>,
    /// Dot chat lines seen since the previous anchor (setup of this row).
    pub setup_chat: Vec<String>,
    /// Outbound messages in the row: (tick, msg_name, decoded).
    pub server: Vec<(u32, String, Value)>,
    /// Window clicks recorded in the row's ticks (LR-11), with their tick.
    /// Like a call, a click before the first anchor has no row.
    pub clicks: Vec<(u32, UiClick)>,
}

/// One tap row, in log order.
enum Event {
    In(Call),
    Out(String, Value),
}

/// Cut the log into rows. Ticks are walked in order and the tap rows inside
/// a tick in tap order; a tick is numbered by its `seq`.
pub fn segment(log: &RecordLog) -> Vec<RecordedRow> {
    let events = events(log);
    let snapshots: Vec<(u32, &DbSnapshot)> = log
        .ticks
        .iter()
        .filter_map(|t| Some((t.seq, t.snapshot.as_ref()?)))
        .collect();
    let is_anchor = anchors(&events, &snapshots);
    let end_tick = log.ticks.last().map_or(0, |t| t.seq);

    let mut rows: Vec<RecordedRow> = Vec::new();
    let mut current_target: Option<u32> = None;
    let mut chat: Vec<String> = Vec::new();
    for (i, (tick, event)) in events.iter().enumerate() {
        let tick = *tick;
        let call = match event {
            Event::Out(name, decoded) => {
                if let Some(row) = rows.last_mut() {
                    row.server.push((tick, name.clone(), decoded.clone()));
                }
                continue;
            }
            Event::In(call) => call,
        };
        match call {
            Call::SetTarget { target } => current_target = Some(*target),
            Call::GmGoto { .. } | Call::Noise => {}
            Call::Chat { text, .. } => {
                if text.starts_with('.') && !text.starts_with(".bug") {
                    chat.push(text.clone());
                }
            }
            _ => match anchor_kind(call).filter(|_| is_anchor[i]) {
                Some(kind) => {
                    if let Some(prev) = rows.last_mut() {
                        // Two anchors in one tick leave the first a one-tick row.
                        prev.last_tick = tick.saturating_sub(1).max(prev.first_tick);
                    }
                    let target = match call {
                        Call::Interact { target: 0 } => current_target,
                        // A use with no target is like an unresolved interact(0).
                        Call::Interact { target }
                        | Call::UseItem { target, .. }
                        | Call::UseAbility { target, .. } => Some(*target).filter(|t| *t != 0),
                        _ => None,
                    };
                    rows.push(RecordedRow {
                        anchor: Anchor {
                            kind,
                            tick,
                            call: call.clone(),
                            target,
                        },
                        first_tick: tick,
                        last_tick: end_tick.max(tick),
                        calls: Vec::new(),
                        setup_chat: std::mem::take(&mut chat),
                        server: Vec::new(),
                        clicks: Vec::new(),
                    });
                }
                // A region that did not anchor is movement, never a TODO (D-LR4).
                None if matches!(call, Call::Region { .. }) => {}
                // Before the first anchor there is no row to attach it to.
                None => {
                    if let Some(row) = rows.last_mut() {
                        row.calls.push((tick, call.clone()));
                    }
                }
            },
        }
    }
    // A tick two anchors share belongs to the later row, as its calls do.
    for t in &log.ticks {
        let row = rows
            .iter_mut()
            .rev()
            .find(|r| (r.first_tick..=r.last_tick).contains(&t.seq));
        if let Some(row) = row {
            row.clicks
                .extend(t.clicks.iter().map(|c| (t.seq, c.clone())));
        }
    }
    rows
}

/// Every decodable tap row with its tick.
fn events(log: &RecordLog) -> Vec<(u32, Event)> {
    let mut out = Vec::new();
    for tick in &log.ticks {
        for row in &tick.tap {
            match row["dir"].as_str() {
                Some("in") => {
                    if let Some(call) = decode_call(row) {
                        out.push((tick.seq, Event::In(call)));
                    }
                }
                Some("out") => {
                    let name = row["msg_name"].as_str().unwrap_or("unknown").to_string();
                    out.push((tick.seq, Event::Out(name, row["decoded"].clone())));
                }
                _ => {}
            }
        }
    }
    out
}

/// The anchors among one tick's tap rows, as `lab_record_status` counts
/// them while recording: every inbound call that opens a row whatever
/// follows. A region entry is not counted, because it anchors only when a
/// later snapshot changes, which the live count cannot know yet.
pub fn count_anchors(rows: &[Value]) -> u32 {
    let n = rows
        .iter()
        .filter_map(decode_call)
        .filter(|c| anchor_kind(c).is_some_and(|k| k != AnchorKind::Region))
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The anchor kind a call would open, if it can open one at all.
fn anchor_kind(call: &Call) -> Option<AnchorKind> {
    Some(match call {
        Call::Interact { .. } => AnchorKind::Interact,
        Call::UseItem { .. } => AnchorKind::UseItem,
        Call::MoveItem { .. } => AnchorKind::MoveItem,
        Call::UseAbility { .. } => AnchorKind::UseAbility,
        Call::Region { entering: true, .. } => AnchorKind::Region,
        _ => return None,
    })
}

/// Which events open a row. Every candidate does, except a region entry:
/// that one does only when a snapshot from its tick up to (not including)
/// the next candidate's tick differs from the last snapshot before it.
/// Walking into a region that changes nothing is just walking.
///
/// Known limitation: persistence lands a tick or more after the call that
/// caused it (README F5), so an anchor's own DB change can show up in the
/// snapshots after a region entered right behind it, and that region then
/// reads as an anchor too.
fn anchors(events: &[(u32, Event)], snapshots: &[(u32, &DbSnapshot)]) -> Vec<bool> {
    let candidates: Vec<(usize, u32, &Call)> = events
        .iter()
        .enumerate()
        .filter_map(|(i, (tick, e))| match e {
            Event::In(call) if anchor_kind(call).is_some() => Some((i, *tick, call)),
            _ => None,
        })
        .collect();
    let mut is_anchor = vec![false; events.len()];
    for (n, &(i, tick, call)) in candidates.iter().enumerate() {
        is_anchor[i] = match call {
            Call::Region { .. } => {
                let until = candidates.get(n + 1).map_or(u32::MAX, |c| c.1);
                snapshot_changed(snapshots, tick, until)
            }
            _ => true,
        };
    }
    is_anchor
}

/// Whether a snapshot in ticks `from..until` differs from the last snapshot
/// before `from`. With no snapshot before `from`, the first one in the
/// window is the baseline, so the start of a recording is not a change.
fn snapshot_changed(snapshots: &[(u32, &DbSnapshot)], from: u32, until: u32) -> bool {
    let mut window = snapshots
        .iter()
        .filter(|(t, _)| (from..until).contains(t))
        .map(|(_, s)| *s);
    let before = snapshots
        .iter()
        .rev()
        .find(|(t, _)| *t < from)
        .map(|(_, s)| *s);
    let Some(baseline) = before.or_else(|| window.next()) else {
        return false;
    };
    window.any(|s| s != baseline)
}

#[cfg(test)]
#[path = "segment_tests.rs"]
mod tests;
