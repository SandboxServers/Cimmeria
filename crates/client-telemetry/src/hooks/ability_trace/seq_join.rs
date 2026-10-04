//! Joining a sent method to the Mercury sequence numbers of the packets
//! that carried it (AB-C1, finding "Getting the Mercury packet sequence").
//!
//! The sequence does not exist when the router runs. It is assigned on the
//! network thread, once per packet, by `FUN_0158bb40` inside `Nub::send`
//! (`0x01582160`). The join therefore runs in three steps:
//!
//! 1. The router hook records each allowlisted send ([`Joiner::note_sent`]).
//! 2. `Channel::send` (`0x01576f90`, main thread) detaches the channel's
//!    bundle (`*(channel+0x28)`) and queues it for the network thread; the
//!    hook tags the bundle pointer with the sends recorded so far, *before*
//!    the original runs, so the network thread can never see an untagged
//!    bundle ([`Joiner::tag_bundle`]). When the original did not detach the
//!    bundle (nothing to send yet) the tag is undone.
//! 3. `Nub::send` on the network thread claims the tag by its bundle
//!    argument, records each value `FUN_0158bb40` returns while it runs,
//!    and reports the range ([`sent_seq_outs`]).
//!
//! A bundle can span several packets, so the join key is a range. The
//! counter is 28 bits (`& 0x0fffffff` at `0x0158bb4c`), so the range can
//! wrap: membership is [`seq_in_range`], never `first <= seq <= last`.

use std::collections::VecDeque;

use serde_json::{json, Value};

use super::{Out, TARGET_SENT_SEQ};

// The DLL reports the range; `SEQ_MASK` and `seq_in_range` are the rule a
// consumer of `client.ability.sent_seq` applies, pinned here by tests.

/// The reliable channel's sequence counter mask.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const SEQ_MASK: u32 = 0x0fff_ffff;

/// Whether `seq` lies in the (possibly wrapped) range `first..=last`.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn seq_in_range(seq: u32, first: u32, last: u32) -> bool {
    (seq.wrapping_sub(first) & SEQ_MASK) <= (last.wrapping_sub(first) & SEQ_MASK)
}

/// One sent method waiting for its packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SentTag {
    /// The `client.ability.sent` row's id.
    pub send_id: u32,
    /// Its press, if one matched.
    pub press_id: Option<u32>,
    /// The method.
    pub method: &'static str,
    /// Its ability, when it has one.
    pub ability_id: Option<i32>,
}

/// Sends not yet detached, kept at most.
pub(crate) const MAX_UNSENT: usize = 64;
/// Detached bundles awaiting the network thread, kept at most.
pub(crate) const MAX_BUNDLES: usize = 32;

/// The shared join state (one per process, behind a mutex).
#[derive(Debug, Default)]
pub(crate) struct Joiner {
    unsent: Vec<SentTag>,
    bundles: VecDeque<(u32, Vec<SentTag>)>,
    evicted: u64,
}

impl Joiner {
    /// The router sent `tag`.
    pub(crate) fn note_sent(&mut self, tag: SentTag) {
        if self.unsent.len() >= MAX_UNSENT {
            self.unsent.remove(0);
            self.evicted += 1;
        }
        self.unsent.push(tag);
    }

    /// Whether any send waits for a bundle (the `Channel::send` hook skips
    /// its reads when none does).
    pub(crate) fn has_unsent(&self) -> bool {
        !self.unsent.is_empty()
    }

    /// Tag `bundle` with every send recorded so far. Returns whether it
    /// tagged anything.
    pub(crate) fn tag_bundle(&mut self, bundle: u32) -> bool {
        if bundle == 0 || self.unsent.is_empty() {
            return false;
        }
        let tags = std::mem::take(&mut self.unsent);
        // A pointer can be reused once its bundle is freed; a stale entry
        // under the same address is replaced, and counted.
        if let Some(i) = self.bundles.iter().position(|(b, _)| *b == bundle) {
            if let Some((_, old)) = self.bundles.remove(i) {
                self.evicted += old.len() as u64;
            }
        }
        if self.bundles.len() >= MAX_BUNDLES {
            if let Some((_, old)) = self.bundles.pop_front() {
                self.evicted += old.len() as u64;
            }
        }
        self.bundles.push_back((bundle, tags));
        true
    }

    /// `Channel::send` returned without detaching `bundle`: its sends wait
    /// for the next one, ahead of anything recorded since.
    pub(crate) fn untag_bundle(&mut self, bundle: u32) {
        if let Some(i) = self.bundles.iter().position(|(b, _)| *b == bundle) {
            if let Some((_, mut tags)) = self.bundles.remove(i) {
                tags.append(&mut self.unsent);
                self.unsent = tags;
            }
        }
    }

    /// `Nub::send` took `bundle`: claim its sends.
    pub(crate) fn take_bundle(&mut self, bundle: u32) -> Option<Vec<SentTag>> {
        let i = self.bundles.iter().position(|(b, _)| *b == bundle)?;
        self.bundles.remove(i).map(|(_, t)| t)
    }

    /// Sends dropped from the tables unmatched since the last call.
    pub(crate) fn take_evicted(&mut self) -> u64 {
        std::mem::take(&mut self.evicted)
    }
}

