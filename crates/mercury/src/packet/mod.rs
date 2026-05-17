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
//!
//! The implementation is split across a few sibling modules:
//!
//! - [`parse`]  — `parse_incoming` (footer-strip path)
//! - [`build`]  — `build_outgoing`, `build_outgoing_fragmented`,
//!   `build_fragmented_bundle` (footer-write path)
//! - [`legacy`] — `Packet` and `PacketFlags` legacy shim used by the
//!   channel/nub stubs

mod build;
mod legacy;
mod parse;

#[cfg(test)]
mod parse_proptest;

#[cfg(test)]
mod proptest_round_trip;

#[cfg(test)]
mod replay_smoke;

#[cfg(test)]
mod tests;

pub use build::{
    build_fragmented_bundle, build_outgoing, build_outgoing_fragmented, FRAGMENT_BODY_SIZE,
};
pub use bytes::Bytes;
pub use legacy::{Packet, PacketFlags};
pub use parse::parse_incoming;

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

/// Valid Mercury sequence-number range is 28 bits — `0x00000000` through
/// `0x0FFFFFFF` inclusive. Mask any candidate sequence with this value
/// to keep it inside the spec'd range; counters that overflow into the
/// 29th bit (e.g. an unmasked `AtomicU32::fetch_add` past `0x0FFFFFFF`)
/// would collide with [`NULL_SEQUENCE`] on the wire.
///
/// Spec: `docs/drafts/spec/mercury-wire-format.md` §1.7 + §2.4 R4.
/// Issue #292 finding #7.
pub const SEQUENCE_MASK: u32 = 0x0FFF_FFFF;

/// Sequence ID value that signals "unset" — exactly one past the valid
/// 28-bit range. Any inbound packet whose `seq_id` field equals (or
/// exceeds) this value is dropped at parse time as an R4-class
/// violation; outbound sequence assignment must mask with
/// [`SEQUENCE_MASK`] to never produce this value on the wire.
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
