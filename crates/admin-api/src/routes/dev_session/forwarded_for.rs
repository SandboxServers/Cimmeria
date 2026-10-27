//! The address a per-IP quota or a per-peer upload slot counts (C18, D-LT19).
//!
//! The colo's board proxy forwards the https routes from loopback, so every
//! such caller shares the proxy's address. A loopback peer's
//! `X-Forwarded-For` names the caller instead, and that is what the quotas
//! key on. The result is a quota key and the in-memory client-telemetry
//! stream address (D-LTL4); it is never logged, persisted or put in a field.

use std::net::IpAddr;

use axum::http::HeaderMap;

/// The header a loopback proxy sets.
const FORWARDED_FOR: &str = "x-forwarded-for";

/// D-LT19 (directive 7a). The address a per-IP quota or a per-peer upload
/// slot counts. When `peer` is loopback (the board proxy's Caddy forwards to
/// loopback :8081) and the request has an `X-Forwarded-For` header, this is
/// the rightmost comma-separated entry of the last such header, trimmed, if
/// it parses as a bare `IpAddr`. In every other case it is `peer`. A
/// non-loopback peer's header is ignored entirely.
///
/// The result goes only into `quota::ip_key(..)`; it is never logged, stored,
/// returned, or put in a field, span, `Uploader` or `RefusalLog` subject. It is
/// also the `StreamRecord.peer_ip` of the in-memory client-telemetry registry
/// (D-LTL4), which is never logged or written.
pub(crate) fn rate_limit_ip(peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    if !peer.to_canonical().is_loopback() {
        return peer;
    }
    headers
        .get_all(FORWARDED_FOR)
        .iter()
        .next_back()
        .and_then(|value| value.to_str().ok())
        .and_then(|list| list.rsplit(',').next())
        .and_then(|entry| entry.trim().parse::<IpAddr>().ok())
        .map_or(peer, |ip| ip.to_canonical())
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use axum::http::{HeaderMap, HeaderValue};

    use super::rate_limit_ip;

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("a literal address")
    }

    /// One `X-Forwarded-For` header per value, in order.
    fn xff(values: &[&[u8]]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            let value = HeaderValue::from_bytes(value).expect("a header value");
            headers.append("x-forwarded-for", value);
        }
        headers
    }

    #[test]
    fn a_loopback_peer_counts_the_rightmost_forwarded_address() {
        let loopback = ip("127.0.0.1");
        assert_eq!(
            rate_limit_ip(loopback, &xff(&[b"198.51.100.7"])),
            ip("198.51.100.7")
        );
        assert_eq!(
            rate_limit_ip(ip("::1"), &xff(&[b"2001:db8::7"])),
            ip("2001:db8::7")
        );
        assert_eq!(
            rate_limit_ip(ip("::ffff:127.0.0.1"), &xff(&[b"198.51.100.7"])),
            ip("198.51.100.7")
        );
        assert_eq!(
            rate_limit_ip(loopback, &xff(&[b"203.0.113.9, 198.51.100.7"])),
            ip("198.51.100.7")
        );
        assert_eq!(
            rate_limit_ip(loopback, &xff(&[b"203.0.113.9", b"198.51.100.7"])),
            ip("198.51.100.7")
        );
        assert_eq!(
            rate_limit_ip(loopback, &xff(&[b"::ffff:198.51.100.7"])),
            ip("198.51.100.7")
        );
    }

    #[test]
    fn anything_unusable_counts_the_peer() {
        let peer = ip("127.0.0.1");
        let cases = [
            HeaderMap::new(),
            xff(&[b""]),
            xff(&[b"unknown"]),
            xff(&[b"198.51.100.7:443"]),
            xff(&[b"[2001:db8::7]"]),
            xff(&[&[0xff, 0xfe]]),
        ];
        for headers in &cases {
            assert_eq!(rate_limit_ip(peer, headers), peer);
        }
    }

    #[test]
    fn a_remote_peer_cannot_name_its_own_address() {
        assert_eq!(
            rate_limit_ip(ip("203.0.113.9"), &xff(&[b"198.51.100.7"])),
            ip("203.0.113.9")
        );
    }
}
