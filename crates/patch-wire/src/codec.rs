//! The [`Encode`] / [`Decode`] traits, their errors, and the primitive
//! readers and writers the per-method codecs are built from.

use std::fmt;

use crate::source::{ByteSource, SliceSource};

/// Why a payload could not be decoded. `field` is always the `.def` name of
/// the argument or `FIXED_DICT` property being read, so a log line points
/// straight at the entity definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The input ended before `field` could be read.
    Truncated {
        /// The field being read.
        field: &'static str,
        /// Bytes the field (or, for an array, its remaining elements at
        /// their minimum size) needs.
        needed: usize,
        /// Bytes that were left.
        remaining: usize,
    },
    /// An `ARRAY` count exceeded the cap for that field.
    CountTooLarge {
        /// The array field.
        field: &'static str,
        /// The count on the wire.
        count: u32,
        /// The cap.
        max: u32,
    },
    /// A `STRING` length prefix exceeded the cap.
    StringTooLong {
        /// The string field.
        field: &'static str,
        /// The length on the wire, in bytes.
        len: u32,
        /// The cap, in bytes.
        max: u32,
    },
    /// A `STRING` body was not valid UTF-8.
    InvalidUtf8 {
        /// The string field.
        field: &'static str,
    },
    /// The source reported enough bytes but then failed to supply them.
    SourceFailed {
        /// The field being read.
        field: &'static str,
    },
    /// A whole-payload decode ([`Decode::decode`]) finished with bytes left
    /// over, which means the two sides disagree about the layout.
    TrailingBytes {
        /// Bytes the decoder consumed.
        consumed: usize,
        /// Bytes left unread.
        trailing: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated {
                field,
                needed,
                remaining,
            } => write!(
                f,
                "truncated at `{field}`: needs {needed} bytes, {remaining} left"
            ),
            Self::CountTooLarge { field, count, max } => {
                write!(f, "`{field}` count {count} exceeds the cap of {max}")
            }
            Self::StringTooLong { field, len, max } => {
                write!(f, "`{field}` length {len} exceeds the cap of {max} bytes")
            }
            Self::InvalidUtf8 { field } => write!(f, "`{field}` is not valid UTF-8"),
            Self::SourceFailed { field } => {
                write!(f, "the byte source failed while reading `{field}`")
            }
            Self::TrailingBytes { consumed, trailing } => write!(
                f,
                "{trailing} trailing bytes after a {consumed}-byte payload"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Why a value could not be encoded. The encoders enforce the same caps as
/// the decoders, so nothing one side writes is rejected by the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// A string is longer than the `STRING` cap.
    StringTooLong {
        /// The `.def` name of the string field.
        field: &'static str,
        /// The string's length in bytes.
        len: usize,
        /// The cap, in bytes.
        max: u32,
    },
    /// An array has more elements than its cap.
    CountTooLarge {
        /// The `.def` name of the array field.
        field: &'static str,
        /// The number of elements.
        count: usize,
        /// The cap.
        max: u32,
    },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StringTooLong { field, len, max } => {
                write!(f, "`{field}` is {len} bytes, over the cap of {max}")
            }
            Self::CountTooLarge { field, count, max } => {
                write!(f, "`{field}` has {count} elements, over the cap of {max}")
            }
        }
    }
}

impl std::error::Error for EncodeError {}

/// A wire value that names no variant of the enum it was parsed into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownEnumValue {
    /// The Rust enum being parsed.
    pub enum_name: &'static str,
    /// The value that matched nothing.
    pub value: i64,
}

impl fmt::Display for UnknownEnumValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} has no variant {}", self.enum_name, self.value)
    }
}

impl std::error::Error for UnknownEnumValue {}

/// A method's argument payload that can be written to the wire.
pub trait Encode {
    /// Append the encoding to `out`. May leave a partial encoding behind on
    /// error; call [`encode`](Self::encode) unless you are composing.
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError>;

