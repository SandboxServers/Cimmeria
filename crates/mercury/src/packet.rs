//! Mercury packet framing — Cimmeria wire format.
//!
//! Wire format (from C++ `packet.cpp`):
//! ```text
//! [flags: u8]   [body...]   [footer bytes appended at end]
//! ```
//!
//! Footer layout — stored in forward memory order (innermost = closest to body),
//! stripped backward on receive via `pop()`:
//! ```text
//!   u16  first_req_offset   (innermost: if FLAG_HAS_REQUESTS)
//!   u32  frag_begin         (if FLAG_FRAGMENTED)
//!   u32  frag_end           (if FLAG_FRAGMENTED)
//!   u32  seq_id             (if FLAG_HAS_SEQUENCE)
//!   u32  ack[ack_count]     (if FLAG_HAS_ACKS, ack_count times)
//!   u8   ack_count          (outermost: if FLAG_HAS_ACKS)
//! ```
//!
//! When encrypted, the ENTIRE wire payload (flags + body + footers + PKCS7 padding)
//! is AES-256-CBC encrypted, then a 16-byte HMAC-MD5 tag is appended.

use bytes::{BufMut, Bytes, BytesMut};

use cimmeria_common::{CimmeriaError, Result};

// ── Flag byte constants (C++ packet.hpp) ────────────────────────────────────

/// Packet contains messages with request IDs (footers include `first_req_offset`).
pub const FLAG_HAS_REQUESTS: u8 = 0x01;

/// Packet contains piggybacked sub-packets (not supported by Cimmeria).
pub const FLAG_PIGGYBACK: u8 = 0x02;

/// Packet footer contains cumulative ACKs.
pub const FLAG_HAS_ACKS: u8 = 0x04;

/// Packet was sent on a persistent channel.
pub const FLAG_ON_CHANNEL: u8 = 0x08;

/// Packet carries a reliable-delivery obligation and must be ACKed.
pub const FLAG_RELIABLE: u8 = 0x10;

/// Packet is a fragment of a larger message bundle.
pub const FLAG_FRAGMENTED: u8 = 0x20;

/// Packet carries a sequence number.
pub const FLAG_HAS_SEQUENCE: u8 = 0x40;

/// Packet addresses an indexed sub-channel (unused in SGW).
pub const FLAG_INDEXED: u8 = 0x80;

/// Sequence ID value that signals "unset" (> valid range 0..0x0FFFFFFF).
pub const NULL_SEQUENCE: u32 = 0x1000_0000;

// ── ParsedPacket ─────────────────────────────────────────────────────────────

/// A Mercury packet after footers have been stripped from the raw UDP datagram.
///
/// Produced by [`parse_incoming`].
#[derive(Debug, Clone)]
pub struct ParsedPacket {
    /// Raw flags byte (see `FLAG_*` constants).
    pub flags: u8,
    /// Message body — everything between the flags byte and the first footer.
    pub body: Bytes,
    /// Sequence number (present when `FLAG_HAS_SEQUENCE` is set).
    pub seq_id: Option<u32>,
    /// Byte offset within the packet buffer (1-based from start, after flags byte)
    /// where the first request message begins. Present when `FLAG_HAS_REQUESTS` is set.
    pub first_req_offset: Option<u16>,
    /// First fragment sequence ID (present when `FLAG_FRAGMENTED` is set).
    pub frag_begin: Option<u32>,
    /// Last fragment sequence ID (present when `FLAG_FRAGMENTED` is set).
    pub frag_end: Option<u32>,
    /// Acknowledged sequence IDs (present when `FLAG_HAS_ACKS` is set).
    pub acks: Vec<u32>,
}

impl ParsedPacket {
    #[inline]
    pub fn has_requests(&self) -> bool {
        self.flags & FLAG_HAS_REQUESTS != 0
    }
    #[inline]
    pub fn has_sequence(&self) -> bool {
        self.flags & FLAG_HAS_SEQUENCE != 0
    }
    #[inline]
    pub fn is_fragmented(&self) -> bool {
        self.flags & FLAG_FRAGMENTED != 0
    }
    #[inline]
    pub fn has_acks(&self) -> bool {
        self.flags & FLAG_HAS_ACKS != 0
    }
    #[inline]
    pub fn is_on_channel(&self) -> bool {
        self.flags & FLAG_ON_CHANNEL != 0
    }
    #[inline]
    pub fn is_reliable(&self) -> bool {
        self.flags & FLAG_RELIABLE != 0
    }
}

