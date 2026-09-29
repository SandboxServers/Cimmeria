//! The footers of one datagram, and whether the packet earns an event.
//!
//! `parse` mirrors, on a copy of the datagram, how `processFilteredPacket`
//! (`0x01580840`) and `processPacket` (`0x0157fd20`) strip footers from the
//! tail. The wire order, front to back, is
//!
//! ```text
//! payload | firstFrag u32 | lastFrag u32 | firstRequestOffset u16 | seq u32 | ack u32 x n | n u8
//!           [0x20]           [0x20]         [0x01]                    [0x40]    [0x04]        [0x04]
//! ```
//!
//! and every word is a raw little-endian read (findings doc, "Footer layout
//! and strip order").

use serde_json::json;

use super::{Fields, SEQ_MASK, SEQ_UNSET};

/// Flag bits of `data[0]`.
pub(crate) mod flag {
    pub const REQUEST_OFFSET: u8 = 0x01;
    pub const PIGGYBACK: u8 = 0x02;
    pub const ACKS: u8 = 0x04;
    pub const ON_CHANNEL: u8 = 0x08;
    pub const RELIABLE: u8 = 0x10;
    pub const FRAGMENT: u8 = 0x20;
    pub const SEQ: u8 = 0x40;
    pub const REJECTED: u8 = 0x80;
}

/// Why the client refuses a datagram before the reliable window sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fault {
    /// Fewer than two bytes (`processFilteredPacket` top test).
    TooShort,
    /// Flag `0x80`: `"received packet with bad flags"`.
    BadFlags,
    /// Too few bytes for the ACK footer.
    ShortAckFooter,
    /// No sequence footer and no ACKs either.
    NoSeq,
    /// Too few bytes for the sequence footer.
    ShortSeqFooter,
    /// `seq == 0x10000000` or has a bit above 28 set.
    SeqOutOfRange,
    /// Too few bytes for the first-request-offset footer.
    ShortRequestFooter,
    /// Fragment flag with fewer than 8 footer bytes left.
    ShortFragmentFooter,
    /// Fragment with `lastFrag - firstFrag + 1 < 2`.
    IllegalBundleSize,
}

impl Fault {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Fault::TooShort => "too_short",
            Fault::BadFlags => "bad_flags",
            Fault::ShortAckFooter => "short_ack_footer",
            Fault::NoSeq => "no_sequence_footer",
            Fault::ShortSeqFooter => "short_sequence_footer",
            Fault::SeqOutOfRange => "sequence_out_of_range",
            Fault::ShortRequestFooter => "short_request_footer",
            Fault::ShortFragmentFooter => "short_fragment_footer",
            Fault::IllegalBundleSize => "illegal_bundle_size",
        }
    }
}

/// One datagram's footers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Tail {
    pub flags: u8,
    /// Datagram length as the client sees it at `processFilteredPacket`
    /// entry (flags byte included).
    pub len: usize,
    /// Number of ACKs the footer carries.
    pub ack_count: u8,
    pub seq: Option<u32>,
    pub request_offset: Option<u16>,
    /// `(firstFrag, lastFrag)`.
    pub fragment: Option<(u32, u32)>,
    /// Payload bytes left once every footer is stripped (`len - 1 - footers`).
    pub payload_len: usize,
    /// The packet carries a nested packet; the rest is not parsed.
    pub piggyback: bool,
    /// An ACK-only packet: ACKs, no sequence footer. The client logs and
    /// drops it after handling the ACKs, which is normal traffic.
    pub ack_only: bool,
    pub fault: Option<Fault>,
}