/// `client.ability.sent_seq` for each send carried by one bundle. `seqs`
/// are the values `FUN_0158bb40` returned during that `Nub::send`, in
/// order; empty when the bundle went out without the reliable counter.
pub(crate) fn sent_seq_outs(tags: &[SentTag], seqs: &[u32], evicted: u64) -> Vec<Out> {
    let first = seqs.first().copied();
    let last = seqs.last().copied();
    tags.iter()
        .enumerate()
        .map(|(i, t)| {
            let mut f = vec![
                ("send_id", json!(t.send_id)),
                (
                    "press_id",
                    t.press_id.map(Value::from).unwrap_or(Value::Null),
                ),
                ("method", json!(t.method)),
                (
                    "ability_id",
                    t.ability_id.map(Value::from).unwrap_or(Value::Null),
                ),
                (
                    "mercury_seq_first",
                    first.map(Value::from).unwrap_or(Value::Null),
                ),
                (
                    "mercury_seq_last",
                    last.map(Value::from).unwrap_or(Value::Null),
                ),
                ("packets", json!(seqs.len())),
                ("seq_bits", json!(28)),
            ];
            if i == 0 && evicted > 0 {
                f.push(("evicted_unmatched", json!(evicted)));
            }
            Out {
                target: TARGET_SENT_SEQ,
                level: "info",
                key: format!("{TARGET_SENT_SEQ}:{}", t.method),
                fields: f,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::field;
    use super::*;

    fn tag(id: u32) -> SentTag {
        SentTag {
            send_id: id,
            press_id: Some(100 + id),
            method: "useAbility",
            ability_id: Some(597),
        }
    }

    #[test]
    fn membership_is_modular_across_the_28_bit_wrap() {
        // No wrap.
        assert!(seq_in_range(10, 10, 12));
        assert!(seq_in_range(12, 10, 12));
        assert!(!seq_in_range(13, 10, 12));
        assert!(!seq_in_range(9, 10, 12));
        // Wrapped: 0x0ffffffe, 0x0fffffff, 0, 1.
        let (first, last) = (0x0fff_fffe, 1);
        for s in [0x0fff_fffe, 0x0fff_ffff, 0, 1] {
            assert!(seq_in_range(s, first, last), "{s:#x}");
        }
        assert!(!seq_in_range(2, first, last));
        assert!(!seq_in_range(0x0fff_fffd, first, last));
        // The naive test gets the wrapped case wrong.
        assert!(!(first..=last).contains(&0u32));
    }

    #[test]
    fn a_tagged_bundle_is_claimed_once_with_its_sends() {
        let mut j = Joiner::default();
        assert!(!j.tag_bundle(0x5000), "nothing to tag");
        j.note_sent(tag(1));
        j.note_sent(tag(2));
        assert!(j.tag_bundle(0x5000));
        assert!(!j.has_unsent());
        let got = j.take_bundle(0x5000).expect("tagged");
        assert_eq!(got.iter().map(|t| t.send_id).collect::<Vec<_>>(), [1, 2]);
        assert!(j.take_bundle(0x5000).is_none(), "claimed once");
    }

    /// `Channel::send` that did not detach its bundle puts the sends back,
    /// ahead of later ones.
    #[test]
    fn an_undetached_bundle_returns_its_sends_in_order() {
        let mut j = Joiner::default();
        j.note_sent(tag(1));
        assert!(j.tag_bundle(0x5000));
        j.untag_bundle(0x5000);
        j.note_sent(tag(2));
        assert!(j.tag_bundle(0x6000));
        let got = j.take_bundle(0x6000).unwrap();
        assert_eq!(got.iter().map(|t| t.send_id).collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn the_tables_are_bounded_and_count_what_they_evict() {
        let mut j = Joiner::default();
        for i in 0..(MAX_UNSENT as u32 + 2) {
            j.note_sent(tag(i));
        }
        assert_eq!(j.take_evicted(), 2);
        for b in 0..(MAX_BUNDLES as u32 + 1) {
            j.note_sent(tag(b));
            j.tag_bundle(0x1_0000 + b);
        }
        assert!(j.take_evicted() > 0, "oldest bundle evicted");
        assert!(j.take_bundle(0x1_0000).is_none());
    }

    #[test]
    fn the_seq_row_carries_the_range_and_packet_count() {
        let outs = sent_seq_outs(&[tag(7)], &[0x0fff_ffff, 0, 1], 3);
        assert_eq!(outs.len(), 1);
        let f = &outs[0].fields;
        assert_eq!(field(f, "send_id"), Some(&json!(7)));
        assert_eq!(field(f, "press_id"), Some(&json!(107)));
        assert_eq!(field(f, "mercury_seq_first"), Some(&json!(0x0fff_ffff)));
        assert_eq!(field(f, "mercury_seq_last"), Some(&json!(1)));
        assert_eq!(field(f, "packets"), Some(&json!(3)));
        assert_eq!(field(f, "evicted_unmatched"), Some(&json!(3)));
        let none = sent_seq_outs(&[tag(8)], &[], 0);
        assert_eq!(
            field(&none[0].fields, "mercury_seq_first"),
            Some(&Value::Null)
        );
        assert_eq!(field(&none[0].fields, "evicted_unmatched"), None);
    }
}