// ── parse_incoming ────────────────────────────────────────────────────────────

/// Parse a raw UDP datagram into a [`ParsedPacket`] by stripping footers.
///
/// Footers are stripped from the END of the buffer in reverse order
/// (outermost → innermost): acks → seq_id → frag ids → first_req_offset.
///
/// The flags byte at `raw[0]` is consumed. The body is `raw[1..body_end]`.
pub fn parse_incoming(raw: &[u8]) -> Result<ParsedPacket> {
    if raw.is_empty() {
        return Err(CimmeriaError::BufferUnderflow {
            needed: 1,
            available: 0,
        });
    }

    let flags = raw[0];
    let mut end = raw.len(); // exclusive upper bound; shrinks as footers are popped

    // ── Inner pop helpers ──────────────────────────────────────────────────
    macro_rules! pop_u8 {
        () => {{
            if end < 1 {
                return Err(CimmeriaError::BufferUnderflow {
                    needed: 1,
                    available: end,
                });
            }
            end -= 1;
            raw[end]
        }};
    }

    macro_rules! pop_u16_le {
        () => {{
            if end < 2 {
                return Err(CimmeriaError::BufferUnderflow {
                    needed: 2,
                    available: end,
                });
            }
            end -= 2;
            u16::from_le_bytes([raw[end], raw[end + 1]])
        }};
    }

    macro_rules! pop_u32_le {
        () => {{
            if end < 4 {
                return Err(CimmeriaError::BufferUnderflow {
                    needed: 4,
                    available: end,
                });
            }
            end -= 4;
            u32::from_le_bytes([raw[end], raw[end + 1], raw[end + 2], raw[end + 3]])
        }};
    }

    // ── Strip footers (outermost first) ────────────────────────────────────

    // 1. ack_count + acks (outermost)
    let mut acks = Vec::new();
    if flags & FLAG_HAS_ACKS != 0 {
        let ack_count = pop_u8!();
        if ack_count == 0 {
            return Err(CimmeriaError::Protocol(
                "FLAG_HAS_ACKS set but ack_count=0".into(),
            ));
        }
        acks.reserve(ack_count as usize);
        // acks are stored before ack_count; pop them in reverse to reconstruct order
        for _ in 0..ack_count {
            acks.push(pop_u32_le!());
        }
        acks.reverse();
    }

    // 2. seq_id
    let seq_id = if flags & FLAG_HAS_SEQUENCE != 0 {
        Some(pop_u32_le!())
    } else {
        None
    };

    // 3. frag_end then frag_begin (pop order from C++: lastFragId, then firstFragId)
    let (frag_begin, frag_end) = if flags & FLAG_FRAGMENTED != 0 {
        let fe = pop_u32_le!();
        let fb = pop_u32_le!();
        (Some(fb), Some(fe))
    } else {
        (None, None)
    };

    // 4. first_req_offset (innermost — closest to body)
    let first_req_offset = if flags & FLAG_HAS_REQUESTS != 0 {
        Some(pop_u16_le!())
    } else {
        None
    };

    // Body is everything between the flags byte (index 0) and the first footer.
    if end < 1 {
        return Err(CimmeriaError::BufferUnderflow {
            needed: 1,
            available: end,
        });
    }
    let body = Bytes::copy_from_slice(&raw[1..end]);

    Ok(ParsedPacket {
        flags,
        body,
        seq_id,
        first_req_offset,
        frag_begin,
        frag_end,
        acks,
    })
}

// ── build_outgoing ────────────────────────────────────────────────────────────

