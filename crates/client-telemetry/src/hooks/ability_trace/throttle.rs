//! Volume control for `client.ability.*` (D-AU5): a burst of 8 then 4 a
//! second per name, with the count of suppressed events on the next one
//! through. A press and its answer are throttled as one unit.
//!
//! A press is decided once, when its `client.ability.press` row arrives,
//! against the bucket of its source. Every later row carrying the same
//! `press_id` (`press_dropped`, `sent`, `sent_seq`) follows that decision,
//! so a press and its answer are either both kept or both dropped. A row
//! with no `press_id` (a router refusal or a send that no press led to),
//! or whose press decision is no longer remembered, goes through its own
//! per-name bucket as before. Suppressed presses are counted on the next
//! press row through; their answers go with them and are not counted
//! again.

use std::collections::{HashMap, VecDeque};

use serde_json::json;

use super::{Out, TARGET_PRESS};
use crate::hooks::entity_trace::Fields;
use crate::hooks::name_throttle::{Decision, NameThrottle};

/// Press decisions remembered. A press's answer arrives within a frame
/// (or, for a ground reticle, when the player places it), so this covers
/// far more than one burst.
pub(crate) const MAX_DECISIONS: usize = 512;

/// The per-name token buckets and the per-press decisions.
#[derive(Debug, Default)]
pub(crate) struct AbilityThrottle {
    table: NameThrottle,
    decisions: HashMap<u32, bool>,
    order: VecDeque<u32>,
}