    /// Append the encoding to `out`, leaving `out` unchanged on error.
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        let start = out.len();
        let result = self.write_to(out);
        if result.is_err() {
            out.truncate(start);
        }
        result
    }

    /// The encoding as a new buffer.
    fn to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        let mut out = Vec::new();
        self.write_to(&mut out)?;
        Ok(out)
    }
}

/// A method's argument payload that can be read back from the wire.
pub trait Decode: Sized {
    /// Read one value from `src`, consuming exactly its bytes. Bytes after
    /// it are left for the caller: the client's stream may carry more than
    /// one value's worth, and only the caller knows whether that is an
    /// error.
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError>;

    /// Decode a complete payload. Unlike [`decode_from`](Self::decode_from)
    /// this rejects trailing bytes, because on a whole payload they mean the
    /// two sides disagree about the layout.
    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        decode_whole(bytes, |src| Self::decode_from(src))
    }
}

/// Run `decode` over all of `bytes` and reject anything it leaves unread.
pub(crate) fn decode_whole<T>(
    bytes: &[u8],
    decode: impl FnOnce(&mut SliceSource<'_>) -> Result<T, DecodeError>,
) -> Result<T, DecodeError> {
    let mut src = SliceSource::new(bytes);
    let value = decode(&mut src)?;
    let trailing = src.remaining();
    if trailing != 0 {
        return Err(DecodeError::TrailingBytes {
            consumed: src.position(),
            trailing,
        });
    }
    Ok(value)
}

// ── primitive readers ────────────────────────────────────────────────────

/// Read `N` bytes, checking `remaining` first so an over-read is never
/// requested from the source.
fn read_bytes<const N: usize, S: ByteSource + ?Sized>(
    src: &mut S,
    field: &'static str,
) -> Result<[u8; N], DecodeError> {
    let remaining = src.remaining();
    if remaining < N {
        return Err(DecodeError::Truncated {
            field,
            needed: N,
            remaining,
        });
    }
    let mut buf = [0u8; N];
    src.read_into(&mut buf)
        .map_err(|_| DecodeError::SourceFailed { field })?;
    Ok(buf)
}

/// `UINT8`.
pub(crate) fn read_u8<S: ByteSource + ?Sized>(
    src: &mut S,
    field: &'static str,
) -> Result<u8, DecodeError> {
    Ok(read_bytes::<1, S>(src, field)?[0])
}

/// `INT32`, little-endian.
pub(crate) fn read_i32<S: ByteSource + ?Sized>(
    src: &mut S,
    field: &'static str,
) -> Result<i32, DecodeError> {
    Ok(i32::from_le_bytes(read_bytes::<4, S>(src, field)?))
}

/// The `u32` length or count prefix in front of a `STRING` or an `ARRAY`.
fn read_u32<S: ByteSource + ?Sized>(src: &mut S, field: &'static str) -> Result<u32, DecodeError> {
    Ok(u32::from_le_bytes(read_bytes::<4, S>(src, field)?))
}

/// Narrow `STRING`: `u32` byte length, then that many UTF-8 bytes. The
/// length is checked against `max` and against the bytes left before the
/// buffer is allocated.
pub(crate) fn read_string<S: ByteSource + ?Sized>(
    src: &mut S,
    field: &'static str,
    max: u32,
) -> Result<String, DecodeError> {
    let len = read_u32(src, field)?;
    if len > max {
        return Err(DecodeError::StringTooLong { field, len, max });
    }
    let len = len as usize;
    let remaining = src.remaining();
    if remaining < len {
        return Err(DecodeError::Truncated {
            field,
            needed: len,
            remaining,
        });
    }
    let mut buf = vec![0u8; len];
    src.read_into(&mut buf)
        .map_err(|_| DecodeError::SourceFailed { field })?;
    String::from_utf8(buf).map_err(|_| DecodeError::InvalidUtf8 { field })
}

/// An `ARRAY` count, checked against `max` and against the bytes left at
/// `min_element_len` bytes per element, so the caller can allocate
/// `count` elements without trusting the input.
pub(crate) fn read_count<S: ByteSource + ?Sized>(
    src: &mut S,
    field: &'static str,
    max: u32,
    min_element_len: usize,
) -> Result<usize, DecodeError> {
    let count = read_u32(src, field)?;
    if count > max {
        return Err(DecodeError::CountTooLarge { field, count, max });
    }
    let count = count as usize;
    // Cannot overflow: count <= max, and the caps are small.
    let needed = count.saturating_mul(min_element_len);
    let remaining = src.remaining();
    if remaining < needed {
        return Err(DecodeError::Truncated {
            field,
            needed,
            remaining,
        });
    }
    Ok(count)
}

// ── primitive writers ────────────────────────────────────────────────────

/// `UINT8`.
pub(crate) fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

/// `INT32`, little-endian.
pub(crate) fn write_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Narrow `STRING`: `u32` byte length, then the UTF-8 bytes.
pub(crate) fn write_string(
    out: &mut Vec<u8>,
    value: &str,
    field: &'static str,
    max: u32,
) -> Result<(), EncodeError> {
    let len = value.len();
    if len > max as usize {
        return Err(EncodeError::StringTooLong { field, len, max });
    }
    out.extend_from_slice(&(len as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

/// An `ARRAY` count prefix.
pub(crate) fn write_count(
    out: &mut Vec<u8>,
    count: usize,
    field: &'static str,
    max: u32,
) -> Result<(), EncodeError> {
    if count > max as usize {
        return Err(EncodeError::CountTooLarge { field, count, max });
    }
    out.extend_from_slice(&(count as u32).to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i32_is_little_endian_and_signed() {
        let mut out = Vec::new();
        write_i32(&mut out, -2);
        assert_eq!(out, [0xFE, 0xFF, 0xFF, 0xFF]);
        let mut src = SliceSource::new(&out);
        assert_eq!(read_i32(&mut src, "x"), Ok(-2));
    }

    #[test]
    fn string_is_u32_length_then_bytes() {
        let mut out = Vec::new();
        write_string(&mut out, "Ab", "s", 8).unwrap();
        assert_eq!(out, [0x02, 0x00, 0x00, 0x00, b'A', b'b']);
    }

    /// A length of `u32::MAX` is rejected on the cap, before anything is
    /// allocated for it.
    #[test]
    fn huge_string_length_is_rejected_before_allocating() {
        let bytes = [0xFF, 0xFF, 0xFF, 0xFF];
        let mut src = SliceSource::new(&bytes);
        assert_eq!(
            read_string(&mut src, "s", 16),
            Err(DecodeError::StringTooLong {
                field: "s",
                len: u32::MAX,
                max: 16
            })
        );
    }

    /// A count under the cap that the remaining bytes cannot hold is
    /// rejected before the caller allocates for it.
    #[test]
    fn count_is_checked_against_remaining_bytes() {
        let bytes = [0x03, 0x00, 0x00, 0x00, 0xAA, 0xBB];
        let mut src = SliceSource::new(&bytes);
        assert_eq!(
            read_count(&mut src, "a", 10, 4),
            Err(DecodeError::Truncated {
                field: "a",
                needed: 12,
                remaining: 2
            })
        );
    }

    #[test]
    fn write_count_enforces_the_cap() {
        let mut out = Vec::new();
        assert_eq!(
            write_count(&mut out, 5, "a", 4),
            Err(EncodeError::CountTooLarge {
                field: "a",
                count: 5,
                max: 4
            })
        );
        assert!(out.is_empty(), "nothing is written for a rejected count");
    }

    #[test]
    fn errors_display_the_field() {
        let e = DecodeError::Truncated {
            field: "sellerName",
            needed: 4,
            remaining: 1,
        };
        assert!(e.to_string().contains("sellerName"));
        let e = EncodeError::StringTooLong {
            field: "itemName",
            len: 300,
            max: 255,
        };
        assert!(e.to_string().contains("itemName"));
    }
}
