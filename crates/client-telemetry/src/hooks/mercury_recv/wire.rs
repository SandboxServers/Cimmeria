//! Raw `recvfrom` observations, before Mercury's packet filter or cipher.
//! The server uses the same fingerprint for its cached encrypted payload.

use serde_json::json;

use super::Fields;

/// `FUN_0158a200` passes this buffer size to `recvfrom` before storing the
/// returned length in `Mercury::Packet+0x24` (QA SGW.exe, Ghidra).
const MERCURY_RECV_CAPACITY: i32 = 0x5c0;
const WSAEWOULDBLOCK: i32 = 10035;

/// Diagnostic FNV-1a key for exact UDP bytes; no packet contents are logged.
pub(crate) fn fingerprint(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// IPv4 `sockaddr_in` as returned by Winsock. Other families remain unnamed.
pub(crate) fn ipv4_peer(addr: &[u8]) -> Option<String> {
    if addr.len() < 8 || u16::from_ne_bytes([addr[0], addr[1]]) != 2 {
        return None;
    }
    let port = u16::from_be_bytes([addr[2], addr[3]]);
    Some(format!(
        "{}.{}.{}.{}:{port}",
        addr[4], addr[5], addr[6], addr[7]
    ))
}

/// Event for one Winsock receive. `WSAEWOULDBLOCK` is the expected result
/// of a nonblocking poll and produces no event.
pub(crate) fn recv_event(
    socket: usize,
    requested_len: i32,
    wire: Option<&[u8]>,
    error: Option<i32>,
    peer: Option<&str>,
) -> Option<(&'static str, Fields)> {
    if requested_len != MERCURY_RECV_CAPACITY || error == Some(WSAEWOULDBLOCK) {
        return None;
    }
    let mut fields: Fields = vec![
        ("socket", json!(socket)),
        ("requested_len", json!(requested_len)),
        ("peer", json!(peer)),
    ];
    if let Some(bytes) = wire {
        let len = bytes.len();
        fields.extend([
            ("outcome", json!("received")),
            ("wire_len", json!(len)),
            ("wire_fingerprint", json!(fingerprint(bytes))),
            ("at_buffer_capacity", json!(len == requested_len as usize)),
        ]);
        Some(("info", fields))
    } else {
        fields.push((
            "outcome",
            json!(if error == Some(10040) {
                "message_too_large"
            } else {
                "socket_error"
            }),
        ));
        fields.push(("wsa_error", json!(error)));
        Some(("warn", fields))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(f: &Fields, key: &str) -> Option<serde_json::Value> {
        f.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn fingerprint_matches_server_vector() {
        assert_eq!(fingerprint(b"hello"), "a430d84680aabd0b");
    }

    #[test]
    fn full_buffer_and_message_too_large_are_distinct_outcomes() {
        let bytes = vec![0x42; 1472];
        let (level, fields) = recv_event(1, 1472, Some(&bytes), None, Some("127.0.0.1:9000"))
            .expect("received datagram");
        assert_eq!(level, "info");
        assert_eq!(get(&fields, "wire_len"), Some(json!(1472)));
        assert_eq!(get(&fields, "at_buffer_capacity"), Some(json!(true)));
        let (level, fields) = recv_event(1, 1472, None, Some(10040), None).unwrap();
        assert_eq!(level, "warn");
        assert_eq!(get(&fields, "outcome"), Some(json!("message_too_large")));
        assert_eq!(get(&fields, "wsa_error"), Some(json!(10040)));
        assert!(recv_event(1, 1472, None, Some(10035), None).is_none());
        assert!(recv_event(1, 0x8000, Some(&bytes), None, None).is_none());
    }

    #[test]
    fn ipv4_source_is_decoded_in_network_byte_order() {
        assert_eq!(
            ipv4_peer(&[2, 0, 0x1f, 0x90, 127, 0, 0, 1]),
            Some("127.0.0.1:8080".into())
        );
        assert_eq!(ipv4_peer(&[23, 0, 0, 0, 0, 0, 0, 0]), None);
    }
}
