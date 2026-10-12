//! The one serializer of every `--json` line the commands print.
//!
//! A PowerShell pipe (`lab ... --json | ConvertFrom-Json`) decodes stdout with
//! the console's code page, so a UTF-8 `é` or emoji in a reason arrives as
//! garbage on a console that is not UTF-8. Every non-ASCII character is written
//! as a `\uXXXX` escape instead (a surrogate pair above U+FFFF), so the line is
//! pure ASCII and reads the same under any code page. It parses to the same
//! value as the unescaped text.

use std::io;

use serde::Serialize;
use serde_json::ser::Formatter;

/// Writes strings with every non-ASCII character as `\uXXXX` (what PowerShell's
/// `ConvertTo-Json -Compress` does for U+0085, U+2028 and U+2029, extended to
/// all of them); everything else is `serde_json`'s own.
pub struct PsFormatter;

impl Formatter for PsFormatter {
    fn write_string_fragment<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        let mut plain = 0;
        for (at, c) in fragment.char_indices() {
            if c.is_ascii() {
                continue;
            }
            writer.write_all(&fragment.as_bytes()[plain..at])?;
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                write!(writer, "\\u{unit:04x}")?;
            }
            plain = at + c.len_utf8();
        }
        writer.write_all(&fragment.as_bytes()[plain..])
    }
}

/// `value` as one compact JSON line (no trailing newline), pure ASCII.
pub fn to_ascii_json<T: Serialize + ?Sized>(value: &T) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, PsFormatter);
    // Only a map with a non-string key or a failing `Serialize` can error; the
    // command structs and `Value`s that come here have neither.
    if value.serialize(&mut serializer).is_err() {
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn non_ascii_is_escaped_and_round_trips() {
        let value =
            json!({"reason": "caf\u{e9} \u{1F600} \u{2028} \u{85} x\"y\\z\n", "k\u{e9}": 1});
        let text = to_ascii_json(&value);
        assert!(text.is_ascii(), "{text}");
        assert!(text.contains("caf\\u00e9"), "{text}");
        assert!(text.contains("\\ud83d\\ude00"), "{text}");
        assert!(
            text.contains("\\u2028") && text.contains("\\u0085"),
            "{text}"
        );
        let back: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(back, value);
    }

    #[test]
    fn plain_ascii_is_left_alone() {
        assert_eq!(
            to_ascii_json(&json!({"a": "b c", "n": 1})),
            r#"{"a":"b c","n":1}"#
        );
    }
}
