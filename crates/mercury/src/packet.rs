//! Mercury packet framing.
//!
//! Each Mercury UDP packet starts with a 4-byte header that packs both the
//! sequence number (bits 0-25) and flag bits (bits 26-31) into a single u32
//! transmitted in little-endian byte order.
//!
//! Wire format:
//! ```text
//! [u32 LE header] [body ...]
//!   bits  0-25 : sequence number (0 .. 67_108_863)
//!   bit  26    : RELIABLE
//!   bit  27    : FRAGMENTED
//!   bit  28    : HAS_REQUESTS
//!   bit  29    : HAS_PIGGYBACKS
//!   bit  30    : HAS_ACKS
//!   bit  31    : CREATE_CHANNEL
//! ```

use bytes::{Buf, BufMut, Bytes, BytesMut};
use cimmeria_common::{CimmeriaError, Result};

use crate::consts::HEADER_SIZE;

// ── Flag bit positions ──────────────────────────────────────────────────────

/// Mask for the 26-bit sequence number field (bits 0-25).
const SEQ_MASK: u32 = 0x03FF_FFFF;

/// Mask for the 6-bit flags field (bits 26-31).
const FLAGS_MASK: u32 = 0xFC00_0000;

/// Packet carries a reliable-delivery obligation.
pub const FLAG_RELIABLE: u32 = 1 << 26;

/// Packet is one fragment of a larger message.
pub const FLAG_FRAGMENTED: u32 = 1 << 27;

/// Packet footer contains piggybacked request data.
pub const FLAG_HAS_REQUESTS: u32 = 1 << 28;

/// Packet footer contains piggybacked reply data.
pub const FLAG_HAS_PIGGYBACKS: u32 = 1 << 29;

/// Packet footer contains cumulative ACK information.
pub const FLAG_HAS_ACKS: u32 = 1 << 30;

/// Packet requests creation of a new channel on the receiver.
pub const FLAG_CREATE_CHANNEL: u32 = 1 << 31;

// ── Helper functions ────────────────────────────────────────────────────────

/// Extract the 26-bit sequence number from a packed header word.
#[inline]
pub fn extract_sequence(header: u32) -> u32 {
    header & SEQ_MASK
}

/// Extract the 6-bit flags field from a packed header word.
#[inline]
pub fn extract_flags(header: u32) -> u32 {
    header & FLAGS_MASK
}

// ── PacketFlags ─────────────────────────────────────────────────────────────

/// Ergonomic wrapper around the Mercury flag bits.
///
/// All methods are `const` so flags can be inspected in compile-time contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketFlags(u32);

impl PacketFlags {
    /// Build from a raw header word (extracts only bits 26-31).
    #[inline]
    pub const fn from_header(header: u32) -> Self {
        Self(header & FLAGS_MASK)
    }

    /// Build from raw flag bits (bits 26-31 already positioned).
    #[inline]
    pub const fn from_raw(flags: u32) -> Self {
        Self(flags & FLAGS_MASK)
    }

    /// Return the raw u32 with only flag bits set.
    #[inline]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[inline]
    pub const fn is_reliable(self) -> bool {
        self.0 & FLAG_RELIABLE != 0
    }

    #[inline]
    pub const fn is_fragmented(self) -> bool {
        self.0 & FLAG_FRAGMENTED != 0
    }

    #[inline]
    pub const fn has_requests(self) -> bool {
        self.0 & FLAG_HAS_REQUESTS != 0
    }

    #[inline]
    pub const fn has_piggybacks(self) -> bool {
        self.0 & FLAG_HAS_PIGGYBACKS != 0
    }

    #[inline]
    pub const fn has_acks(self) -> bool {
        self.0 & FLAG_HAS_ACKS != 0
    }

    #[inline]
    pub const fn is_create_channel(self) -> bool {
        self.0 & FLAG_CREATE_CHANNEL != 0
    }

    /// Return a new `PacketFlags` with the given flag set.
    #[inline]
    pub const fn with(self, flag: u32) -> Self {
        Self(self.0 | (flag & FLAGS_MASK))
    }

