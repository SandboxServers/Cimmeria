//! Little-endian readers for the semantic decoders. Every read is checked
//! against the argument bytes first, so a short or forged body becomes a
//! [`SemanticError`] instead of a panic or a large allocation.

use super::SemanticError;

/// A cursor over one method's argument bytes. The method name travels with
/// the cursor so every error says which decoder rejected the body.
#[derive(Debug)]
pub struct Reader<'a> {
    method: &'static str,
    buf: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// Starts reading `buf` from offset 0 for the wire method `method`.
    pub fn new(method: &'static str, buf: &'a [u8]) -> Self {
        Reader { method, buf, at: 0 }
    }

    /// Takes the next `n` bytes. The bound is checked before the cursor moves,
    /// so a failed read leaves the error pointing at the field that failed.
    fn take(&mut self, n: usize) -> Result<&'a [u8], SemanticError> {
        let buf = self.buf;
        let end = self.at.saturating_add(n);
        if end > buf.len() {
            return Err(SemanticError::Short {
                method: self.method,
                at: self.at,
                need: n,
                len: buf.len(),
            });
        }
        let bytes = &buf[self.at..end];
        self.at = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], SemanticError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// Reads a u32 element count, then takes the `count * width` bytes it
    /// promises. The length check runs before any allocation, so a forged
    /// count cannot reserve memory the body does not hold.
    fn counted(&mut self, width: usize) -> Result<&'a [u8], SemanticError> {
        let count = self.u32()? as usize;
        self.take(count.saturating_mul(width))
    }

    /// One unsigned byte.
    pub fn u8(&mut self) -> Result<u8, SemanticError> {
        Ok(u8::from_le_bytes(self.array()?))
    }

    /// One signed byte.
    pub fn i8(&mut self) -> Result<i8, SemanticError> {
        Ok(i8::from_le_bytes(self.array()?))
    }

    /// A little-endian signed 32-bit integer.
    pub fn i32(&mut self) -> Result<i32, SemanticError> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    /// A little-endian unsigned 32-bit integer.
    pub fn u32(&mut self) -> Result<u32, SemanticError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// A little-endian unsigned 64-bit integer (`UINT64` on the wire).
    pub fn u64(&mut self) -> Result<u64, SemanticError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// A little-endian IEEE-754 single.
    pub fn f32(&mut self) -> Result<f32, SemanticError> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    /// A `WSTRING`: a u32 unit count, then that many UTF-16LE units. A lone
    /// surrogate is an error, not a replacement character, because the real
    /// client would not have sent one.
    pub fn wstring(&mut self) -> Result<String, SemanticError> {
        let (pairs, _) = self.counted(2)?.as_chunks::<2>();
        let units: Vec<u16> = pairs.iter().map(|&c| u16::from_le_bytes(c)).collect();
        String::from_utf16(&units).map_err(|_| SemanticError::BadWstring {
            method: self.method,
        })
    }

    /// An `ARRAY<INT32>`: a u32 element count, then that many i32s.
    pub fn array_i32(&mut self) -> Result<Vec<i32>, SemanticError> {
        let (quads, _) = self.counted(4)?.as_chunks::<4>();
        Ok(quads.iter().map(|&c| i32::from_le_bytes(c)).collect())
    }

    /// Succeeds only when every byte was read. Each decoder ends with this so
    /// a body longer than its layout fails instead of being silently cut.
    pub fn finish(self) -> Result<(), SemanticError> {
        let extra = self.buf.len() - self.at;
        if extra == 0 {
            return Ok(());
        }
        Err(SemanticError::Trailing {
            method: self.method,
            extra,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;
    use crate::semantic::SemanticError;

    #[test]
    fn u8_and_i8_read_one_byte_each_without_sign_mixup() {
        let mut r = Reader::new("test", &[0xFF, 0x80]);
        assert_eq!(r.u8(), Ok(255));
        assert_eq!(r.i8(), Ok(-128));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn i32_and_u32_read_little_endian() {
        let mut r = Reader::new("test", &[0xFE, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x80]);
        assert_eq!(r.i32(), Ok(-2));
        assert_eq!(r.u32(), Ok(0x8000_0001));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn u64_reads_eight_bytes_little_endian() {
        // Fixture #22 (InteractionType): 0x4000_0000 in the low half.
        let mut r = Reader::new("test", &[0x00, 0x00, 0x00, 0x40, 0, 0, 0, 0]);
        assert_eq!(r.u64(), Ok(0x4000_0000));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn f32_reads_ieee_754_bits() {
        let bytes = 1.5f32.to_le_bytes();
        let mut r = Reader::new("test", &bytes);
        assert_eq!(r.f32(), Ok(1.5));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn wstring_reads_system_from_fixture_5() {
        // First 16 bytes of fixture #5 (onPlayerCommunication): the count 6,
        // then "SYSTEM" as UTF-16LE.
        let bytes = [
            0x06, 0, 0, 0, 0x53, 0, 0x59, 0, 0x53, 0, 0x54, 0, 0x45, 0, 0x4D, 0,
        ];
        let mut r = Reader::new("test", &bytes);
        assert_eq!(r.wstring(), Ok("SYSTEM".to_string()));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn wstring_rejects_a_lone_surrogate() {
        // One unit, 0xD800: a high surrogate with no low half after it.
        let mut r = Reader::new("test", &[0x01, 0, 0, 0, 0x00, 0xD8]);
        assert_eq!(
            r.wstring(),
            Err(SemanticError::BadWstring { method: "test" })
        );
    }

    #[test]
    fn array_i32_reads_count_then_values() {
        let mut r = Reader::new(
            "test",
            &[0x02, 0, 0, 0, 0x07, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF],
        );
        assert_eq!(r.array_i32(), Ok(vec![7, -1]));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn array_count_past_the_end_is_short_and_allocates_nothing() {
        // The count says 1,000,000 elements; only four value bytes follow.
        let mut r = Reader::new("test", &[0x40, 0x42, 0x0F, 0x00, 0, 0, 0, 0]);
        assert_eq!(
            r.array_i32(),
            Err(SemanticError::Short {
                method: "test",
                at: 4,
                need: 4_000_000,
                len: 8,
            })
        );
    }

    #[test]
    fn short_read_reports_the_offset_and_the_length() {
        let mut r = Reader::new("test", &[1, 2, 3]);
        assert_eq!(
            r.u32(),
            Err(SemanticError::Short {
                method: "test",
                at: 0,
                need: 4,
                len: 3,
            })
        );
    }

    #[test]
    fn finish_rejects_a_leftover_byte() {
        let mut r = Reader::new("test", &[1, 2]);
        assert_eq!(r.u8(), Ok(1));
        assert_eq!(
            r.finish(),
            Err(SemanticError::Trailing {
                method: "test",
                extra: 1,
            })
        );
    }
}
