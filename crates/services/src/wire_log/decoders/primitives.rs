//! Wire-format primitive readers.
//!
//! Cursor-style readers over `&[u8]` for the BigWorld wire types used
//! across SGW methods. Each returns `Option` — bounds failures yield
//! `None` so decoders compose with `?` and fall back to "no decoded
//! field" rather than panicking on malformed payloads.

/// Cursor for in-order primitive reads. Tracks position; advances on
/// successful reads. Failures leave the cursor untouched (caller
/// will typically abort the decode and return `None`).
pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Bytes left in the buffer. Used by callers that need to decide
    /// whether to keep reading variable-length tails (NVP arrays,
    /// component lists with truncated decoders). Allow unused — most
    /// current decoders read fixed schemas and don't need it; Phase 2
    /// decoders for ARRAY-tailed methods will use it heavily.
    #[allow(dead_code)]
    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub fn u8(&mut self) -> Option<u8> {
        let b = *self.buf.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }

    pub fn i8(&mut self) -> Option<i8> {
        self.u8().map(|b| b as i8)
    }

    #[allow(dead_code)]
    pub fn u16_le(&mut self) -> Option<u16> {
        let end = self.pos.checked_add(2)?;
        let slice = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(u16::from_le_bytes([slice[0], slice[1]]))
    }

    #[allow(dead_code)]
    pub fn i16_le(&mut self) -> Option<i16> {
        self.u16_le().map(|v| v as i16)
    }

    pub fn u32_le(&mut self) -> Option<u32> {
        let end = self.pos.checked_add(4)?;
        let slice = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    pub fn i32_le(&mut self) -> Option<i32> {
        self.u32_le().map(|v| v as i32)
    }

    // u64/i64 readers — kept for Phase 2 decoders (UINT64 TypeId on
    // InteractionType, UINT64 aFlags on onEntityFlags, etc.). Allow
    // unused at the function level rather than the module level so a
    // genuine unused primitive elsewhere still warns.
    #[allow(dead_code)]
    pub fn u64_le(&mut self) -> Option<u64> {
        let end = self.pos.checked_add(8)?;
        let slice = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(u64::from_le_bytes([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ]))
    }

    #[allow(dead_code)]
    pub fn i64_le(&mut self) -> Option<i64> {
        self.u64_le().map(|v| v as i64)
    }

    pub fn f32_le(&mut self) -> Option<f32> {
        self.u32_le().map(f32::from_bits)
    }

    /// Read a BigWorld `WSTRING`: `u32` UTF-16 char count, then that
    /// many `u16` LE code units. Returns the decoded UTF-8 string
    /// (lossy on invalid surrogates — observability shouldn't fail
    /// the parse for cosmetic encoding issues).
    pub fn wstring(&mut self) -> Option<String> {
        let char_count = self.u32_le()? as usize;
        let bytes_needed = char_count.checked_mul(2)?;
        let end = self.pos.checked_add(bytes_needed)?;
        let slice = self.buf.get(self.pos..end)?;
        self.pos = end;
        let utf16: Vec<u16> = slice
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        Some(String::from_utf16_lossy(&utf16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_reads_primitives_in_order() {
        let buf = [
            0x42, // u8 = 0x42
            0xFF, 0x00, // u16_le = 0x00FF
            0x04, 0x03, 0x02, 0x01, // u32_le = 0x01020304
        ];
        let mut c = Cursor::new(&buf);
        assert_eq!(c.u8(), Some(0x42));
        assert_eq!(c.u16_le(), Some(0x00FF));
        assert_eq!(c.u32_le(), Some(0x01020304));
        assert_eq!(c.remaining(), 0);
        assert_eq!(c.u8(), None, "past end returns None");
    }

    #[test]
    fn cursor_truncated_payload_returns_none() {
        let buf = [0x01, 0x02]; // only 2 bytes
        let mut c = Cursor::new(&buf);
        assert_eq!(c.u32_le(), None, "u32_le needs 4 bytes");
        assert_eq!(c.pos, 0, "failed read does not advance");
    }

    #[test]
    fn wstring_round_trips_ascii() {
        // char_count = 5, "Hello"
        let mut buf = vec![0x05, 0x00, 0x00, 0x00];
        for ch in "Hello".encode_utf16() {
            buf.extend_from_slice(&ch.to_le_bytes());
        }
        let mut c = Cursor::new(&buf);
        assert_eq!(c.wstring(), Some("Hello".to_string()));
    }

    #[test]
    fn wstring_truncated_length_returns_none() {
        // Claims 100 chars but only 4 bytes follow.
        let buf = [0x64, 0x00, 0x00, 0x00, 0x41, 0x00, 0x42, 0x00];
        let mut c = Cursor::new(&buf);
        assert_eq!(c.wstring(), None);
    }
}