impl Tail {
    pub(crate) fn reliable(&self) -> bool {
        self.flags & flag::RELIABLE != 0
    }
    pub(crate) fn is_fragment(&self) -> bool {
        self.flags & flag::FRAGMENT != 0
    }
    pub(crate) fn on_channel(&self) -> bool {
        self.flags & flag::ON_CHANNEL != 0
    }
    /// Fragments the bundle is made of, `lastFrag - firstFrag + 1` as the
    /// client computes it (signed, no wrap handling).
    pub(crate) fn fragment_count(&self) -> Option<i32> {
        self.fragment
            .map(|(first, last)| (last as i32).wrapping_sub(first as i32).wrapping_add(1))
    }
}

fn u32_at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

/// Parse the footers of `data` (`data[0]` is the flags byte).
pub(crate) fn parse(data: &[u8]) -> Tail {
    let mut t = Tail {
        len: data.len(),
        ..Tail::default()
    };
    if data.len() < 2 {
        t.fault = Some(Fault::TooShort);
        return t;
    }
    t.flags = data[0];
    // The client tests the piggyback bit first; `0x80` is only rejected on
    // the non-piggyback branch.
    if t.flags & flag::PIGGYBACK != 0 {
        t.piggyback = true;
        return t;
    }
    if t.flags & flag::REJECTED != 0 {
        t.fault = Some(Fault::BadFlags);
        return t;
    }
    // `pos` is the client's `len`: bytes left, flags byte included.
    let mut pos = data.len();
    if t.flags & flag::ACKS != 0 {
        if pos < 2 {
            t.fault = Some(Fault::ShortAckFooter);
            return t;
        }
        let n = data[pos - 1] as usize;
        pos -= 1;
        if pos - 1 < n * 4 {
            t.fault = Some(Fault::ShortAckFooter);
            return t;
        }
        pos -= n * 4;
        t.ack_count = n as u8;
    }
    if t.flags & flag::SEQ == 0 {
        if t.flags & flag::ACKS != 0 {
            t.ack_only = true;
        } else {
            t.fault = Some(Fault::NoSeq);
        }
        t.payload_len = pos.saturating_sub(1);
        return t;
    }
    if pos < 5 {
        t.fault = Some(Fault::ShortSeqFooter);
        return t;
    }
    let seq = u32_at(data, pos - 4);
    pos -= 4;
    t.seq = Some(seq);
    if seq == SEQ_UNSET || seq & SEQ_MASK != seq {
        t.fault = Some(Fault::SeqOutOfRange);
        return t;
    }
    if t.flags & flag::REQUEST_OFFSET != 0 {
        if pos < 3 {
            t.fault = Some(Fault::ShortRequestFooter);
            return t;
        }
        t.request_offset = Some(u16::from_le_bytes([data[pos - 2], data[pos - 1]]));
        pos -= 2;
    }
    if t.flags & flag::FRAGMENT != 0 {
        if pos < 9 {
            t.fault = Some(Fault::ShortFragmentFooter);
            return t;
        }
        let last = u32_at(data, pos - 4);
        let first = u32_at(data, pos - 8);
        pos -= 8;
        t.fragment = Some((first, last));
        if t.fragment_count().is_some_and(|c| c < 2) {
            t.fault = Some(Fault::IllegalBundleSize);
        }
    }
    t.payload_len = pos - 1;
    t
}

/// Parse a packet as `processPacket` sees it: the ACK and sequence footers
/// are already stripped (`data` is shorter by them, the sequence number
/// lives in the packet object). A reliable packet released from the reorder
/// buffer reaches `processPacket` this way, long after its datagram was
/// filtered, so the datagram's own tail is gone.
pub(crate) fn at_process_packet(data: &[u8], seq: u32) -> Tail {
    if data.is_empty() {
        return parse(data);
    }
    let mut d = data.to_vec();
    // The ACK bit describes a footer that is no longer there.
    d[0] &= !flag::ACKS;
    if d[0] & flag::SEQ != 0 {
        d.extend(seq.to_le_bytes());
    }
    parse(&d)
}

