//! The channel's open fragment group, and what `processPacket`
//! (`0x0157fd20`) did with one fragment.
//!
//! The client keeps **one** fragment group per channel at `channel+0x124`
//! (`+0` `lastFrag`, `+4` fragments still missing, `+0x10` a seq-sorted
//! packet list linked through `Packet+8`). The detour reads the group before
//! the call and after it; [`classify`] turns the two views into the outcome
//! the client's log strings would have named (its logger is a stub in this
//! build, so nothing else reports them). See the findings doc, "Fragment
//! reassembly".

use serde_json::json;

use super::tail::Tail;
use super::{channel, group, packet, Fields, MAX_CHAIN};
use crate::hooks::entity_trace::map::Mem;

/// Longest list of held sequence numbers put in an event.
const MAX_SEQS_REPORTED: usize = 32;

/// The open group of a channel at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupView {
    /// Address of the group object (an identity, not dereferenced later).
    pub ptr: u32,
    /// `lastFrag` of the bundle being collected.
    pub last: u32,
    /// Fragments still missing (the client's counter, signed).
    pub remaining: i32,
    /// Sequence numbers of the packets held, in list (sorted) order.
    pub held_seqs: Vec<u32>,
    /// Payload bytes held (`len - 1` per packet).
    pub held_bytes: usize,
}

/// Read the group of `channel`: `None` if a read failed, `Some(None)` if the
/// channel has no open group.
pub(crate) fn read_group(mem: &dyn Mem, channel_ptr: u32) -> Option<Option<GroupView>> {
    let ptr = mem.u32_at(channel_ptr.wrapping_add(channel::FRAG_GROUP))?;
    if ptr == 0 {
        return Some(None);
    }
    let last = mem.u32_at(ptr.wrapping_add(group::LAST))?;
    let remaining = mem.u32_at(ptr.wrapping_add(group::REMAINING))? as i32;
    let mut node = mem.u32_at(ptr.wrapping_add(group::LIST))?;
    let mut held_seqs = Vec::new();
    let mut held_bytes = 0usize;
    while node != 0 && held_seqs.len() < MAX_CHAIN {
        held_seqs.push(mem.u32_at(node.wrapping_add(packet::SEQ))?);
        let len = mem.u32_at(node.wrapping_add(packet::LEN))? as usize;
        held_bytes += len.saturating_sub(1);
        node = mem.u32_at(node.wrapping_add(packet::NEXT))?;
    }
    Some(Some(GroupView {
        ptr,
        last,
        remaining,
        held_seqs,
        held_bytes,
    }))
}

/// What became of one fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// First fragment seen: a group was opened for `expected` fragments.
    Started { expected: i32, remaining: i32 },
    /// Added to the open group; `remaining` are still missing.
    Added { remaining: i32 },
    /// The last missing fragment arrived: the group was freed and its list
    /// queued as one bundle.
    Completed {
        packets: usize,
        bytes: usize,
        expected: i32,
    },
    /// A fragment with this `seq` was already in the group; discarded.
    Duplicate,
    /// Reliable fragment whose `lastFrag` differs from the open group's:
    /// dropped for good (it was already ACKed), group left open.
    MangledFooters { open_last: u32 },
    /// No group and this is not the first fragment: dropped
    /// (`"Bundle (#%d,#%d) is missing"`).
    BundleMissing,
    /// The open group was stale or headed by an unreliable packet, freed,
    /// and a new group opened for this fragment.
    Restarted { expected: i32, remaining: i32 },
    /// The open group vanished without completing (stale discard with no
    /// replacement).
    Discarded,
    /// Fewer than two fragments declared, or too few footer bytes.
    IllegalFooters,
    /// The packet is not on a channel: its group lives in the nub's hash
    /// table, which this trace does not read.
    NoChannel,
    /// Neither view was readable, or the change fits no known path.
    Unknown,
}

impl Outcome {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Outcome::Started { .. } => "group_started",
            Outcome::Added { .. } => "added",
            Outcome::Completed { .. } => "completed",
            Outcome::Duplicate => "duplicate",
            Outcome::MangledFooters { .. } => "mangled_footers",
            Outcome::BundleMissing => "bundle_missing",
            Outcome::Restarted { .. } => "group_restarted",
            Outcome::Discarded => "group_discarded",
            Outcome::IllegalFooters => "illegal_footers",
            Outcome::NoChannel => "no_channel",
            Outcome::Unknown => "unknown",
        }
    }

    /// The fragment is in the group (or the group is done).
    pub(crate) fn is_happy(&self) -> bool {
        matches!(
            self,
            Outcome::Started { .. } | Outcome::Added { .. } | Outcome::Completed { .. }
        )
    }
}

