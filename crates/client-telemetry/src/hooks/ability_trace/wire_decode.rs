//! A type-driven decoder for BigWorld method arguments, as the server
//! writes them and the client's `MemoryIStream` holds them.
//!
//! The encodings are the ones `cimmeria-wire` writes (and its byte tests
//! pin): integers and `FLOAT` little-endian at their natural width, an
//! `ARRAY` as a `u32` element count and then the elements, a `WSTRING` as
//! a `u32` character count and then UTF-16LE, a `FIXED_DICT` as its
//! fields in declaration order with nothing between them. The decoder is
//! driven by a static description of each method's `.def` argument list
//! ([`Arg`], [`WireType`]), so a table row is the whole decoder for a
//! method; `recv_methods` holds the rows and a test checks them against
//! `entities/defs/`.
//!
//! Everything is bounded: arrays are cut at [`MAX_ELEMENTS`] (the count
//! is still reported), strings at [`MAX_STRING_CHARS`], and a count that
//! claims more bytes than remain fails the decode instead of reading on.

use serde_json::{json, Value};

/// Elements kept per array. The rest are skipped (not decoded), and the
/// array's `*_count` field still says how many there were.
pub(crate) const MAX_ELEMENTS: usize = 32;
/// Characters kept per string.
pub(crate) const MAX_STRING_CHARS: usize = 256;

/// A wire type, as the `.def` / `alias.xml` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WireType {
    /// `INT8`.
    I8,
    /// `UINT8`.
    U8,
    /// `UINT16`.
    U16,
    /// `INT32`.
    I32,
    /// `FLOAT`.
    F32,
    /// `WSTRING`: `u32` character count, then UTF-16LE.
    WString,
    /// `ARRAY <of> T </of>`: `u32` count, then the elements.
    Array(&'static WireType),
    /// `FIXED_DICT`: the fields in order. Decoded as a JSON array of the
    /// field values, in the same order, to keep rows compact.
    Dict(&'static [Field]),
    /// A named `alias.xml` type; decodes as the type it names.
    Alias(&'static str, &'static WireType),
}

/// One `FIXED_DICT` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Field {
    /// The property name in `alias.xml`.
    pub name: &'static str,
    /// Its type.
    pub ty: WireType,
}

/// One method argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Arg {
    /// The event field it becomes (snake case).
    pub field: &'static str,
    /// The `.def` `ArgName`.
    pub def_name: &'static str,
    /// Its wire type.
    pub ty: WireType,
}

/// Why a decode stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DecodeError {
    /// The bytes ran out inside argument `arg`.
    Truncated {
        /// The `.def` name of the argument being read.
        arg: &'static str,
    },
    /// Every argument decoded and bytes were left over: the payload is
    /// longer than its `.def` says, so the decode may be misaligned.
    TrailingBytes {
        /// How many bytes were left.
        n: usize,
    },
}

impl DecodeError {
    /// Short text for the `decode_error` field.
    pub(crate) fn as_text(&self) -> String {
        match self {
            DecodeError::Truncated { arg } => format!("truncated in {arg}"),
            DecodeError::TrailingBytes { .. } => "trailing_bytes".to_string(),
        }
    }

    /// The left-over byte count, for the `trailing_bytes` field.
    pub(crate) fn trailing(&self) -> Option<usize> {
        match self {
            DecodeError::TrailingBytes { n } => Some(*n),
            DecodeError::Truncated { .. } => None,
        }
    }
}

