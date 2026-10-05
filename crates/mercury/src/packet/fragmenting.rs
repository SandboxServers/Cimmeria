//! Message-aware fragment planning for server-to-client bundles.
//!
//! A bundle body larger than [`FRAGMENT_BODY_SIZE`] is cut into per-packet
//! chunks. The client reads the bundle back with `Bundle::iterator::unpack`
//! (`ghidra://SGW.exe@0x01579830`), which reads a message's id and length
//! field from ONE packet: when `packet_end < header_len + offset` it logs
//! `Error unpacking header length` and the caller
//! (`Nub::processOrderedPacket`, `0x0157c820`) discards the rest of the
//! bundle as "corrupted header". A message BODY may span packets (the
//! iterator's `data()` at `0x01579a50` copies across them), but a HEADER may
//! not. The original server enforced the same rule with `expandAtomic`
//! ("BW has trouble unpacking bundles where header fields are in different
//! packets", `deprecated/cpp/src/mercury/bundle.cpp`).
//!
//! A raw 1300-byte split ignores that, so about one cut in ten lands inside a
//! `WORD_LENGTH` header (message id + `u16` length) and the client drops every
//! message after it. [`plan_fragments`] walks the body with the server-to-
//! client message table and moves a cut that would land inside a header back
//! to the message start.
//!
//! The walk stops at the first message it cannot frame (unknown id, length
//! overrunning the body); the rest of the body is then cut raw, exactly as
//! before, so an opaque test body still fragments byte-for-byte.

use std::ops::Range;

use super::FRAGMENT_BODY_SIZE;

/// Framing of one server-to-client message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMessageFraming {
    /// Fixed payload of this many bytes; the header is the id byte alone.
    Constant(usize),
    /// `u16` LE length prefix; the header is 3 bytes (id + length).
    Word,
}

/// Framing of a server-to-client message id, `None` for an id that is not in
/// the client's interface table (`0x38..=0x7E`).
///
/// Ids `0x80..=0xFE` are entity method calls, always `WORD_LENGTH`. Static
/// ids follow the client's `InterfaceElementVec`, ported from
/// `tools/pcap_dissect.py` (`SERVER_MSG_FORMAT`).
pub fn server_message_framing(msg_id: u8) -> Option<ServerMessageFraming> {
    use ServerMessageFraming::{Constant, Word};
    Some(match msg_id {
        0x00 => Word,         // authenticate
        0x01 => Constant(4),  // bandwidthNotification
        0x02 => Constant(1),  // updateFrequencyNotification
        0x03 => Constant(4),  // setGameTime
        0x04 => Constant(1),  // resetEntities
        0x05 => Word,         // createBasePlayer
        0x06 => Word,         // createCellPlayer
        0x07 => Word,         // spaceData
        0x08 => Constant(13), // spaceViewportInfo
        0x09 => Word,         // createEntity
        0x0A => Word,         // updateEntity
        0x0B => Constant(5),  // entityInvisible
        0x0C => Word,         // leaveAoI
        0x0D => Constant(8),  // tickSync
        0x0E => Constant(1),  // setSpaceViewport
        0x0F => Constant(4),  // setVehicle
        // avatarUpdate family (0x10..=0x2F).
        0x10 | 0x14 | 0x18 | 0x20 | 0x24 | 0x28 => Constant(25),
        0x11 | 0x15 | 0x19 | 0x21 | 0x25 | 0x29 => Constant(24),
        0x12 | 0x16 | 0x1A | 0x22 | 0x26 | 0x2A => Constant(23),
        0x13 | 0x17 | 0x1B | 0x23 | 0x27 | 0x2B => Constant(22),
        0x1C | 0x2C => Constant(13),
        0x1D | 0x2D => Constant(12),
        0x1E | 0x2E => Constant(11),
        0x1F | 0x2F => Constant(10),
        0x30 => Constant(41), // detailedPosition
        0x31 => Constant(49), // forcedPosition
        0x32 => Constant(5),  // controlEntity
        0x33 => Word,         // voiceData
        0x34 => Word,         // restoreClient
        0x35 => Word,         // restoreBaseApp
        0x36 => Word,         // resourceFragment
        0x37 => Constant(1),  // loggedOff
        0x80..=0xFF => Word,  // entity methods, connectReply (0xFF)
        _ => return None,
    })
}

