//! Legacy compatibility shim used by the `channel.rs` and `nub.rs` stubs.
//!
//! Provides minimal `Packet` / `PacketFlags` types so those stubs continue
//! to compile until they are rewritten against the modern
//! `build_outgoing` / `parse_incoming` path.

use bytes::{Bytes, BytesMut};

use super::{
    build_outgoing, FLAG_FRAGMENTED, FLAG_HAS_ACKS, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE,
    FLAG_ON_CHANNEL, FLAG_RELIABLE,
};

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
    /// in `self.flags`, so that [`super::parse_incoming`] can round-trip
    /// correctly. Assumes no ACKs and no first_req_offset (server-side reply
    /// format).
    pub fn encode(&self) -> BytesMut {
        let seq = if self.flags.has_sequence() {
            Some(self.sequence)
        } else {
            None
        };
        build_outgoing(self.flags.bits(), &self.body, seq, &[], None)
    }
}