/// A cursor over the argument bytes.
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Bytes not yet read.
    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// One value of type `ty`, or `None` when the bytes run out.
    fn value(&mut self, ty: &WireType) -> Option<Value> {
        Some(match ty {
            WireType::I8 => json!(self.take(1)?[0] as i8),
            WireType::U8 => json!(self.take(1)?[0]),
            WireType::U16 => {
                let b = self.take(2)?;
                json!(u16::from_le_bytes([b[0], b[1]]))
            }
            WireType::I32 => json!(self.u32()? as i32),
            WireType::F32 => float(f32::from_bits(self.u32()?)),
            WireType::WString => {
                let chars = self.u32()? as usize;
                let bytes = self.take(chars.checked_mul(2)?)?;
                let units: Vec<u16> = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .take(MAX_STRING_CHARS)
                    .map(|c| u16::from_le_bytes(*c))
                    .collect();
                json!(String::from_utf16_lossy(&units))
            }
            WireType::Array(inner) => {
                let count = self.u32()? as usize;
                // A count no remaining byte could satisfy is corrupt: stop
                // before looping on it.
                if count.saturating_mul(min_size(inner)) > self.remaining() {
                    return None;
                }
                let mut out = Vec::with_capacity(count.min(MAX_ELEMENTS));
                for i in 0..count {
                    let v = self.value(inner)?;
                    if i < MAX_ELEMENTS {
                        out.push(v);
                    }
                }
                Value::Array(out)
            }
            WireType::Dict(fields) => Value::Array(
                fields
                    .iter()
                    .map(|f| self.value(&f.ty))
                    .collect::<Option<Vec<_>>>()?,
            ),
            WireType::Alias(_, inner) => self.value(inner)?,
        })
    }
}

/// The fewest bytes one value of `ty` takes.
fn min_size(ty: &WireType) -> usize {
    match ty {
        WireType::I8 | WireType::U8 => 1,
        WireType::U16 => 2,
        WireType::I32 | WireType::F32 => 4,
        WireType::WString | WireType::Array(_) => 4,
        WireType::Dict(fields) => fields.iter().map(|f| min_size(&f.ty)).sum(),
        WireType::Alias(_, inner) => min_size(inner),
    }
}

/// A float as JSON, rounded to 3 decimals so `0.1f32` reads as `0.1`. A
/// non-finite value (which JSON cannot hold) is reported as a string.
pub(crate) fn float(v: f32) -> Value {
    if v.is_finite() {
        json!((f64::from(v) * 1000.0).round() / 1000.0)
    } else {
        json!(v.to_string())
    }
}