/// Build a Cimmeria Mercury packet for transmission.
///
/// Assembles `[flags:u8][body][footers]` into a `BytesMut` suitable for
/// AES-256-CBC encryption or direct UDP transmission.
///
/// Footer order in output (innermost → outermost, matching the C++ `finalize()`):
/// - `first_req_offset` (u16 LE) — only if `FLAG_HAS_REQUESTS` in `flags`
/// - `frag_begin` (u32 LE) — only if `FLAG_FRAGMENTED` in `flags`
/// - `frag_end` (u32 LE) — only if `FLAG_FRAGMENTED` in `flags`
/// - `seq_id` (u32 LE) — only if `FLAG_HAS_SEQUENCE` in `flags`
/// - `acks` (u32 LE each) — only if `!acks.is_empty()`
/// - `ack_count` (u8) — only if `!acks.is_empty()`
///
/// Caller is responsible for setting correct flag bits to match provided data.
pub fn build_outgoing(
    flags: u8,
    body: &[u8],
    seq_id: Option<u32>,
    acks: &[u32],
    first_req_offset: Option<u16>,
) -> BytesMut {
    let footer_size = first_req_offset.map_or(0usize, |_| 2)
        + seq_id.map_or(0usize, |_| 4)
        + if acks.is_empty() { 0 } else { acks.len() * 4 + 1 };

    let mut buf = BytesMut::with_capacity(1 + body.len() + footer_size);

    buf.put_u8(flags);
    buf.put_slice(body);

    // Footers in innermost-first order (matches C++ finalize() write order):

    // first_req_offset (innermost: closest to body)
    if let Some(off) = first_req_offset {
        buf.put_u16_le(off);
    }

    // Fragment IDs are written if FLAG_FRAGMENTED — not needed for Phase 3 server
    // replies which never fragment. (Extend here when fragmentation is required.)
    debug_assert!(
        flags & FLAG_FRAGMENTED == 0,
        "build_outgoing: fragment support not yet implemented"
    );

    // seq_id
    if let Some(seq) = seq_id {
        buf.put_u32_le(seq);
    }

    // acks + ack_count (outermost)
    if !acks.is_empty() {
        for &ack in acks {
            buf.put_u32_le(ack);
        }
        buf.put_u8(acks.len() as u8);
    }

    buf
}

// ── Legacy compatibility shim ─────────────────────────────────────────────────
//
// `channel.rs` and `nub.rs` are stubs that reference `Packet` and `PacketFlags`.
// Provide minimal types so those stubs continue to compile until they are rewritten.

/// Ergonomic wrapper around the single-byte Mercury flags field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacketFlags(pub u8);

impl PacketFlags {
    #[inline]
    pub fn from_byte(b: u8) -> Self {
        Self(b)
    }
    #[inline]
    pub fn bits(self) -> u8 {
        self.0
    }
    #[inline]
    pub fn is_reliable(self) -> bool {
        self.0 & FLAG_RELIABLE != 0
    }
    #[inline]
    pub fn is_fragmented(self) -> bool {
        self.0 & FLAG_FRAGMENTED != 0
    }
    #[inline]
    pub fn has_requests(self) -> bool {
        self.0 & FLAG_HAS_REQUESTS != 0
    }
    #[inline]
    pub fn has_acks(self) -> bool {
        self.0 & FLAG_HAS_ACKS != 0
    }
    #[inline]
    pub fn is_on_channel(self) -> bool {
        self.0 & FLAG_ON_CHANNEL != 0
    }
    #[inline]
    pub fn has_sequence(self) -> bool {
        self.0 & FLAG_HAS_SEQUENCE != 0
    }
    /// Return a new `PacketFlags` with the given flag bit set.
    #[inline]
    pub fn with(self, flag: u8) -> Self {
        Self(self.0 | flag)
    }
    /// Return a new `PacketFlags` with the given flag bit cleared.
    #[inline]
    pub fn without(self, flag: u8) -> Self {
        Self(self.0 & !flag)
    }
}

/// A simple packet handle used by the channel/nub stubs.
#[derive(Debug, Clone)]
pub struct Packet {
    /// Flags byte.
    pub flags: PacketFlags,
    /// Sequence number.
    pub sequence: u32,
    /// Payload body (between flags byte and footers).
    pub body: Bytes,
}

impl Packet {
    /// Create a new outgoing packet.
    pub fn new(flags: PacketFlags, sequence: u32, body: Bytes) -> Self {
        Self {
            flags,
            sequence,
            body,
        }
    }

