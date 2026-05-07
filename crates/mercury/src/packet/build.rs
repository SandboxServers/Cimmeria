//! Outgoing-packet builders — `build_outgoing` (single packet),
//! `build_outgoing_fragmented` (one fragment of a bundle), and
//! `build_fragmented_bundle` (the chunker that picks fragmented vs.
//! single-packet output based on body size).

use bytes::{BufMut, BytesMut};

use super::{FLAG_FRAGMENTED, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE};

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
        + if acks.is_empty() {
            0
        } else {
            acks.len() * 4 + 1
        };

    let mut buf = BytesMut::with_capacity(1 + body.len() + footer_size);

    buf.put_u8(flags);
    buf.put_slice(body);

    // Footers in innermost-first order (matches C++ finalize() write order):

    // first_req_offset (innermost: closest to body)
    if let Some(off) = first_req_offset {
        buf.put_u16_le(off);
    }

    // Fragment IDs (frag_begin, frag_end) written when FLAG_FRAGMENTED is set.
    // Caller must set FLAG_FRAGMENTED in flags and provide frag range via
    // build_outgoing_fragmented() or by manually constructing the packet.
    // For non-fragmented packets this block is a no-op.

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

/// Build a single fragmented Mercury packet with `frag_begin` / `frag_end` footers.
///
/// Same as [`build_outgoing`] but includes `FLAG_FRAGMENTED` and the two u32
/// fragment range footers. Used by [`build_fragmented_bundle`] to create each
/// fragment of a large message body.
pub fn build_outgoing_fragmented(
    flags: u8,
    body: &[u8],
    seq_id: u32,
    frag_begin: u32,
    frag_end: u32,
    acks: &[u32],
) -> BytesMut {
    let frag_flags = flags | FLAG_FRAGMENTED | FLAG_HAS_SEQUENCE;
    let footer_size = 4 + 4 + 4 // frag_begin + frag_end + seq_id
        + if acks.is_empty() { 0 } else { acks.len() * 4 + 1 };

    let mut buf = BytesMut::with_capacity(1 + body.len() + footer_size);

    buf.put_u8(frag_flags);
    buf.put_slice(body);

    // Footers in innermost-first order:
    // frag_begin, frag_end
    buf.put_u32_le(frag_begin);
    buf.put_u32_le(frag_end);

    // seq_id
    buf.put_u32_le(seq_id);

    // acks + ack_count (outermost)
    if !acks.is_empty() {
        for &ack in acks {
            buf.put_u32_le(ack);
        }
        buf.put_u8(acks.len() as u8);
    }

    buf
}

/// Maximum body bytes per Mercury UDP datagram (plaintext, before encryption).
///
/// Mercury `MAX_BODY_LENGTH` is 1411 bytes. With footer overhead (flags=1,
/// frag_begin=4, frag_end=4, seq_id=4, plus ack headroom) and AES-CBC
/// encryption overhead (PKCS7 padding up to 16 bytes + 16-byte HMAC), we use
/// 1300 bytes to stay safely under the UDP MTU.
pub const FRAGMENT_BODY_SIZE: usize = 1300;

/// Fragment a large message body into multiple encrypted Mercury packets.
///
/// Splits `body` into chunks of [`FRAGMENT_BODY_SIZE`] bytes, wraps each in a
/// Mercury packet with `FLAG_FRAGMENTED` + sequential sequence IDs, and encrypts
/// with the session key.
///
/// If the body fits in a single packet, returns one non-fragmented packet
/// (no `FLAG_FRAGMENTED` overhead).
///
/// Returns `(encrypted_packets, sequence_ids_consumed)`.
pub fn build_fragmented_bundle(
    base_flags: u8,
    body: &[u8],
    base_seq: u32,
    acks: &[u32],
    encrypt: impl Fn(&[u8]) -> Vec<u8>,
) -> (Vec<Vec<u8>>, u32) {
    if body.len() <= FRAGMENT_BODY_SIZE {
        // Fits in one packet — no fragmentation needed.
        let flags =
            base_flags | FLAG_HAS_SEQUENCE | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
        let plaintext = build_outgoing(flags, body, Some(base_seq), acks, None);
        return (vec![encrypt(&plaintext)], 1);
    }

    // Calculate fragment count and sequence range.
    let num_frags = body.len().div_ceil(FRAGMENT_BODY_SIZE);
    let frag_begin = base_seq;
    let frag_end = base_seq + num_frags as u32 - 1;

    let mut packets = Vec::with_capacity(num_frags);
    for (i, chunk) in body.chunks(FRAGMENT_BODY_SIZE).enumerate() {
        let seq = base_seq + i as u32;
        // Only first fragment carries acks.
        let pkt_acks: &[u32] = if i == 0 { acks } else { &[] };
        let flags = base_flags
            | if pkt_acks.is_empty() {
                0
            } else {
                FLAG_HAS_ACKS
            };
        let plaintext =
            build_outgoing_fragmented(flags, chunk, seq, frag_begin, frag_end, pkt_acks);
        packets.push(encrypt(&plaintext));
    }

    (packets, num_frags as u32)
}
