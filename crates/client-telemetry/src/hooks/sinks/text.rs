//! Pure text helpers shared by the log sinks: turning the bytes a client
//! log call carries into a line an event can hold, and the "shape" key the
//! rate limit groups lines by.
//!
//! Nothing here touches the game's memory, so all of it is unit-tested on
//! any target.

/// Longest message a sink keeps, in characters. BigWorld asserts carry a
/// file path and an expression, log4cxx lines a lock-trace sentence; 512
/// keeps every kind of line whole and bounds the event size.
pub const MAX_MESSAGE_CHARS: usize = 512;

/// Longest prefix of a message the rate limit's "shape" key uses.
const SHAPE_CHARS: usize = 64;

/// Decode the bytes of a narrow (ANSI) client string. The client is a
/// 2009 Windows build, so a line is either ASCII, UTF-8 (log4cxx built with
/// UTF-8 output) or the system code page; anything that is not valid UTF-8
/// is read as Latin-1, which keeps every byte visible and never fails.
pub fn decode_ansi(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// Decode UTF-16 code units, replacing unpaired surrogates.
pub fn decode_wide(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

/// Strip what a client adds around a line: trailing CR/LF and NULs, and a
/// leading run of the same. Interior line breaks stay (an assert message is
/// two lines).
pub fn trim_line(s: &str) -> &str {
    s.trim_matches(|c| matches!(c, '\r' | '\n' | '\0'))
}

/// Cut `s` to at most `max_chars` characters. The flag says whether
/// anything was cut.
pub fn truncate_chars(s: &str, max_chars: usize) -> (String, bool) {
    match s.char_indices().nth(max_chars) {
        Some((byte, _)) => (s[..byte].to_string(), true),
        None => (s.to_string(), false),
    }
}

/// Trim, then truncate to [`MAX_MESSAGE_CHARS`]: the message field of a
/// sink event.
pub fn message_field(raw: &str) -> (String, bool) {
    truncate_chars(trim_line(raw), MAX_MESSAGE_CHARS)
}

/// A key for "the same message with different numbers".
///
/// A lock trace ("Thread id 51428 holds write lock, num readers = 0")
/// differs on every line by a thread id or a counter; one rate-limit
/// bucket per exact text would never fill and would never throttle. So
/// every run of digits becomes `#`, a `0x` hex literal becomes `0x#`, and
/// the result is cut to a short prefix. Two lines with the same shape share
/// a bucket; two genuinely different messages do not, because their words
/// differ.
pub fn message_shape(s: &str) -> String {
    let mut out = String::with_capacity(SHAPE_CHARS + 4);
    let mut chars = s.chars().peekable();
    let mut count = 0usize;
    while let Some(c) = chars.next() {
        if count >= SHAPE_CHARS {
            break;
        }
        if c == '0' && matches!(chars.peek(), Some('x') | Some('X')) {
            // `0x` followed by at least one hex digit is a literal.
            let mut look = chars.clone();
            look.next();
            if look.peek().is_some_and(|d| d.is_ascii_hexdigit()) {
                chars.next();
                while chars.peek().is_some_and(|d| d.is_ascii_hexdigit()) {
                    chars.next();
                }
                out.push_str("0x#");
                count += 3;
                continue;
            }
        }
        if c.is_ascii_digit() {
            while chars.peek().is_some_and(|d| d.is_ascii_digit()) {
                chars.next();
            }
            out.push('#');
        } else {
            out.push(c);
        }
        count += 1;
    }
    out
}

/// Hex text for an address field: `0x019cdf80`.
pub fn hex32(v: usize) -> String {
    format!("0x{v:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_is_utf8_when_it_can_be_and_latin1_otherwise() {
        assert_eq!(decode_ansi(b"plain"), "plain");
        assert_eq!(decode_ansi("caf\u{e9}".as_bytes()), "caf\u{e9}");
        // 0xE9 alone is invalid UTF-8; as Latin-1 it is e-acute.
        assert_eq!(decode_ansi(&[b'c', b'a', b'f', 0xE9]), "caf\u{e9}");
    }

    #[test]
    fn wide_text_replaces_unpaired_surrogates() {
        assert_eq!(decode_wide(&[0x48, 0x69]), "Hi");
        assert_eq!(decode_wide(&[0x48, 0xD800, 0x69]), "H\u{fffd}i");
    }

    #[test]
    fn trimming_strips_line_ends_and_nuls_at_the_edges_only() {
        assert_eq!(trim_line("a b\r\n"), "a b");
        assert_eq!(trim_line("\0\0x\n\n"), "x");
        assert_eq!(trim_line("line1\nline2\n"), "line1\nline2");
        assert_eq!(trim_line(""), "");
    }

    #[test]
    fn truncation_counts_characters_not_bytes() {
        assert_eq!(truncate_chars("abc", 5), ("abc".to_string(), false));
        assert_eq!(truncate_chars("abc", 3), ("abc".to_string(), false));
        assert_eq!(truncate_chars("abcd", 3), ("abc".to_string(), true));
        // Two-byte characters: cutting at 2 characters must not split one.
        assert_eq!(
            truncate_chars("\u{e9}\u{e9}\u{e9}", 2),
            ("\u{e9}\u{e9}".to_string(), true)
        );
    }

    #[test]
    fn the_message_field_trims_then_bounds() {
        let (m, cut) = message_field("hello\r\n");
        assert_eq!((m.as_str(), cut), ("hello", false));
        let long = "x".repeat(MAX_MESSAGE_CHARS + 10);
        let (m, cut) = message_field(&long);
        assert_eq!(m.chars().count(), MAX_MESSAGE_CHARS);
        assert!(cut);
    }

    /// The real client's lock trace: every line differs by a number, and all
    /// of them must share one rate-limit bucket. Without the shape they
    /// would each get a fresh bucket and 65 000 lines a session would all
    /// go out.
    #[test]
    fn lock_trace_lines_share_a_shape_across_thread_ids() {
        let a = message_shape("Thread id 51428 holds write lock, num readers = 0 num writers = 1");
        let b = message_shape("Thread id 22296 holds write lock, num readers = 0 num writers = 2");
        assert_eq!(a, b);
        assert_eq!(
            a,
            "Thread id # holds write lock, num readers = # num writers = #"
        );
    }

    #[test]
    fn different_messages_have_different_shapes() {
        assert_ne!(
            message_shape("inside writeLock"),
            message_shape("finished writeLock, num readers = 0")
        );
        assert_ne!(
            message_shape("Error opening static cache archive A.pak"),
            message_shape("Error opening static cache archive B.pak")
        );
    }

    #[test]
    fn hex_literals_collapse_and_a_lone_zero_x_does_not() {
        assert_eq!(message_shape("at 0x00ab12cd in f"), "at 0x# in f");
        assert_eq!(message_shape("at 0xDEADBEEF"), "at 0x#");
        // `0x` with no hex digit after it is a zero and an `x`.
        assert_eq!(message_shape("0xg"), "#xg");
        assert_eq!(message_shape("a0"), "a#");
    }

    #[test]
    fn a_shape_is_bounded() {
        let long = "abcdefghij".repeat(50);
        assert!(message_shape(&long).chars().count() <= SHAPE_CHARS + 3);
    }

    #[test]
    fn addresses_print_as_eight_hex_digits() {
        assert_eq!(hex32(0x019c_df80), "0x019cdf80");
        assert_eq!(hex32(0x10), "0x00000010");
    }
}
