//! IP-literal guard (launcher-telemetry LT-40, directive 6). No launcher row carries an
//! IPv4 or IPv6 address: `scrub_ips` takes it out of every forwarded string, and the C9
//! capture helpers fail a test whose event still holds one. Nothing is hashed.

use std::borrow::Cow;
use std::net::IpAddr;

/// True when `s` holds an IPv4 or IPv6 literal: a token split on chars outside
/// `[0-9A-Fa-f:.\[\]%]` that parses as `std::net::IpAddr` after stripping brackets
/// and a `%zone`, either whole or with one trailing `:port` removed (try whole first,
/// so `fe80::1` is not cut to `fe80:`). A `[addr]` or `[addr]:port` token parses
/// only the text between the brackets.
pub fn contains_ip(s: &str) -> bool {
    tokens(s).iter().any(|(_, token)| ip_span(token).is_some())
}

/// Replace every such token with "[ip]". Borrowed when nothing changes. Only the
/// address is replaced: a `:port` or a `/path` after it stays.
pub fn scrub_ips(s: &str) -> Cow<'_, str> {
    if !contains_ip(s) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for (start, token) in tokens(s) {
        if let Some((from, to)) = ip_span(token) {
            out.push_str(&s[last..start + from]);
            out.push_str("[ip]");
            last = start + to;
        }
    }
    out.push_str(&s[last..]);
    Cow::Owned(out)
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

/// The byte range of the address inside `token`, or None when the token holds no IP.
fn ip_span(token: &str) -> Option<(usize, usize)> {
    if let Some(inner) = token.strip_prefix('[') {
        let end = inner.find(']')?;
        return parses(&inner[..end]).then_some((0, end + 2));
    }
    if parses(token) {
        return Some((0, token.len()));
    }
    let (head, _port) = token.rsplit_once(':')?;
    parses(head).then_some((0, head.len()))
}

fn parses(addr: &str) -> bool {
    let bare = addr.split('%').next().unwrap_or(addr);
    bare.parse::<IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn scrub_ips_borrows_when_nothing_changes() {
        assert!(matches!(
            scrub_ips("launcher.install 0.9.3"),
            Cow::Borrowed(_)
        ));
        assert!(matches!(scrub_ips("198.51.100.7"), Cow::Owned(_)));
    }
}
