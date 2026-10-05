//! The message a server-to-client packet's body starts in, for the
//! transmit-hole watchdog.
//!
//! A packet whose body starts at a message boundary (every unfragmented
//! packet, and the first fragment of a bundle) is named at stall time from
//! its own retained bytes: [`first_message_head`] reads the head at offset 0.
//! A later fragment starts inside the bundle's message stream, so its head
//! comes from [`super::plan_fragments`], which frames the body for its
//! header guard anyway and records each fragment's head from that same walk
//! ([`heads_for_ranges`]). A pre-composed blob of several messages
//! (`createEntity` plus its avatar update, a player-ghost cascade) is framed
//! message by message, so a fragment inside it reports its own message.

use std::ops::Range;

use super::fragmenting::{server_message_framing, ServerMessageFraming};

/// The extended-encoding marker of an entity method at or past idbase.
const EXTENDED_MARKER: u8 = 0xBD;
/// The idbase of SGWPlayer: an `0xBD` method is index `61 + sub_index`.
/// No NPC type has a method at 61, so the player reading is the only one.
const PLAYER_IDBASE: u16 = 61;

/// The header of one message in a bundle body, unnamed: the transport has no
/// method tables, so a caller with `cimmeria_wire::names` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageHead {
    /// Byte offset of the message's id byte in the body.
    pub offset: usize,
    /// The message id byte (`0xBD` for an extended entity method).
    pub msg_id: u8,
    /// The entity an entity method (0x80-0xFE) is about.
    pub entity_id: Option<u32>,
    /// The flat client-method index of an entity method.
    pub method_index: Option<u16>,
}

impl MessageHead {
    fn read(body: &[u8], offset: usize) -> Self {
        let msg_id = body[offset];
        let is_method = (0x80..=0xFE).contains(&msg_id);
        let entity_id = is_method
            .then(|| body.get(offset + 3..offset + 7))
            .flatten()
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let method_index = match msg_id {
            EXTENDED_MARKER => body
                .get(offset + 7)
                .map(|sub| PLAYER_IDBASE + u16::from(*sub)),
            0x80..=0xBC => Some(u16::from(msg_id - 0x80)),
            _ => None,
        };
        Self {
            offset,
            msg_id,
            entity_id,
            method_index,
        }
    }
}

/// The head of the message at offset 0 of a body that starts at a message
/// boundary, if it frames.
pub fn first_message_head(body: &[u8]) -> Option<MessageHead> {
    (!body.is_empty() && message_end(body, 0).is_some()).then(|| MessageHead::read(body, 0))
}

/// The head of the message each range starts in, from a walk's message
/// `starts` (ascending) that framed the body up to `framed_end`. A range
/// starting past `framed_end` (behind a message the walk could not frame)
/// gets `None`.
pub(super) fn heads_for_ranges(
    body: &[u8],
    starts: &[usize],
    framed_end: usize,
    ranges: &[Range<usize>],
) -> Vec<Option<MessageHead>> {
    ranges
        .iter()
        .map(|r| {
            if r.start >= framed_end {
                return None;
            }
            let at = starts.partition_point(|&s| s <= r.start).checked_sub(1)?;
            Some(MessageHead::read(body, starts[at]))
        })
        .collect()
}

/// The offset just past the message starting at `pos`, if it frames.
pub(super) fn message_end(body: &[u8], pos: usize) -> Option<usize> {
    let end = match server_message_framing(body[pos])? {
        ServerMessageFraming::Constant(len) => pos + 1 + len,
        ServerMessageFraming::Word => {
            let len = body.get(pos + 1..pos + 3)?;
            pos + 3 + usize::from(u16::from_le_bytes([len[0], len[1]]))
        }
    };
    (end <= body.len()).then_some(end)
}

#[cfg(test)]
mod tests {
    use super::super::{plan_fragments, FRAGMENT_BODY_SIZE};
    use super::*;

    /// `createEntity` then its avatar update, as one pre-composed blob, then
    /// a run of extended methods (index 68 = 61 + 7 on entity 42), long
    /// enough to fragment.
    fn blob() -> Vec<u8> {
        let mut body = vec![0x09, 4, 0, 1, 2, 3, 4]; // createEntity, 7 bytes
        body.push(0x10); // avatarUpdateNoAliasFullPosYawPitchRoll, 1 + 25
        body.extend_from_slice(&[0; 25]);
        while body.len() <= FRAGMENT_BODY_SIZE * 2 {
            // [0xBD][len = 5 + 40][entity_id = 42][sub_index = 7][40 bytes]
            body.extend_from_slice(&[0xBD, 45, 0, 42, 0, 0, 0, 7]);
            body.extend_from_slice(&[0; 40]);
        }
        body
    }

    #[test]
    fn each_fragment_of_a_blob_reports_the_message_it_starts_in() {
        let body = blob();
        let plan = plan_fragments(&body);
        assert!(plan.ranges.len() >= 3, "{:?}", plan.ranges);
        assert_eq!(plan.heads.len(), plan.ranges.len());
        assert_eq!(plan.heads[0].map(|h| h.msg_id), Some(0x09));
        for (range, head) in plan.ranges.iter().zip(&plan.heads).skip(1) {
            let head = head.expect("every fragment starts inside a framed message");
            assert_eq!(head.msg_id, 0xBD, "fragment at {}", range.start);
            assert!(head.offset <= range.start && range.start < head.offset + 48);
            assert_eq!((head.entity_id, head.method_index), (Some(42), Some(68)));
        }
    }

    /// One packet: nothing is recorded at plan time; the head is read from
    /// the packet's own body when a stall is reported.
    #[test]
    fn a_one_packet_plan_records_nothing() {
        let body = blob();
        let plan = plan_fragments(&body[..40]);
        assert!(plan.heads.is_empty());
        assert_eq!(first_message_head(&body).map(|h| h.msg_id), Some(0x09));
    }

    #[test]
    fn an_unframeable_message_has_no_head() {
        assert_eq!(first_message_head(&[0x50, 0, 0]), None);
        assert_eq!(first_message_head(&[]), None);
        // A length that overruns the body does not frame either.
        assert_eq!(first_message_head(&[0x09, 9, 0, 1]), None);
    }
}