    /// Return a new `PacketFlags` with the given flag cleared.
    #[inline]
    pub const fn without(self, flag: u32) -> Self {
        Self(self.0 & !(flag & FLAGS_MASK))
    }
}

impl Default for PacketFlags {
    fn default() -> Self {
        Self(0)
    }
}

// ── Packet ──────────────────────────────────────────────────────────────────

/// A single Mercury UDP packet (header + body).
#[derive(Debug, Clone)]
pub struct Packet {
    /// 26-bit sequence number.
    pub sequence: u32,
    /// Packed flag bits (bits 26-31 of the header word).
    pub flags: PacketFlags,
    /// Payload body (everything after the 4-byte header).
    pub body: Bytes,
}

impl Packet {
    /// Create a new packet with the given sequence, flags, and body.
    pub fn new(sequence: u32, flags: PacketFlags, body: Bytes) -> Self {
        Self {
            sequence: sequence & SEQ_MASK,
            flags,
            body,
        }
    }

    /// Pack the sequence and flags into the 4-byte header word.
    #[inline]
    fn header_word(&self) -> u32 {
        (self.sequence & SEQ_MASK) | self.flags.bits()
    }

    /// Encode this packet into `buf` in Mercury wire format (LE header + body).
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.reserve(HEADER_SIZE + self.body.len());
        buf.put_u32_le(self.header_word());
        buf.put_slice(&self.body);
    }

    /// Decode a packet from `buf`.
    ///
    /// Consumes exactly `HEADER_SIZE` bytes for the header; the remainder of
    /// `buf` becomes the body.
    pub fn decode(buf: &mut Bytes) -> Result<Packet> {
        if buf.len() < HEADER_SIZE {
            return Err(CimmeriaError::BufferUnderflow {
                needed: HEADER_SIZE,
                available: buf.len(),
            });
        }

        // Read the 4-byte LE header.
        let header = {
            let mut hdr_bytes = [0u8; 4];
            hdr_bytes.copy_from_slice(&buf[..HEADER_SIZE]);
            buf.advance(HEADER_SIZE);
            u32::from_le_bytes(hdr_bytes)
        };

        let sequence = extract_sequence(header);
        let flags = PacketFlags::from_header(header);
        let body = buf.split_to(buf.len());

        Ok(Packet {
            sequence,
            flags,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encode_decode() {
        let original = Packet::new(
            42,
            PacketFlags::default().with(FLAG_RELIABLE).with(FLAG_HAS_ACKS),
            Bytes::from_static(b"hello mercury"),
        );

        let mut buf = BytesMut::new();
        original.encode(&mut buf);

        let mut read_buf = buf.freeze();
        let decoded = Packet::decode(&mut read_buf).unwrap();

        assert_eq!(decoded.sequence, 42);
        assert!(decoded.flags.is_reliable());
        assert!(decoded.flags.has_acks());
        assert!(!decoded.flags.is_fragmented());
        assert_eq!(decoded.body.as_ref(), b"hello mercury");
    }

    #[test]
    fn extract_sequence_and_flags() {
        let header: u32 = 1000 | FLAG_FRAGMENTED | FLAG_RELIABLE;
        assert_eq!(extract_sequence(header), 1000);
        assert_eq!(extract_flags(header), FLAG_FRAGMENTED | FLAG_RELIABLE);
    }

    #[test]
    fn sequence_wraps_at_26_bits() {
        // Maximum 26-bit value.
        let pkt = Packet::new(0x03FF_FFFF, PacketFlags::default(), Bytes::new());
        assert_eq!(pkt.sequence, 0x03FF_FFFF);

        // Overflow gets masked.
        let pkt = Packet::new(0x0400_0000, PacketFlags::default(), Bytes::new());
        assert_eq!(pkt.sequence, 0);
    }

    #[test]
    fn decode_too_short() {
        let mut buf = Bytes::from_static(&[0u8, 1]);
        let err = Packet::decode(&mut buf).unwrap_err();
        assert!(matches!(err, CimmeriaError::BufferUnderflow { .. }));
    }
}
