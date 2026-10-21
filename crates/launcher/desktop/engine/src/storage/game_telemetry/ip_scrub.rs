//! IP-literal guard (launcher-telemetry LT-40, directive 6). No launcher row carries an
//! IPv4 or IPv6 address: `scrub_ips` takes it out of every forwarded string, `scrub_event`
//! does the same for every string of a queued `ClientNative` row, and the C9 capture
//! helpers fail a test whose event still holds one. Nothing is hashed.

use cimmeria_launcher_core::telemetry::events::TelemetryEvent;
use serde_json::Value;
use std::borrow::Cow;
use std::net::IpAddr;

/// True when `s` holds an IPv4 or IPv6 literal. The rule is the one [`scrub_ips`] uses.
pub fn contains_ip(s: &str) -> bool {
    tokens(s)
        .iter()
        .any(|&(start, token)| ip_span(s, start, token).is_some())
}

/// Replace every IP literal with "[ip]". Borrowed when nothing changes. Only the
/// address is replaced: a `:port`, a `/path` or a trailing `.` after it stays.
/// An address may start a token or follow any `:` in it, and may be bracketed. A pure
/// IPv6 form (a `:` and no `.`) counts only with a digit and not glued to a word, so
/// `std::io::Error` and `fade::bead` pass through.
pub fn scrub_ips(s: &str) -> Cow<'_, str> {
    if !contains_ip(s) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for (start, token) in tokens(s) {
        if let Some((from, to)) = ip_span(s, start, token) {
            out.push_str(&s[last..from]);
            out.push_str("[ip]");
            last = to;
        }
    }
    out.push_str(&s[last..]);
    Cow::Owned(out)
}

/// Scrub every string of a `ClientNative` event (target, level, and each `fields`
/// value, nested arrays and objects included). Other variants carry no launcher
/// strings and pass through. The queue entry points call this, so rows built outside
/// `launcher_event` are covered too.
pub fn scrub_event(event: &mut TelemetryEvent) {
    if let TelemetryEvent::ClientNative(native) = event {
        scrub_in_place(&mut native.target);
        scrub_in_place(&mut native.level);
        native.fields.values_mut().for_each(scrub_value);
    }
}

fn scrub_value(value: &mut Value) {
    match value {
        Value::String(text) => scrub_in_place(text),
        Value::Array(items) => items.iter_mut().for_each(scrub_value),
        Value::Object(map) => map.values_mut().for_each(scrub_value),
        _ => {}
    }
}

fn scrub_in_place(text: &mut String) {
    if contains_ip(text) {
        *text = scrub_ips(text).into_owned();
    }
}