/// Classify one fragment. `tail` is the fragment's parsed footers, `pre` and
/// `post` the channel's group before and after `processPacket`, `result` its
/// return value. A completing fragment's own payload is `tail.payload_len`.
pub(crate) fn classify(
    tail: &Tail,
    pre: Option<&GroupView>,
    post: Option<&GroupView>,
    result: i32,
) -> Outcome {
    let Some((first, last)) = tail.fragment else {
        // Too few footer bytes for the fragment ids.
        return if tail.fault.is_some() {
            Outcome::IllegalFooters
        } else {
            Outcome::Unknown
        };
    };
    let expected = tail.fragment_count().unwrap_or(0);
    if expected < 2 {
        return Outcome::IllegalFooters;
    }
    match (pre, post) {
        (None, Some(g)) if g.last == last => Outcome::Started {
            expected,
            remaining: g.remaining,
        },
        (None, None) => {
            if tail.seq.is_some_and(|s| s != first) {
                Outcome::BundleMissing
            } else {
                Outcome::Unknown
            }
        }
        (None, Some(_)) => Outcome::Unknown,
        (Some(p), _) if p.last != last => Outcome::MangledFooters { open_last: p.last },
        (Some(p), None) => {
            if p.remaining <= 1 {
                Outcome::Completed {
                    packets: p.held_seqs.len() + 1,
                    bytes: p.held_bytes + tail.payload_len,
                    expected,
                }
            } else if result != 0 {
                Outcome::BundleMissing
            } else {
                Outcome::Discarded
            }
        }
        (Some(p), Some(g)) => {
            if g.remaining > p.remaining || (g.ptr != p.ptr && g.remaining >= p.remaining) {
                Outcome::Restarted {
                    expected,
                    remaining: g.remaining,
                }
            } else if g.remaining < p.remaining {
                Outcome::Added {
                    remaining: g.remaining,
                }
            } else {
                Outcome::Duplicate
            }
        }
    }
}