/// The count an array argument declared, read without decoding it: the
/// first `u32` at its position. Only meaningful for an array argument.
fn array_count(bytes: &[u8]) -> Option<u32> {
    bytes
        .get(..4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Decode `args` from `bytes`. Each argument becomes `(field, value)`; an
/// array argument also gets `<field>_count`, its declared length. On
/// failure the arguments decoded so far are returned with the error. The
/// bytes must be used up exactly: anything left after the last argument
/// is [`DecodeError::TrailingBytes`] (the values are still returned), so a
/// payload that is longer than its `.def` never passes as a clean decode.
pub(crate) fn decode(
    args: &[Arg],
    bytes: &[u8],
) -> (Vec<(&'static str, Value)>, Option<DecodeError>) {
    let mut r = Reader::new(bytes);
    let mut out = Vec::with_capacity(args.len() + 2);
    for a in args {
        let at = r.pos;
        let Some(v) = r.value(&a.ty) else {
            // An array cut short (a payload capped at `MAX_ARG_BYTES`, or a
            // truncated one) still reports its declared count and the
            // elements that are there.
            if let Some((count, partial)) = partial_array(&a.ty, &bytes[at..]) {
                out.push((count_field(a.field), json!(count)));
                out.push((a.field, partial));
            }
            return (out, Some(DecodeError::Truncated { arg: a.def_name }));
        };
        if is_array(&a.ty) {
            if let Some(n) = array_count(&bytes[at..]) {
                out.push((count_field(a.field), json!(n)));
            }
        }
        out.push((a.field, v));
    }
    let left = r.remaining();
    (
        out,
        (left > 0).then_some(DecodeError::TrailingBytes { n: left }),
    )
}

/// The element type of an array type, through aliases.
fn array_element(ty: &WireType) -> Option<&WireType> {
    match ty {
        WireType::Array(inner) => Some(inner),
        WireType::Alias(_, inner) => array_element(inner),
        _ => None,
    }
}

/// An array whose elements run out before its declared count: the count
/// and the first [`MAX_ELEMENTS`] whole elements. Every element takes at
/// least one byte, so the loop ends when the bytes do.
fn partial_array(ty: &WireType, bytes: &[u8]) -> Option<(u32, Value)> {
    let inner = array_element(ty)?;
    let count = array_count(bytes)?;
    let mut r = Reader::new(&bytes[4..]);
    let mut out = Vec::new();
    for _ in 0..count {
        let Some(v) = r.value(inner) else { break };
        if out.len() < MAX_ELEMENTS {
            out.push(v);
        }
    }
    Some((count, Value::Array(out)))
}

fn is_array(ty: &WireType) -> bool {
    match ty {
        WireType::Array(_) => true,
        WireType::Alias(_, inner) => is_array(inner),
        _ => false,
    }
}

/// `results` becomes `results_count`. The names are static, so the table
/// of array fields is closed; anything else gets a generic name.
fn count_field(field: &'static str) -> &'static str {
    match field {
        "results" => "results_count",
        "stats" => "stats_count",
        "nvps" => "nvps_count",
        "ability_ids" => "ability_ids_count",
        "ability_lists" => "ability_lists_count",
        _ => "array_count",
    }
}

/// The `.def` spelling of a type, with whitespace removed: `INT32`,
/// `ARRAY<of>INT32</of>`, or an alias name. Used by the conformance tests.
#[cfg(test)]
pub(crate) fn def_spelling(ty: &WireType) -> String {
    match ty {
        WireType::I8 => "INT8".into(),
        WireType::U8 => "UINT8".into(),
        WireType::U16 => "UINT16".into(),
        WireType::I32 => "INT32".into(),
        WireType::F32 => "FLOAT".into(),
        WireType::WString => "WSTRING".into(),
        WireType::Array(inner) => format!("ARRAY<of>{}</of>", def_spelling(inner)),
        WireType::Dict(_) => "FIXED_DICT".into(),
        WireType::Alias(name, _) => (*name).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DICT: WireType = WireType::Dict(&[
        Field {
            name: "A",
            ty: WireType::I8,
        },
        Field {
            name: "B",
            ty: WireType::I32,
        },
    ]);
    const LIST: WireType = WireType::Array(&DICT);

    fn arg(field: &'static str, ty: WireType) -> Arg {
        Arg {
            field,
            def_name: field,
            ty,
        }
    }

    #[test]
    fn scalars_are_little_endian_at_their_width() {
        let bytes = [
            0xff, // i8 -1
            0x80, // u8 128
            0x34, 0x12, // u16 0x1234
            0x01, 0x00, 0x00, 0x80, // i32 i32::MIN + 1
            0x00, 0x00, 0xc0, 0x3f, // f32 1.5
        ];
        let args = [
            arg("a", WireType::I8),
            arg("b", WireType::U8),
            arg("d", WireType::U16),
            arg("e", WireType::I32),
            arg("f", WireType::F32),
        ];
        let (v, err) = decode(&args, &bytes);
        assert_eq!(err, None);
        assert_eq!(
            v,
            vec![
                ("a", json!(-1)),
                ("b", json!(128)),
                ("d", json!(0x1234)),
                ("e", json!(i32::MIN + 1)),
                ("f", json!(1.5)),
            ]
        );
    }

    #[test]
    fn a_wstring_is_a_char_count_then_utf16() {
        let mut bytes = 3u32.to_le_bytes().to_vec();
        for u in "Hé!".encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        let (v, err) = decode(&[arg("s", WireType::WString)], &bytes);
        assert_eq!(err, None);
        assert_eq!(v, vec![("s", json!("Hé!"))]);
    }

    #[test]
    fn an_array_of_dicts_decodes_to_rows_and_reports_its_count() {
        let mut bytes = 2u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[7, 10, 0, 0, 0, 0xf9, 0xff, 0xff, 0xff, 0xff]);
        let (v, err) = decode(&[arg("results", LIST)], &bytes);
        assert_eq!(err, None);
        assert_eq!(
            v,
            vec![
                ("results_count", json!(2)),
                ("results", json!([[7, 10], [-7, -1]])),
            ]
        );
    }

    /// A long array is decoded through (so later arguments still line up)
    /// but only the first `MAX_ELEMENTS` are kept.
    #[test]
    fn a_long_array_is_cut_but_later_args_still_align() {
        let n = MAX_ELEMENTS as u32 + 5;
        let mut bytes = n.to_le_bytes().to_vec();
        for i in 0..n {
            bytes.extend_from_slice(&(i as i32).to_le_bytes());
        }
        bytes.push(9);
        let args = [
            arg("ability_ids", WireType::Array(&WireType::I32)),
            arg("tail", WireType::U8),
        ];
        let (v, err) = decode(&args, &bytes);
        assert_eq!(err, None);
        assert_eq!(v[0], ("ability_ids_count", json!(n)));
        assert_eq!(v[1].1.as_array().unwrap().len(), MAX_ELEMENTS);
        assert_eq!(v[2], ("tail", json!(9)));
    }

    /// Short bytes stop the decode and say where, keeping what was read.
    #[test]
    fn truncation_names_the_argument() {
        let bytes = [1, 0, 0, 0, 2, 0];
        let args = [arg("a", WireType::I32), arg("b", WireType::I32)];
        let (v, err) = decode(&args, &bytes);
        assert_eq!(v, vec![("a", json!(1))]);
        assert_eq!(err, Some(DecodeError::Truncated { arg: "b" }));
        assert_eq!(err.unwrap().as_text(), "truncated in b");
    }

    /// An array whose bytes stop early (a capped payload) still reports
    /// its declared count and the elements that arrived.
    #[test]
    fn a_cut_array_keeps_its_count_and_prefix() {
        let mut bytes = 2000u32.to_le_bytes().to_vec();
        for i in 0..100i32 {
            bytes.extend_from_slice(&i.to_le_bytes());
        }
        bytes.extend_from_slice(&[0xff, 0xff]); // half an element
        let (v, err) = decode(
            &[arg("ability_ids", WireType::Array(&WireType::I32))],
            &bytes,
        );
        assert_eq!(err, Some(DecodeError::Truncated { arg: "ability_ids" }));
        assert_eq!(v[0], ("ability_ids_count", json!(2000)));
        let ids = v[1].1.as_array().unwrap();
        assert_eq!(ids.len(), MAX_ELEMENTS);
        assert_eq!(ids[31], json!(31));
    }

    /// Bytes left after the last argument are an error, with the count,
    /// and the decoded values are kept.
    #[test]
    fn trailing_bytes_are_an_error() {
        let bytes = [1, 0, 0, 0, 0xaa, 0xbb];
        let (v, err) = decode(&[arg("a", WireType::I32)], &bytes);
        assert_eq!(v, vec![("a", json!(1))]);
        let err = err.expect("two bytes were left over");
        assert_eq!(err, DecodeError::TrailingBytes { n: 2 });
        assert_eq!(
            (err.as_text().as_str(), err.trailing()),
            ("trailing_bytes", Some(2))
        );
        // An exact payload is clean.
        assert_eq!(decode(&[arg("a", WireType::I32)], &bytes[..4]).1, None);
    }

    /// A corrupt count (four billion elements in eight bytes) fails at
    /// once instead of looping.
    #[test]
    fn an_impossible_count_fails_fast() {
        let mut bytes = u32::MAX.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0; 4]);
        let (_, err) = decode(&[arg("results", LIST)], &bytes);
        assert!(err.is_some());
        let mut s = u32::MAX.to_le_bytes().to_vec();
        s.extend_from_slice(&[0x41, 0]);
        let (_, err) = decode(&[arg("s", WireType::WString)], &s);
        assert!(err.is_some());
    }

    #[test]
    fn floats_are_rounded_and_non_finite_is_text() {
        assert_eq!(float(0.1), json!(0.1));
        assert_eq!(float(12.3456), json!(12.346));
        assert_eq!(float(f32::INFINITY), json!("inf"));
    }

    #[test]
    fn def_spellings_match_the_def_files() {
        assert_eq!(def_spelling(&WireType::I32), "INT32");
        assert_eq!(
            def_spelling(&WireType::Array(&WireType::Array(&WireType::I32))),
            "ARRAY<of>ARRAY<of>INT32</of></of>"
        );
        assert_eq!(
            def_spelling(&WireType::Alias("StatUpdateList", &LIST)),
            "StatUpdateList"
        );
    }
}