/// The maximal runs of token characters in `s`, with their byte offsets.
fn tokens(s: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices() {
        match (is_token_char(c), start) {
            (true, None) => start = Some(i),
            (false, Some(from)) => {
                out.push((from, &s[from..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        out.push((from, &s[from..]));
    }
    out
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_hexdigit() || matches!(c, ':' | '.' | '[' | ']' | '%')
}

/// The absolute byte range in `s` of the IP literal in the token that starts at
/// `token_start`, or None when there is none. The token is tried as it is, then with
/// its leading `:` and `.` dropped, so `host:198.51.100.7` matches and `::1` keeps
/// its colons.
fn ip_span(s: &str, token_start: usize, token: &str) -> Option<(usize, usize)> {
    let trimmed = token.len() - token.trim_start_matches([':', '.']).len();
    [0, trimmed]
        .into_iter()
        .find_map(|lead| address_in(s, token_start + lead, &token[lead..]))
}

/// The first address in `body` (which starts at `base` in `s`), after dropping a
/// trailing `.`. Each start is tried: the text itself, then the text after each `:`,
/// so `addr:2001:db8::1` finds `2001:db8::1`.
fn address_in(s: &str, base: usize, body: &str) -> Option<(usize, usize)> {
    let body = body.trim_end_matches('.');
    for start in std::iter::once(0).chain(body.match_indices(':').map(|(i, _)| i + 1)) {
        if let Some((from, to)) = candidate(&body[start..]) {
            let (from, to) = (base + start + from, base + start + to);
            if plausible(s, from, to) {
                return Some((from, to));
            }
        }
    }
    None
}

/// The address at the start of `rest`, as a range inside `rest`: the `[addr]` form
/// (an unclosed `[addr` runs to the end), then the whole text, then the text with one
/// trailing `:port` removed.
fn candidate(rest: &str) -> Option<(usize, usize)> {
    if let Some(inner) = rest.strip_prefix('[') {
        return match inner.find(']') {
            Some(end) => parses(&inner[..end]).then_some((0, end + 2)),
            None => parses(inner).then_some((0, rest.len())),
        };
    }
    if parses(rest) {
        return Some((0, rest.len()));
    }
    let (head, _port) = rest.rsplit_once(':')?;
    parses(head).then_some((0, head.len()))
}

/// The IPv6-only guard. A span with a `:` and no `.` must hold a digit and must not be
/// glued to a letter, digit or `_` before it. `::` and hex-only words such as `a::b`
/// or `Error::E` fail it. Spans with a `.` (IPv4 and IPv4-mapped IPv6) always pass.
fn plausible(s: &str, from: usize, to: usize) -> bool {
    let span = &s[from..to];
    if !span.contains(':') || span.contains('.') {
        return true;
    }
    let glued = s[..from]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    span.bytes().any(|b| b.is_ascii_digit()) && !glued
}

fn parses(addr: &str) -> bool {
    let bare = addr.split('%').next().unwrap_or(addr);
    bare.parse::<IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn contains_ip_finds_ipv4_and_ipv6_literals() {
        for s in [
            "198.51.100.7",
            "http://198.51.100.7:8081/x",
            "[2001:db8::1]:8081",
            "fe80::1%3",
        ] {
            assert!(contains_ip(s), "{s}");
        }
    }

    #[test]
    fn contains_ip_finds_an_address_after_a_key_or_a_sentence_dot() {
        for s in [
            "host:198.51.100.7",
            "peer:198.51.100.7:8081",
            "to:[2001:db8::1]:8081",
            "addr:2001:db8::1",
            "addr 2001:db8::1.",
            "connect to 198.51.100.7.",
            "::ffff:198.51.100.7",
            "[::ffff:198.51.100.7]:8081",
            "[::1",
        ] {
            assert!(contains_ip(s), "{s}");
        }
    }

    #[test]
    fn contains_ip_finds_pure_ipv6_forms_with_a_digit() {
        for s in ["::1", "fe80::1", "peer:2001:db8::1"] {
            assert!(contains_ip(s), "{s}");
        }
    }

    #[test]
    fn contains_ip_ignores_rust_paths_and_hex_words() {
        for s in [
            "reqwest::Error { kind: Request }",
            "std::io::Error",
            "a::b",
            "fade::bead",
            "Error::Io",
            "launcher_core::install::Error",
        ] {
            assert!(!contains_ip(s), "{s}");
        }
    }

    #[test]
    fn contains_ip_ignores_versions_times_hashes_and_paths() {
        let sha = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        for s in [
            "0.9.3",
            "10.0.26300.0",
            "12:34:56",
            "launcher.install",
            sha,
            r"C:\Users\x",
        ] {
            assert!(!contains_ip(s), "{s}");
        }
    }

    #[test]
    fn scrub_ips_replaces_the_address_and_keeps_the_port() {
        assert_eq!(
            scrub_ips("error at 198.51.100.7:8081"),
            "error at [ip]:8081"
        );
        assert_eq!(
            scrub_ips("http://198.51.100.7:8081/x"),
            "http://[ip]:8081/x"
        );
        assert_eq!(scrub_ips("[2001:db8::1]:8081"), "[ip]:8081");
        assert_eq!(scrub_ips("fe80::1%3"), "[ip]");
    }

    #[test]
    fn scrub_ips_keeps_the_key_and_the_sentence_dot() {
        assert_eq!(scrub_ips("peer:198.51.100.7:8081"), "peer:[ip]:8081");
        assert_eq!(scrub_ips("connect to 198.51.100.7."), "connect to [ip].");
        assert_eq!(scrub_ips("to:[2001:db8::1]:8081"), "to:[ip]:8081");
    }

    #[test]
    fn scrub_ips_leaves_rust_paths_alone() {
        assert!(matches!(scrub_ips("std::io::Error"), Cow::Borrowed(_)));
        assert!(matches!(
            scrub_ips("reqwest::Error { kind: Request }"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn scrub_ips_borrows_when_nothing_changes() {
        assert!(matches!(
            scrub_ips("launcher.install 0.9.3"),
            Cow::Borrowed(_)
        ));
        assert!(matches!(scrub_ips("198.51.100.7"), Cow::Owned(_)));
    }

    #[test]
    fn scrub_event_scrubs_every_string_of_a_client_native_row() {
        let mut event = TelemetryEvent::ClientNative(
            cimmeria_launcher_core::telemetry::events::ClientNativeEvent {
                ts_ms: 0,
                seq: 0,
                target: "launcher.install".into(),
                level: "info".into(),
                fields: json!({
                    "error": "connect to 198.51.100.7 failed",
                    "nested": ["peer:198.51.100.7:8081", 3],
                    "path": "reqwest::Error",
                })
                .as_object()
                .unwrap()
                .clone(),
            },
        );
        scrub_event(&mut event);
        let TelemetryEvent::ClientNative(native) = event else {
            panic!("expected ClientNative");
        };
        assert_eq!(native.fields["error"], "connect to [ip] failed");
        assert_eq!(native.fields["nested"], json!(["peer:[ip]:8081", 3]));
        assert_eq!(native.fields["path"], "reqwest::Error");
    }
}