/// The fields of a `client.mercury.fragment` event.
pub(crate) fn fields(
    tail: &Tail,
    outcome: &Outcome,
    pre: Option<&GroupView>,
    post: Option<&GroupView>,
) -> Fields {
    let mut f: Fields = vec![("outcome", json!(outcome.name()))];
    if let Some(s) = tail.seq {
        f.push(("seq", json!(s)));
    }
    if let Some((first, last)) = tail.fragment {
        f.push(("frag_first", json!(first)));
        f.push(("frag_last", json!(last)));
    }
    if let Some(c) = tail.fragment_count() {
        f.push(("expected_fragments", json!(c)));
    }
    f.push(("payload_len", json!(tail.payload_len)));
    f.push(("reliable", json!(tail.reliable())));
    if let Some(p) = pre {
        f.push(("open_group_last", json!(p.last)));
        f.push(("remaining_before", json!(p.remaining)));
        f.push(("held_before", json!(p.held_seqs.len())));
        f.push(("held_bytes_before", json!(p.held_bytes)));
        // The held sequence numbers are what a missing fragment shows up
        // against; report them when the fragment did not simply join.
        if !outcome.is_happy() {
            let seqs: Vec<u32> = p
                .held_seqs
                .iter()
                .copied()
                .take(MAX_SEQS_REPORTED)
                .collect();
            f.push(("held_seqs", json!(seqs)));
        }
    }
    if let Some(g) = post {
        f.push(("remaining_after", json!(g.remaining)));
        f.push(("held_after", json!(g.held_seqs.len())));
        f.push(("held_bytes_after", json!(g.held_bytes)));
    }
    match outcome {
        Outcome::Started {
            expected,
            remaining,
        }
        | Outcome::Restarted {
            expected,
            remaining,
        } => {
            f.push(("expected", json!(expected)));
            f.push(("remaining", json!(remaining)));
        }
        Outcome::Completed {
            packets,
            bytes,
            expected,
        } => {
            f.push(("assembled_packets", json!(packets)));
            f.push(("assembled_bytes", json!(bytes)));
            f.push(("expected", json!(expected)));
            f.push((
                "count_matches",
                json!(i64::try_from(*packets).ok() == Some(i64::from(*expected))),
            ));
        }
        Outcome::MangledFooters { open_last } => f.push(("open_group_last", json!(open_last))),
        _ => {}
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::entity_trace::map::fake::FakeMem;
    use crate::hooks::mercury_recv::tail::parse;

    fn frag_tail(seq: u32, first: u32, last: u32, payload: usize) -> Tail {
        let mut d = vec![0x78u8];
        d.extend(std::iter::repeat_n(0u8, payload));
        d.extend(first.to_le_bytes());
        d.extend(last.to_le_bytes());
        d.extend(seq.to_le_bytes());
        parse(&d)
    }

    fn view(ptr: u32, last: u32, remaining: i32, seqs: &[u32], bytes: usize) -> GroupView {
        GroupView {
            ptr,
            last,
            remaining,
            held_seqs: seqs.to_vec(),
            held_bytes: bytes,
        }
    }

    #[test]
    fn the_first_fragment_opens_a_group() {
        let t = frag_tail(70, 70, 84, 1400);
        let post = view(0x9000, 84, 14, &[70], 1400);
        assert_eq!(
            classify(&t, None, Some(&post), 0),
            Outcome::Started {
                expected: 15,
                remaining: 14
            }
        );
    }

    #[test]
    fn a_middle_fragment_is_added_and_the_last_completes_the_bundle() {
        let t = frag_tail(71, 70, 84, 1400);
        let pre = view(0x9000, 84, 14, &[70], 1400);
        let post = view(0x9000, 84, 13, &[70, 71], 2800);
        assert_eq!(
            classify(&t, Some(&pre), Some(&post), 0),
            Outcome::Added { remaining: 13 }
        );
        let t = frag_tail(84, 70, 84, 967);
        let pre = view(0x9000, 84, 1, &(70..84).collect::<Vec<_>>(), 17400);
        assert_eq!(
            classify(&t, Some(&pre), None, 0),
            Outcome::Completed {
                packets: 15,
                bytes: 17400 + 967,
                expected: 15
            }
        );
    }

    /// The reliable-drop shape: another bundle's fragment while a group is
    /// open. It was ACKed, so nothing will resend it.
    #[test]
    fn a_fragment_of_another_bundle_is_dropped_and_the_group_kept() {
        let t = frag_tail(90, 90, 95, 1000);
        let pre = view(0x9000, 84, 10, &[70, 71, 72, 73], 5600);
        let post = pre.clone();
        let o = classify(&t, Some(&pre), Some(&post), -4);
        assert_eq!(o, Outcome::MangledFooters { open_last: 84 });
        assert!(!o.is_happy());
    }

    #[test]
    fn a_repeat_of_a_held_fragment_is_a_duplicate() {
        let t = frag_tail(71, 70, 84, 1400);
        let pre = view(0x9000, 84, 12, &[70, 71, 72], 4200);
        assert_eq!(
            classify(&t, Some(&pre), Some(&pre.clone()), 0),
            Outcome::Duplicate
        );
    }

    #[test]
    fn a_fragment_with_no_group_and_not_first_is_bundle_missing() {
        let t = frag_tail(75, 70, 84, 1400);
        assert_eq!(classify(&t, None, None, -4), Outcome::BundleMissing);
    }

    #[test]
    fn a_group_freed_and_reopened_is_a_restart_even_if_the_allocator_reuses_the_address() {
        let t = frag_tail(70, 70, 84, 1400);
        let pre = view(0x9000, 84, 3, &[70, 71, 72], 100);
        let post = view(0x9000, 84, 14, &[70], 1400);
        assert_eq!(
            classify(&t, Some(&pre), Some(&post), 0),
            Outcome::Restarted {
                expected: 15,
                remaining: 14
            }
        );
    }

    #[test]
    fn a_single_fragment_bundle_is_illegal() {
        let t = frag_tail(5, 5, 5, 10);
        assert_eq!(classify(&t, None, None, -4), Outcome::IllegalFooters);
    }

    #[test]
    fn the_group_is_read_through_the_list_links() {
        let mut m = FakeMem::default();
        let (ch, g, p1, p2) = (0x1000u32, 0x2000u32, 0x3000u32, 0x4000u32);
        m.set(ch + channel::FRAG_GROUP, g);
        m.set(g + group::LAST, 84);
        m.set(g + group::REMAINING, 12);
        m.set(g + group::LIST, p1);
        m.set(p1 + packet::SEQ, 70);
        m.set(p1 + packet::LEN, 1401);
        m.set(p1 + packet::NEXT, p2);
        m.set(p2 + packet::SEQ, 71);
        m.set(p2 + packet::LEN, 1401);
        m.set(p2 + packet::NEXT, 0);
        let v = read_group(&m, ch).unwrap().unwrap();
        assert_eq!(v.held_seqs, vec![70, 71]);
        assert_eq!(v.held_bytes, 2800);
        assert_eq!((v.last, v.remaining), (84, 12));
        // No group, and an unreadable channel, are different answers.
        m.set(ch + channel::FRAG_GROUP, 0);
        assert_eq!(read_group(&m, ch), Some(None));
        assert_eq!(read_group(&m, 0x7000), None);
    }

    #[test]
    fn a_looping_list_is_cut_off() {
        let mut m = FakeMem::default();
        let (ch, g, p) = (0x1000u32, 0x2000u32, 0x3000u32);
        m.set(ch + channel::FRAG_GROUP, g);
        m.set(g + group::LAST, 9);
        m.set(g + group::REMAINING, 1);
        m.set(g + group::LIST, p);
        m.set(p + packet::SEQ, 1);
        m.set(p + packet::LEN, 10);
        m.set(p + packet::NEXT, p);
        let v = read_group(&m, ch).unwrap().unwrap();
        assert_eq!(v.held_seqs.len(), MAX_CHAIN);
    }
}
