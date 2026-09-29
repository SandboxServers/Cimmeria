//! The `Bundle::iterator` as the message loop leaves it, decoded from one
//! read of its first `0x24` bytes, and the message's `InterfaceElement`.
//!
//! Layout from `Bundle::iterator::unpack` (`0x01579830`), `data()`
//! (`0x01579a50`) and `next()` (`0x01579cd0`); see the findings doc, "The
//! bundle message loop".

/// The iterator's position, read before `unpack`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IterState {
    /// Current packet.
    pub packet: u32,
    /// Its data length (u16 at iterator `+4`).
    pub packet_len: u16,
    /// Cursor within it (u16 at iterator `+6`; the first message is at 1).
    pub cursor: u16,
}

/// What `unpack` left in the iterator, read after the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unpacked {
    pub msg_id: u8,
    /// Iterator `+0x19`: `0x20` marks an unpack error.
    pub flag: u8,
    /// Body offset within the packet (u16 at `+8`).
    pub body_offset: u16,
    /// Body length (`+0xc`).
    pub len: u32,
    /// Decoded length (`+0x20`); `0xffffffff` when `expandLength` failed.
    pub decoded_len: u32,
}

/// The iterator's first `0x24` bytes, as one read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IterRaw {
    packet: u32,
    packet_len: u16,
    cursor: u16,
    body_offset: u16,
    len: u32,
    msg_id: u8,
    flag: u8,
    decoded_len: u32,
}

/// Bytes of the iterator read per message.
pub(crate) const ITER_BYTES: usize = 0x24;

impl IterRaw {
    /// Parse `bytes` (at least [`ITER_BYTES`] long) laid out as the
    /// `Bundle::iterator` (`+0` packet, `+4`/`+6` packet length and cursor,
    /// `+8` body offset, `+0xc` length, `+0x18` id, `+0x19` flag, `+0x20`
    /// decoded length).
    pub(crate) fn parse(b: &[u8]) -> Option<Self> {
        if b.len() < ITER_BYTES {
            return None;
        }
        let u16_at = |at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
        let u32_at = |at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
        Some(Self {
            packet: u32_at(0),
            packet_len: u16_at(4),
            cursor: u16_at(6),
            body_offset: u16_at(8),
            len: u32_at(0xc),
            msg_id: b[0x18],
            flag: b[0x19],
            decoded_len: u32_at(0x20),
        })
    }

    /// The position part, meaningful before `unpack`.
    pub(crate) fn state(&self) -> IterState {
        IterState {
            packet: self.packet,
            packet_len: self.packet_len,
            cursor: self.cursor,
        }
    }

    /// The result part, meaningful after `unpack`.
    pub(crate) fn unpacked(&self) -> Unpacked {
        Unpacked {
            msg_id: self.msg_id,
            flag: self.flag,
            body_offset: self.body_offset,
            len: self.len,
            decoded_len: self.decoded_len,
        }
    }
}

/// The message's `InterfaceElement`: length style and parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Element {
    /// `0` fixed length, `1` variable (width in `param`).
    pub style: u8,
    pub param: u32,
}

impl Element {
    /// Parse the first 8 bytes of an `InterfaceElement` (`+1` length style,
    /// `+4` length parameter).
    pub(crate) fn parse(b: &[u8]) -> Option<Self> {
        (b.len() >= 8).then(|| Self {
            style: b[1],
            param: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        })
    }

    /// Header bytes (`0x0158aa40`): message id plus the inline length.
    pub(crate) fn header_len(&self) -> Option<usize> {
        match self.style {
            0 => Some(1),
            1 => Some(1 + self.param as usize),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORD: Element = Element { style: 1, param: 2 };

    /// The raw iterator bytes decode at the offsets the RE found.
    #[test]
    fn the_iterator_is_decoded_from_its_raw_bytes() {
        let mut b = [0u8; ITER_BYTES];
        b[0..4].copy_from_slice(&0x1234u32.to_le_bytes());
        b[4..6].copy_from_slice(&1401u16.to_le_bytes());
        b[6..8].copy_from_slice(&1400u16.to_le_bytes());
        b[8..10].copy_from_slice(&1403u16.to_le_bytes());
        b[0xc..0x10].copy_from_slice(&96u32.to_le_bytes());
        b[0x18] = 0x80;
        b[0x19] = 0x20;
        b[0x20..0x24].copy_from_slice(&u32::MAX.to_le_bytes());
        let raw = IterRaw::parse(&b).unwrap();
        assert_eq!(
            raw.state(),
            IterState {
                packet: 0x1234,
                packet_len: 1401,
                cursor: 1400
            }
        );
        let u = raw.unpacked();
        assert_eq!(
            (u.msg_id, u.flag, u.body_offset, u.len),
            (0x80, 0x20, 1403, 96)
        );
        assert_eq!(u.decoded_len, u32::MAX);
        assert_eq!(IterRaw::parse(&b[..0x20]), None);
        let e = Element::parse(&[0, 1, 0, 0, 2, 0, 0, 0]).unwrap();
        assert_eq!(e.header_len(), Some(3));
    }

    #[test]
    fn header_length_follows_the_length_style() {
        assert_eq!(Element { style: 0, param: 8 }.header_len(), Some(1));
        assert_eq!(WORD.header_len(), Some(3));
        assert_eq!(Element { style: 9, param: 0 }.header_len(), None);
    }
}