/// Where a body is cut into packets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentPlan {
    /// One body range per packet, in order, covering the whole body.
    pub ranges: Vec<Range<usize>>,
    /// Cuts moved back to a message start because the raw cut fell inside a
    /// message header.
    pub header_guarded_cuts: usize,
    /// The message each packet starts in, by packet index, recorded from
    /// the same framing walk as the cuts. Empty for a one-packet plan: that
    /// packet starts at a message boundary, so its head is read from its own
    /// bytes when needed ([`super::first_message_head`]).
    pub heads: Vec<Option<super::MessageHead>>,
}

impl FragmentPlan {
    /// Body bytes of each packet.
    pub fn packet_sizes(&self) -> Vec<usize> {
        self.ranges.iter().map(|r| r.len()).collect()
    }
}

/// One framing walk of a body from the front.
struct Walk {
    /// Cut positions a fragment boundary must avoid: `start + 1` and
    /// `start + 2` of every `WORD_LENGTH` message.
    forbidden: Vec<usize>,
    /// The start of every framed message.
    starts: Vec<usize>,
    /// Where the walk stopped: the body end, or the first message it could
    /// not frame.
    framed_end: usize,
}

fn walk(body: &[u8]) -> Walk {
    let mut forbidden = Vec::new();
    let mut starts = Vec::new();
    let mut pos = 0usize;
    while pos < body.len() {
        let Some(end) = super::message_head::message_end(body, pos) else {
            break;
        };
        if server_message_framing(body[pos]) == Some(ServerMessageFraming::Word) {
            forbidden.push(pos + 1);
            forbidden.push(pos + 2);
        }
        starts.push(pos);
        pos = end;
    }
    Walk {
        forbidden,
        starts,
        framed_end: pos,
    }
}

/// Plan the packet cuts for `body`. A body of at most [`FRAGMENT_BODY_SIZE`]
/// is one range.
pub fn plan_fragments(body: &[u8]) -> FragmentPlan {
    if body.len() <= FRAGMENT_BODY_SIZE {
        return FragmentPlan {
            ranges: std::iter::once(0..body.len()).collect(),
            header_guarded_cuts: 0,
            heads: Vec::new(),
        };
    }
    let walk = walk(body);
    let forbidden = &walk.forbidden;
    let mut ranges = Vec::with_capacity(body.len().div_ceil(FRAGMENT_BODY_SIZE));
    let mut guarded = 0usize;
    let mut start = 0usize;
    while body.len() - start > FRAGMENT_BODY_SIZE {
        let mut cut = start + FRAGMENT_BODY_SIZE;
        // At most two steps: the forbidden run is `s + 1, s + 2`.
        while forbidden.binary_search(&cut).is_ok() {
            cut -= 1;
            guarded += 1;
        }
        ranges.push(start..cut);
        start = cut;
    }
    ranges.push(start..body.len());
    let heads = super::message_head::heads_for_ranges(body, &walk.starts, walk.framed_end, &ranges);
    FragmentPlan {
        ranges,
        header_guarded_cuts: guarded,
        heads,
    }
}