fn press_id(fields: &Fields) -> Option<u32> {
    fields
        .iter()
        .find(|(k, _)| *k == "press_id")
        .and_then(|(_, v)| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
}

impl AbilityThrottle {
    /// A plain per-name decision, for rows that carry no `press_id` and
    /// build their fields only when admitted (recv, applied, shown).
    pub(crate) fn check_name(&mut self, key: &str, now_ms: u64) -> Decision {
        self.table.check(key, now_ms)
    }

    /// One per-name bucket decision; `suppressed` added when non-zero.
    fn by_name(&mut self, key: &str, mut fields: Fields, now_ms: u64) -> Option<Fields> {
        let Decision::Emit { suppressed } = self.table.check(key, now_ms) else {
            return None;
        };
        if suppressed > 0 {
            fields.push(("suppressed", json!(suppressed)));
        }
        Some(fields)
    }

    fn remember(&mut self, id: u32, kept: bool) {
        if self.decisions.insert(id, kept).is_none() {
            self.order.push_back(id);
            if self.order.len() > MAX_DECISIONS {
                if let Some(old) = self.order.pop_front() {
                    self.decisions.remove(&old);
                }
            }
        }
    }

    /// Decide for one event at monotonic `now_ms`: `None` when suppressed,
    /// else its fields (with `suppressed` on a row that decided by bucket).
    pub(crate) fn admit(&mut self, out: Out, now_ms: u64) -> Option<Fields> {
        let id = press_id(&out.fields);
        match id {
            Some(id) if out.target == TARGET_PRESS => {
                let kept = self.by_name(&out.key, out.fields, now_ms);
                self.remember(id, kept.is_some());
                kept
            }
            Some(id) => match self.decisions.get(&id) {
                Some(true) => Some(out.fields),
                Some(false) => None,
                None => self.by_name(&out.key, out.fields, now_ms),
            },
            None => self.by_name(&out.key, out.fields, now_ms),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{field, TARGET_DROPPED, TARGET_SENT, TARGET_SENT_SEQ};
    use super::*;
    use serde_json::Value;
    use std::collections::HashSet;

    fn out(target: &'static str, key: &str, press: Option<u32>) -> Out {
        Out {
            target,
            level: "info",
            key: key.to_owned(),
            fields: vec![("press_id", press.map(Value::from).unwrap_or(Value::Null))],
        }
    }

    /// D-AU5: eight back to back, then four a second, and the next event
    /// through carries how many were dropped.
    #[test]
    fn the_throttle_is_a_burst_of_eight_then_four_a_second() {
        let mut t = AbilityThrottle::default();
        for i in 0..8 {
            assert!(
                t.admit(out(TARGET_SENT, "sent:x", None), 0).is_some(),
                "burst {i}"
            );
        }
        for _ in 0..5 {
            assert!(t.admit(out(TARGET_SENT, "sent:x", None), 0).is_none());
        }
        let next = t
            .admit(out(TARGET_SENT, "sent:x", None), 250)
            .expect("one token refilled");
        assert_eq!(field(&next, "suppressed"), Some(&json!(5)));
        assert!(t.admit(out(TARGET_SENT, "sent:x", None), 250).is_none());
    }

    /// A storm on one name never silences another.
    #[test]
    fn one_hot_name_does_not_hide_another() {
        let mut t = AbilityThrottle::default();
        for _ in 0..100 {
            t.admit(out(TARGET_SENT, "sent:useAbility", None), 0);
        }
        let other = t.admit(out(TARGET_SENT, "sent:confirmationResponse", None), 0);
        assert!(other.is_some());
        assert_eq!(field(&other.unwrap(), "suppressed"), None);
    }

    /// Under spam, every emitted answer has its emitted press and every
    /// emitted press has its emitted answer. The answers are spread over
    /// several names (two drop reasons and a send), so per-name buckets
    /// alone would let answers through whose press was suppressed, and
    /// suppress answers whose press went out.
    #[test]
    fn a_press_and_its_answer_are_kept_or_dropped_together() {
        let mut t = AbilityThrottle::default();
        let mut presses = HashSet::new();
        let mut answers = HashSet::new();
        let mut suppressed_total = 0;
        for id in 1..=400u32 {
            let now = u64::from(id) * 20; // 50 presses a second
            if let Some(f) = t.admit(out(TARGET_PRESS, "press:hotbar", Some(id)), now) {
                presses.insert(id);
                suppressed_total += field(&f, "suppressed").and_then(Value::as_u64).unwrap_or(0);
            }
            let (target, key) = match id % 3 {
                0 => (TARGET_DROPPED, "press_dropped:not_known"),
                1 => (TARGET_DROPPED, "press_dropped:no_action"),
                _ => (TARGET_SENT, "sent:useAbility"),
            };
            if t.admit(out(target, key, Some(id)), now).is_some() {
                answers.insert(id);
            }
            if target == TARGET_SENT && presses.contains(&id) {
                assert!(
                    t.admit(out(TARGET_SENT_SEQ, "sent_seq:useAbility", Some(id)), now)
                        .is_some(),
                    "sent_seq follows its press {id}"
                );
            }
        }
        assert!(presses.len() < 400, "the spam was throttled");
        assert!(presses.len() > 8, "presses still get through");
        let orphans: Vec<_> = answers.difference(&presses).collect();
        assert!(
            orphans.is_empty(),
            "answers without their press: {orphans:?}"
        );
        let unanswered: Vec<_> = presses.difference(&answers).collect();
        assert!(
            unanswered.is_empty(),
            "presses without their answer: {unanswered:?}"
        );
        // Suppressed presses are counted on the next press row through
        // (those after the last emitted press are still pending).
        assert!(suppressed_total > 0);
        assert!(suppressed_total as usize + presses.len() <= 400);
    }

    /// Rows with no press, or a press decision long forgotten, use their own
    /// bucket.
    #[test]
    fn rows_without_a_remembered_press_use_their_own_bucket() {
        let mut t = AbilityThrottle::default();
        assert!(t
            .admit(out(TARGET_DROPPED, "press_dropped:class_mismatch", None), 0)
            .is_some());
        assert!(t
            .admit(out(TARGET_SENT, "sent:x", Some(999_999)), 0)
            .is_some());
        for id in 0..(MAX_DECISIONS as u32 + 10) {
            t.remember(id, false);
        }
        assert_eq!(t.decisions.len(), MAX_DECISIONS);
        assert!(
            t.admit(out(TARGET_SENT, "sent:y", Some(0)), 0).is_some(),
            "evicted"
        );
        assert!(t
            .admit(
                out(TARGET_SENT, "sent:y", Some(MAX_DECISIONS as u32 + 9)),
                0
            )
            .is_none());
    }
}
