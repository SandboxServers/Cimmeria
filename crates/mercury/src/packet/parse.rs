//! `parse_incoming` — pop the flag-driven footer stack off the raw UDP
//! datagram and surface the resulting body + parsed footer fields.

use bytes::Bytes;

use cimmeria_common::{CimmeriaError, Result};

use super::{ParsedPacket, FLAG_FRAGMENTED, FLAG_HAS_ACKS, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE};

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