/// Number of packets `body` fragments into. Always agrees with
/// [`plan_fragments`], so a sequence reservation made from it matches what
/// `build_fragmented_bundle` emits.
pub fn fragment_count(body: &[u8]) -> usize {
    plan_fragments(body).ranges.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One entity-method message: `[0x80|idx][len u16][payload of len bytes]`.
    fn word_msg(payload_len: usize) -> Vec<u8> {
        let mut m = vec![0x85];
        m.extend_from_slice(&(payload_len as u16).to_le_bytes());
        m.extend(std::iter::repeat_n(0x11u8, payload_len));
        m
    }

    #[test]
    fn small_body_is_one_range() {
        let plan = plan_fragments(&word_msg(100));
        assert_eq!(plan.ranges, vec![0..103]);
    }

    #[test]
    fn opaque_body_cuts_raw_like_before() {
        let body = vec![0xAAu8; FRAGMENT_BODY_SIZE * 3 - 100];
        let plan = plan_fragments(&body);
        assert_eq!(plan.ranges.len(), 3);
        assert_eq!(plan.ranges[0], 0..FRAGMENT_BODY_SIZE);
        assert_eq!(plan.header_guarded_cuts, 0);
    }

    /// Messages of `47` bytes total: the raw cut at 1300 falls at offset
    /// 1300 % 47 == 31 in a message, not in a header, so the plan is raw.
    #[test]
    fn cut_inside_a_body_is_kept() {
        let mut body = Vec::new();
        while body.len() < FRAGMENT_BODY_SIZE * 2 {
            body.extend(word_msg(44));
        }
        let plan = plan_fragments(&body);
        assert_eq!(plan.ranges[0], 0..FRAGMENT_BODY_SIZE);
        assert_eq!(plan.header_guarded_cuts, 0);
    }

    /// Filler so the next message header starts at offset `at`.
    fn body_with_header_at(at: usize) -> Vec<u8> {
        // A single filler message occupying [0, at): header 3 + payload.
        let mut body = word_msg(at - 3);
        // Then enough messages to need three packets.
        while body.len() < FRAGMENT_BODY_SIZE * 2 + 500 {
            body.extend(word_msg(60));
        }
        body
    }

    #[test]
    fn cut_one_byte_into_a_header_moves_back_to_the_message_start() {
        let body = body_with_header_at(FRAGMENT_BODY_SIZE - 1);
        let plan = plan_fragments(&body);
        // Header at 1299: the raw cut 1300 is header + 1.
        assert_eq!(plan.ranges[0], 0..FRAGMENT_BODY_SIZE - 1);
        assert_eq!(plan.header_guarded_cuts, 1);
    }

    #[test]
    fn cut_two_bytes_into_a_header_moves_back_to_the_message_start() {
        let body = body_with_header_at(FRAGMENT_BODY_SIZE - 2);
        let plan = plan_fragments(&body);
        assert_eq!(plan.ranges[0], 0..FRAGMENT_BODY_SIZE - 2);
        // 1300 and 1299 are both inside the header: two steps back.
        assert_eq!(plan.header_guarded_cuts, 2);
    }

    #[test]
    fn cut_at_a_message_start_or_after_the_header_is_kept() {
        for at in [FRAGMENT_BODY_SIZE, FRAGMENT_BODY_SIZE - 3] {
            let plan = plan_fragments(&body_with_header_at(at));
            assert_eq!(plan.ranges[0], 0..FRAGMENT_BODY_SIZE, "header at {at}");
            assert_eq!(plan.header_guarded_cuts, 0);
        }
    }

    #[test]
    fn constant_length_header_is_one_byte_and_never_guarded() {
        // resetEntities (0x04, Constant(1)): its 1-byte header sits at 1299,
        // so the raw cut at 1300 lands right after it.
        let mut body = word_msg(FRAGMENT_BODY_SIZE - 3 - 1);
        body.extend([0x04, 0x00]);
        while body.len() < FRAGMENT_BODY_SIZE * 2 + 500 {
            body.extend(word_msg(60));
        }
        let plan = plan_fragments(&body);
        assert_eq!(plan.ranges[0], 0..FRAGMENT_BODY_SIZE);
        assert_eq!(plan.header_guarded_cuts, 0);
    }

    #[test]
    fn ranges_tile_the_body_and_fragment_count_agrees() {
        let mut body = Vec::new();
        for i in 0..400usize {
            body.extend(word_msg(20 + (i * 7) % 90));
        }
        let plan = plan_fragments(&body);
        assert_eq!(plan.ranges.first().unwrap().start, 0);
        assert_eq!(plan.ranges.last().unwrap().end, body.len());
        for w in plan.ranges.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert!(plan.ranges.iter().all(|r| r.len() <= FRAGMENT_BODY_SIZE));
        assert_eq!(fragment_count(&body), plan.ranges.len());
    }
}