/// Names of the flag bits set, for a readable event.
pub(crate) fn flag_names(flags: u8) -> Vec<&'static str> {
    const NAMES: [(u8, &str); 8] = [
        (flag::REQUEST_OFFSET, "request_offset"),
        (flag::PIGGYBACK, "piggyback"),
        (flag::ACKS, "acks"),
        (flag::ON_CHANNEL, "on_channel"),
        (flag::RELIABLE, "reliable"),
        (flag::FRAGMENT, "fragment"),
        (flag::SEQ, "seq"),
        (flag::REJECTED, "rejected"),
    ];
    NAMES
        .iter()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|(_, n)| *n)
        .collect()
}

/// The footer fields common to every `client.mercury.packet_in`.
pub(crate) fn fields(t: &Tail) -> Fields {
    let mut f: Fields = vec![
        ("flags", json!(t.flags)),
        ("flag_names", json!(flag_names(t.flags))),
        ("len", json!(t.len)),
        ("payload_len", json!(t.payload_len)),
        ("reliable", json!(t.reliable())),
        ("on_channel", json!(t.on_channel())),
        ("fragmented", json!(t.is_fragment())),
        ("has_acks", json!(t.flags & flag::ACKS != 0)),
    ];
    if t.ack_count > 0 {
        f.push(("ack_count", json!(t.ack_count)));
    }
    if let Some(s) = t.seq {
        f.push(("seq", json!(s)));
    }
    if let Some((first, last)) = t.fragment {
        f.push(("frag_first", json!(first)));
        f.push(("frag_last", json!(last)));
    }
    if let Some(o) = t.request_offset {
        f.push(("request_offset", json!(o)));
    }
    f
}

/// How an event about one packet is gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gate {
    /// Emit, bypassing every throttle: a fragment, anything while a
    /// fragment group is in flight, a packet buffered out of order, or any
    /// non-happy outcome.
    Always,
    /// Ordinary traffic: emit through the per-name token bucket.
    Throttled,
}

