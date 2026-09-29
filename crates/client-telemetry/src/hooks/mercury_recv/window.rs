//! What the reliable window (`UnAckedHandler::queueAckForPacket`,
//! `0x0158cba0`) did with one sequence number.
//!
//! The detour reads three channel words before and after the call and
//! [`classify`] turns them into a disposition. The rules it inverts are in
//! the findings doc, "The reliable window": the ACK is queued first, then
//! `seq == inSeqAt` delivers (and drains any buffered followers), a `seq`
//! ahead within the window is buffered (or a duplicate of a buffered one),
//! and everything else is dropped as out of range.

use serde_json::json;

use super::{Fields, SEQ_MASK, SEQ_UNSET};

/// The channel's window words at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowState {
    /// `inSeqAt`: the next reliable sequence expected.
    pub in_seq_at: u32,
    /// Packets buffered ahead of it.
    pub buffered: u32,
}

/// What became of a reliable packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disposition {
    /// It was the expected one. `released` packets left the window in
    /// order: itself plus `released - 1` buffered followers.
    Delivered { released: u32 },
    /// Ahead of `inSeqAt` by `ahead`: held back for a gap.
    Buffered { ahead: u32 },
    /// Ahead within the window but the slot was already taken.
    DuplicateBuffered { ahead: u32 },
    /// Ahead of `inSeqAt` by more than the window allows.
    OutOfWindow { ahead: u32, window: u32 },
    /// Behind `inSeqAt`: already delivered, or lost to the "first packet
    /// defines the origin" rule.
    OldDuplicate { behind: u32 },
}

impl Disposition {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Disposition::Delivered { .. } => "delivered",
            Disposition::Buffered { .. } => "buffered",
            Disposition::DuplicateBuffered { .. } => "duplicate_buffered",
            Disposition::OutOfWindow { .. } => "out_of_window",
            Disposition::OldDuplicate { .. } => "old_duplicate",
        }
    }

    /// Delivered or buffered: the packet is not lost. Everything else is a
    /// drop (already ACKed, so the server will not resend it).
    pub(crate) fn is_happy(self) -> bool {
        matches!(
            self,
            Disposition::Delivered { .. } | Disposition::Buffered { .. }
        )
    }

    pub(crate) fn is_buffered(self) -> bool {
        matches!(self, Disposition::Buffered { .. })
    }
}

/// Classify what `queueAckForPacket` did with `seq`, from the window words
/// `before` and `after` the call and the configured `window` size.
pub(crate) fn classify(
    seq: u32,
    before: WindowState,
    after: WindowState,
    window: u32,
) -> Disposition {
    // The first reliable packet on a channel defines the origin.
    let origin = if before.in_seq_at == SEQ_UNSET {
        seq
    } else {
        before.in_seq_at
    };
    if after.in_seq_at != before.in_seq_at && after.in_seq_at != SEQ_UNSET {
        return Disposition::Delivered {
            released: after.in_seq_at.wrapping_sub(origin) & SEQ_MASK,
        };
    }
    let ahead = seq.wrapping_sub(origin) & SEQ_MASK;
    if ahead < 0x0800_0001 {
        if ahead > window {
            Disposition::OutOfWindow { ahead, window }
        } else if after.buffered > before.buffered {
            Disposition::Buffered { ahead }
        } else {
            Disposition::DuplicateBuffered { ahead }
        }
    } else {
        Disposition::OldDuplicate {
            behind: origin.wrapping_sub(seq) & SEQ_MASK,
        }
    }
}

/// The window fields of a `client.mercury.packet_in`.
pub(crate) fn fields(d: Disposition, before: WindowState, after: WindowState) -> Fields {
    let mut f: Fields = vec![
        ("disposition", json!(d.name())),
        ("in_seq_at_before", json!(before.in_seq_at)),
        ("in_seq_at_after", json!(after.in_seq_at)),
        ("buffered_after", json!(after.buffered)),
    ];
    match d {
        Disposition::Delivered { released } => f.push(("released", json!(released))),
        Disposition::Buffered { ahead } | Disposition::DuplicateBuffered { ahead } => {
            f.push(("ahead", json!(ahead)))
        }
        Disposition::OutOfWindow { ahead, window } => {
            f.push(("ahead", json!(ahead)));
            f.push(("window", json!(window)));
        }
        Disposition::OldDuplicate { behind } => f.push(("behind", json!(behind))),
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(in_seq_at: u32, buffered: u32) -> WindowState {
        WindowState {
            in_seq_at,
            buffered,
        }
    }

    #[test]
    fn the_expected_packet_is_delivered() {
        let d = classify(70, st(70, 0), st(71, 0), 512);
        assert_eq!(d, Disposition::Delivered { released: 1 });
        assert!(d.is_happy());
    }

    #[test]
    fn a_packet_that_fills_a_gap_releases_its_buffered_followers() {
        // 74 arrives after 75 and 76 were buffered: 74, 75, 76 leave.
        let d = classify(74, st(74, 2), st(77, 0), 512);
        assert_eq!(d, Disposition::Delivered { released: 3 });
    }

    #[test]
    fn a_packet_ahead_of_a_gap_is_buffered() {
        let d = classify(75, st(74, 0), st(74, 1), 512);
        assert_eq!(d, Disposition::Buffered { ahead: 1 });
        assert!(d.is_happy() && d.is_buffered());
    }

    /// The bug shape the window hides: a duplicate of a buffered packet and
    /// a stale packet are both ACKed and both dropped.
    #[test]
    fn duplicates_and_stale_packets_are_drops() {
        let dup = classify(75, st(74, 1), st(74, 1), 512);
        assert_eq!(dup, Disposition::DuplicateBuffered { ahead: 1 });
        assert!(!dup.is_happy());
        let old = classify(60, st(74, 0), st(74, 0), 512);
        assert_eq!(old, Disposition::OldDuplicate { behind: 14 });
        assert!(!old.is_happy());
    }

    #[test]
    fn a_packet_beyond_the_window_is_out_of_window() {
        let d = classify(700, st(74, 0), st(74, 0), 512);
        assert_eq!(
            d,
            Disposition::OutOfWindow {
                ahead: 626,
                window: 512
            }
        );
    }

    /// The first reliable packet on a channel becomes the origin, so it is
    /// delivered even though `inSeqAt` read as unset before the call.
    #[test]
    fn the_first_packet_on_a_channel_defines_the_origin() {
        let d = classify(9, st(SEQ_UNSET, 0), st(10, 0), 512);
        assert_eq!(d, Disposition::Delivered { released: 1 });
    }

    #[test]
    fn sequence_numbers_wrap_at_28_bits() {
        let d = classify(0, st(SEQ_MASK, 0), st(1, 0), 512);
        assert_eq!(d, Disposition::Delivered { released: 2 });
        let d = classify(1, st(SEQ_MASK, 0), st(SEQ_MASK, 1), 512);
        assert_eq!(d, Disposition::Buffered { ahead: 2 });
    }
}