    /// Encode this packet to wire bytes using [`build_outgoing`].
    ///
    /// Only writes the sequence-number footer when `FLAG_HAS_SEQUENCE` is set
    /// in `self.flags`, so that [`parse_incoming`] can round-trip correctly.
    /// Assumes no ACKs and no first_req_offset (server-side reply format).
    pub fn encode(&self) -> BytesMut {
        let seq = if self.flags.has_sequence() {
            Some(self.sequence)
        } else {
            None
        };
        build_outgoing(self.flags.bits(), &self.body, seq, &[], None)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build then parse a round-trip.
    fn round_trip(flags: u8, body: &[u8], seq_id: Option<u32>) -> ParsedPacket {
        let built = build_outgoing(flags, body, seq_id, &[], None);
        parse_incoming(&built).unwrap()
    }

    #[test]
    fn parse_baseapp_login_packet() {
        // Construct a synthetic baseAppLogin packet (flags=0x41, 41 bytes).
        // Packet layout: [0x41][msg_id=0x00][len:u16=25][requestId:u32][nextReqOff:u16=0]
        //                [accountId:u32][ticketLen=20][ticket:20][first_req_off:u16=1][seqId:u32]
        let mut raw = Vec::new();
        raw.push(0x41u8); // flags
        raw.push(0x00u8); // msg_id
        raw.extend_from_slice(&25u16.to_le_bytes()); // WORD_LENGTH
        raw.extend_from_slice(&0xDEADBEEFu32.to_le_bytes()); // requestId
        raw.extend_from_slice(&0u16.to_le_bytes()); // nextReqOffset=0
        raw.extend_from_slice(&42u32.to_le_bytes()); // accountId
        raw.push(20u8); // ticketLen
        raw.extend_from_slice(b"12345678901234567890"); // ticket (20 bytes)
        // footer:
        raw.extend_from_slice(&1u16.to_le_bytes()); // first_req_offset=1
        raw.extend_from_slice(&7u32.to_le_bytes()); // seq_id=7

        assert_eq!(raw.len(), 41);

        let pkt = parse_incoming(&raw).unwrap();
        assert_eq!(pkt.flags, 0x41);
        assert_eq!(pkt.seq_id, Some(7));
        assert_eq!(pkt.first_req_offset, Some(1));
        assert!(pkt.acks.is_empty());

        // Body should be everything between flags byte and footers.
        assert_eq!(pkt.body.len(), 34); // 1+2+4+2+4+1+20 = 34
        assert_eq!(pkt.body[0], 0x00); // msg_id
    }

    #[test]
    fn build_and_parse_reply_packet() {
        // Server reply: flags=0x58, no requests, just seq_id footer.
        let flags = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL | FLAG_RELIABLE;
        let body = b"\xFF\x19\x00\x00\x00\xDE\xAD\xBE\xEF\x14"; // truncated for test

        let pkt = round_trip(flags, body, Some(1));

        assert_eq!(pkt.flags, flags);
        assert_eq!(pkt.seq_id, Some(1));
        assert_eq!(pkt.first_req_offset, None);
        assert_eq!(pkt.body.as_ref(), body);
    }

    #[test]
    fn parse_empty_body_with_seq() {
        let flags = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL;
        let pkt = round_trip(flags, b"", Some(42));
        assert_eq!(pkt.seq_id, Some(42));
        assert!(pkt.body.is_empty());
    }

    #[test]
    fn parse_flags_only_no_footers() {
        // A packet with no flags that add footers.
        let raw = [FLAG_PIGGYBACK]; // no seq, no acks, no requests
        let pkt = parse_incoming(&raw).unwrap();
        assert_eq!(pkt.flags, FLAG_PIGGYBACK);
        assert!(pkt.seq_id.is_none());
        assert!(pkt.body.is_empty());
    }

    #[test]
    fn parse_too_short_fails() {
        let err = parse_incoming(&[]).unwrap_err();
        assert!(matches!(err, CimmeriaError::BufferUnderflow { .. }));
    }

    #[test]
    fn parse_seq_truncated_fails() {
        // flags says HAS_SEQUENCE but only 3 footer bytes available
        let raw = [FLAG_HAS_SEQUENCE, 0x01, 0x02, 0x03]; // only 3 bytes after flags
        // seq_id needs 4 bytes, we only have 3
        assert!(parse_incoming(&raw).is_err());
    }

    #[test]
    fn packet_flags_operations() {
        let f = PacketFlags::default()
            .with(FLAG_RELIABLE)
            .with(FLAG_HAS_SEQUENCE);
        assert!(f.is_reliable());
        assert!(f.has_sequence());
        assert!(!f.has_acks());

        let f2 = f.without(FLAG_RELIABLE);
        assert!(!f2.is_reliable());
        assert!(f2.has_sequence());
    }
}