/// Decide the gate for a packet. `happy` is whether the client accepted it
/// without a fault, drop or duplicate; `buffered` whether the reliable
/// window held it back for a gap; `group_in_flight` whether a fragment
/// group is open on the channel.
pub(crate) fn gate(t: &Tail, happy: bool, buffered: bool, group_in_flight: bool) -> Gate {
    if !happy || t.is_fragment() || buffered || group_in_flight {
        Gate::Always
    } else {
        Gate::Throttled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A datagram: flags, `payload` bytes, then footers in wire order.
    fn datagram(
        flags: u8,
        payload: usize,
        frag: Option<(u32, u32)>,
        req: Option<u16>,
        seq: Option<u32>,
        acks: &[u32],
    ) -> Vec<u8> {
        let mut d = vec![flags];
        d.extend(std::iter::repeat_n(0xAA, payload));
        if let Some((first, last)) = frag {
            d.extend(first.to_le_bytes());
            d.extend(last.to_le_bytes());
        }
        if let Some(r) = req {
            d.extend(r.to_le_bytes());
        }
        if let Some(s) = seq {
            d.extend(s.to_le_bytes());
        }
        if !acks.is_empty() {
            for a in acks {
                d.extend(a.to_le_bytes());
            }
            d.push(acks.len() as u8);
        }
        d
    }

    #[test]
    fn a_reliable_fragment_yields_seq_and_both_fragment_ids() {
        let d = datagram(0x78, 1200, Some((70, 84)), None, Some(72), &[]);
        let t = parse(&d);
        assert_eq!(t.fault, None);
        assert_eq!(t.seq, Some(72));
        assert_eq!(t.fragment, Some((70, 84)));
        assert_eq!(t.fragment_count(), Some(15));
        assert_eq!(t.payload_len, 1200);
        assert!(t.reliable() && t.is_fragment() && t.on_channel());
    }

    #[test]
    fn footer_order_is_frag_then_request_then_seq_then_acks() {
        let d = datagram(0x7d, 10, Some((5, 9)), Some(0x0123), Some(6), &[1, 2]);
        let t = parse(&d);
        assert_eq!(t.fault, None);
        assert_eq!(t.ack_count, 2);
        assert_eq!(t.seq, Some(6));
        assert_eq!(t.request_offset, Some(0x0123));
        assert_eq!(t.fragment, Some((5, 9)));
        assert_eq!(t.payload_len, 10);
    }

    /// The client reads the words raw, little-endian; a big-endian encoder
    /// would show up as an out-of-range sequence number.
    #[test]
    fn a_big_endian_seq_is_reported_out_of_range() {
        let mut d = vec![0x58u8, 0xAA];
        d.extend(72u32.to_be_bytes());
        let t = parse(&d);
        assert_eq!(t.fault, Some(Fault::SeqOutOfRange));
    }

    #[test]
    fn an_ack_only_packet_is_not_a_fault() {
        let d = datagram(0x04, 0, None, None, None, &[7, 8, 9]);
        let t = parse(&d);
        assert!(t.ack_only);
        assert_eq!(t.fault, None);
        assert_eq!(t.ack_count, 3);
    }

    #[test]
    fn faults_follow_the_clients_checks() {
        assert_eq!(parse(&[0x40]).fault, Some(Fault::TooShort));
        assert_eq!(parse(&[0x80, 0]).fault, Some(Fault::BadFlags));
        assert_eq!(parse(&[0x10, 1, 2, 3]).fault, Some(Fault::NoSeq));
        // Fragment with the first == last footer: count 1.
        let d = datagram(0x78, 4, Some((9, 9)), None, Some(9), &[]);
        assert_eq!(parse(&d).fault, Some(Fault::IllegalBundleSize));
        // Fragment flag but only 4 footer bytes after the seq.
        let mut d = vec![0x68u8, 1, 2, 3];
        d.extend(5u32.to_le_bytes());
        assert_eq!(parse(&d).fault, Some(Fault::ShortFragmentFooter));
        // The null sequence number.
        let d = datagram(0x58, 2, None, None, Some(SEQ_UNSET), &[]);
        assert_eq!(parse(&d).fault, Some(Fault::SeqOutOfRange));
    }

    /// A buffered packet released later reaches `processPacket` with its ACK
    /// and sequence footers already stripped; the reconstruction must give
    /// the same view as parsing the whole datagram.
    #[test]
    fn a_stripped_packet_parses_like_the_whole_datagram() {
        let whole = datagram(0x7c, 300, Some((70, 84)), None, Some(75), &[5, 6]);
        let full = parse(&whole);
        // Strip the ACK footer (2 x u32 + count) and the sequence footer.
        let stripped = &whole[..whole.len() - (2 * 4 + 1) - 4];
        let t = at_process_packet(stripped, 75);
        assert_eq!(t.fragment, full.fragment);
        assert_eq!(t.seq, full.seq);
        assert_eq!(t.payload_len, full.payload_len);
        assert_eq!(t.fault, None);
    }

    #[test]
    fn a_piggyback_is_flagged_and_not_parsed_further() {
        let t = parse(&[0x02, 1, 2, 3]);
        assert!(t.piggyback);
        assert_eq!(t.fault, None);
    }

    #[test]
    fn the_gate_never_throttles_a_fragment_or_a_non_happy_packet() {
        let frag = parse(&datagram(0x78, 8, Some((1, 3)), None, Some(1), &[]));
        let plain = parse(&datagram(0x58, 8, None, None, Some(1), &[]));
        assert_eq!(gate(&frag, true, false, false), Gate::Always);
        assert_eq!(gate(&plain, false, false, false), Gate::Always);
        assert_eq!(gate(&plain, true, true, false), Gate::Always);
        assert_eq!(gate(&plain, true, false, true), Gate::Always);
        assert_eq!(gate(&plain, true, false, false), Gate::Throttled);
    }
}
