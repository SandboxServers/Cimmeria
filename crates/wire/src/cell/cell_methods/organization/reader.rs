//! A bounded little-endian argument reader for the organization decoders,
//! shared by the cell-method (8-19, 94) and base-method (0xCF-0xD2) paths.
//!
//! Every read checks the remaining payload first, and a `WSTRING`'s declared
//! unit count is bounded by the bytes actually left **before** anything is
//! allocated (D-ORG10), so a forged count of `0xFFFF_FFFF` costs nothing.

use std::fmt;

use cimmeria_entity::organization::TextReject;

/// Why an organization method's arguments did not decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgDecodeError {
    /// The payload ended before `field` (a `WSTRING`'s declared length
    /// counts: a count larger than the bytes left is reported here).
    Truncated {
        field: &'static str,
        need: usize,
        have: usize,
    },
    /// Bytes remained after the last declared argument.
    TrailingBytes { extra: usize },
    /// A `WSTRING` held an unpaired UTF-16 surrogate. D-ORG10 rejects these;
    /// [`OrgDecodeError::text_reject`] maps it for the player's feedback.
    LoneSurrogate { field: &'static str },
    /// The index is not an organization method.
    UnknownMethod(u16),
    /// `field` decoded but holds a value no legitimate client sends: a zero
    /// cash amount (CM 19) or a non-finite coordinate (CM 10).
    InvalidValue { field: &'static str },
}

impl OrgDecodeError {
    /// The D-ORG10 rejection this error stands for, if it is a text error
    /// rather than a malformed payload.
    pub fn text_reject(&self) -> Option<TextReject> {
        match self {
            OrgDecodeError::LoneSurrogate { .. } => Some(TextReject::LoneSurrogate),
            _ => None,
        }
    }

    /// Stable value for the `reason` log field.
    pub fn reason(&self) -> &'static str {
        match self {
            OrgDecodeError::Truncated { .. } => "truncated",
            OrgDecodeError::TrailingBytes { .. } => "trailing_bytes",
            OrgDecodeError::LoneSurrogate { .. } => "lone_surrogate",
            OrgDecodeError::UnknownMethod(_) => "unknown_method",
            OrgDecodeError::InvalidValue { .. } => "invalid_value",
        }
    }
}

impl fmt::Display for OrgDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrgDecodeError::Truncated { field, need, have } => {
                write!(f, "{field}: need {need} bytes, have {have}")
            }
            OrgDecodeError::TrailingBytes { extra } => {
                write!(f, "{extra} trailing bytes after the last argument")
            }
            OrgDecodeError::LoneSurrogate { field } => {
                write!(f, "{field}: unpaired UTF-16 surrogate")
            }
            OrgDecodeError::UnknownMethod(idx) => {
                write!(f, "method {idx} is not an organization method")
            }
            OrgDecodeError::InvalidValue { field } => write!(f, "{field}: invalid value"),
        }
    }
}

impl std::error::Error for OrgDecodeError {}

/// Cursor over one method's argument bytes.
pub(crate) struct ArgReader<'a> {
    buf: &'a [u8],
    off: usize,
}

impl<'a> ArgReader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        ArgReader { buf, off: 0 }
    }

    fn remaining(&self) -> usize {
        self.buf.len() - self.off
    }

    fn take<const N: usize>(&mut self, field: &'static str) -> Result<[u8; N], OrgDecodeError> {
        if self.remaining() < N {
            return Err(OrgDecodeError::Truncated {
                field,
                need: N,
                have: self.remaining(),
            });
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.off..self.off + N]);
        self.off += N;
        Ok(out)
    }

    pub(crate) fn u8(&mut self, field: &'static str) -> Result<u8, OrgDecodeError> {
        Ok(self.take::<1>(field)?[0])
    }

    pub(crate) fn i32(&mut self, field: &'static str) -> Result<i32, OrgDecodeError> {
        Ok(i32::from_le_bytes(self.take(field)?))
    }

    /// A finite `FLOAT`. NaN and the infinities are rejected: no client
    /// position holds them, and they poison every distance check downstream.
    pub(crate) fn finite_f32(&mut self, field: &'static str) -> Result<f32, OrgDecodeError> {
        let v = f32::from_le_bytes(self.take(field)?);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(OrgDecodeError::InvalidValue { field })
        }
    }

    /// `WSTRING`: a `u32` UTF-16 unit count, then the units.
    pub(crate) fn wstring(&mut self, field: &'static str) -> Result<String, OrgDecodeError> {
        let count = u32::from_le_bytes(self.take(field)?) as usize;
        // Bound the declared length by what is actually left before
        // allocating: a forged count must not size the buffer.
        let need = count.saturating_mul(2);
        if need > self.remaining() {
            return Err(OrgDecodeError::Truncated {
                field,
                need,
                have: self.remaining(),
            });
        }
        let units: Vec<u16> = self.buf[self.off..self.off + need]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&p| u16::from_le_bytes(p))
            .collect();
        self.off += need;
        String::from_utf16(&units).map_err(|_| OrgDecodeError::LoneSurrogate { field })
    }

    /// Require that every byte was consumed.
    pub(crate) fn finish(self) -> Result<(), OrgDecodeError> {
        match self.remaining() {
            0 => Ok(()),
            extra => Err(OrgDecodeError::TrailingBytes { extra }),
        }
    }
}
