//! Raw `recvfrom` observations, before Mercury's packet filter or cipher.
//! The server uses the same fingerprint for its cached encrypted payload.
//!
//! The fingerprint is also the join key that lets a client fault row be
//! matched to the server's send of the same datagram. It reaches the fault
//! rows in two hops:
//!
//! 1. **`recvfrom` to the packet filter, per thread.** `Nub::processPendingEvents`
//!    (`0x01581ab0`) runs, on the network thread and in one loop iteration,
//!    the socket reader `FUN_0158a200` (`recvfrom`) and then `0x01581830`,
//!    which calls `processFilteredPacket` (directly, or from the channel's
//!    `PacketEncrypter::recv` after decryption) before the next read. So the
//!    fingerprint the `recvfrom` detour stashes on its thread is the one the
//!    filter detour takes next ([`stash_received`], [`take_received`]).
//! 2. **Filter to the message loop, per packet.** `Bundle::iterator::unpack`
//!    runs on the game thread, after the packet crossed the `tbb` queue, so a
//!    thread-local cannot reach it. The filter detour records the packet
//!    object and its sequence number ([`remember`]); `unpack` looks the same
//!    pair up ([`join_fields`]). The packet object travels the queue
//!    unchanged, so the pair is the same on both threads.

use std::cell::Cell;
use std::sync::Mutex;

use serde_json::json;

use super::{Fields, SEQ_MASK, SEQ_UNSET};

/// `FUN_0158a200` passes this buffer size to `recvfrom` before storing the
/// returned length in `Mercury::Packet+0x24` (QA SGW.exe, Ghidra).
const MERCURY_RECV_CAPACITY: i32 = 0x5c0;
const WSAEWOULDBLOCK: i32 = 10035;

/// FNV-1a hash of exact UDP bytes.
pub(crate) fn fingerprint_u64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}

/// The fingerprint as it is logged on both sides: 16 hex digits.
pub(crate) fn fingerprint_hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Diagnostic FNV-1a key for exact UDP bytes; no packet contents are logged.
#[cfg(test)]
fn fingerprint(bytes: &[u8]) -> String {
    fingerprint_hex(fingerprint_u64(bytes))
}

thread_local! {
    /// The fingerprint of the datagram the last Mercury `recvfrom` on this
    /// thread returned, until the packet filter takes it.
    static RECEIVED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Note what a `recvfrom` on this thread returned: the datagram's
/// fingerprint for a Mercury read, `None` otherwise (so a stale key is never
/// handed to the next packet). Returns the fingerprint.
pub(crate) fn stash_received(requested_len: i32, wire: Option<&[u8]>) -> Option<u64> {
    let fp = wire
        .filter(|_| requested_len == MERCURY_RECV_CAPACITY)
        .map(fingerprint_u64);
    let _ = RECEIVED.try_with(|c| c.set(fp));
    fp
}

/// Take the fingerprint stashed by this thread's last `recvfrom`. The packet
/// filter takes it on entry, so a piggybacked packet (the filter recursing
/// on a nested packet) never inherits its carrier's key.
pub(crate) fn take_received() -> Option<u64> {
    RECEIVED.try_with(Cell::take).unwrap_or(None)
}

/// How many packets the filter-to-unpack join remembers. The game thread
/// drains the queue every frame, so a packet is unpacked a few frames after
/// it is filtered; an evicted entry only means the row lacks the key.
const JOIN_SLOTS: usize = 256;

#[derive(Clone, Copy)]
struct JoinSlot {
    packet: u32,
    seq: u32,
    fingerprint: u64,
}

static JOIN: Mutex<[Option<JoinSlot>; JOIN_SLOTS]> = Mutex::new([None; JOIN_SLOTS]);

fn slot_of(packet: u32, seq: u32) -> usize {
    ((packet >> 4) ^ seq) as usize % JOIN_SLOTS
}

/// Record that the packet object at `packet`, sequence `seq`, arrived as the
/// datagram with fingerprint `fp` (network thread, packet filter).
pub(crate) fn remember(packet: u32, seq: u32, fp: u64) {
    let mut slots = JOIN.lock().unwrap_or_else(|e| e.into_inner());
    slots[slot_of(packet, seq)] = Some(JoinSlot {
        packet,
        seq,
        fingerprint: fp,
    });
}

/// The fingerprint recorded for `packet` with sequence `seq`, if it is still
/// remembered. A reused packet object with another sequence number misses.
pub(crate) fn recall(packet: u32, seq: u32) -> Option<u64> {
    let slots = JOIN.lock().unwrap_or_else(|e| e.into_inner());
    slots[slot_of(packet, seq)]
        .filter(|s| s.packet == packet && s.seq == seq)
        .map(|s| s.fingerprint)
}

/// The join keys for a row about the packet object at `packet` whose stored
/// sequence number (`Packet+0x44`) is `seq`: `seq` when the packet has one,
/// and `wire_fingerprint` when the filter recorded it.
pub(crate) fn join_fields(packet: u32, seq: Option<u32>) -> Fields {
    let Some(seq) = seq.filter(|&s| s != SEQ_UNSET && s & SEQ_MASK == s) else {
        return Vec::new();
    };
    let mut f: Fields = vec![("seq", json!(seq))];
    if let Some(fp) = recall(packet, seq) {
        f.push(("wire_fingerprint", json!(fingerprint_hex(fp))));
    }
    f
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
/// of a nonblocking poll and produces no event. `wire_fp` is the
/// fingerprint [`stash_received`] already computed, so the bytes are hashed
/// once; `None` hashes them here.
pub(crate) fn recv_event(
    socket: usize,
    requested_len: i32,
    wire: Option<&[u8]>,
    wire_fp: Option<u64>,
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
            (
                "wire_fingerprint",
                json!(fingerprint_hex(
                    wire_fp.unwrap_or_else(|| fingerprint_u64(bytes))
                )),
            ),
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
        let (level, fields) = recv_event(1, 1472, Some(&bytes), None, None, Some("127.0.0.1:9000"))
            .expect("received datagram");
        assert_eq!(level, "info");
        assert_eq!(get(&fields, "wire_len"), Some(json!(1472)));
        assert_eq!(get(&fields, "at_buffer_capacity"), Some(json!(true)));
        assert_eq!(
            get(&fields, "wire_fingerprint"),
            Some(json!(fingerprint(&bytes)))
        );
        let (level, fields) = recv_event(1, 1472, None, None, Some(10040), None).unwrap();
        assert_eq!(level, "warn");
        assert_eq!(get(&fields, "outcome"), Some(json!("message_too_large")));
        assert_eq!(get(&fields, "wsa_error"), Some(json!(10040)));
        assert!(recv_event(1, 1472, None, None, Some(10035), None).is_none());
        assert!(recv_event(1, 0x8000, Some(&bytes), None, None, None).is_none());
    }

    /// The filter takes the key the last Mercury `recvfrom` on its thread
    /// stashed, once; a non-Mercury read or a failed read clears it.
    #[test]
    fn the_received_fingerprint_is_taken_once_per_datagram() {
        let bytes = b"hello";
        assert_eq!(
            stash_received(MERCURY_RECV_CAPACITY, Some(bytes)),
            Some(0xa430_d846_80aa_bd0b)
        );
        assert_eq!(take_received(), Some(0xa430_d846_80aa_bd0b));
        assert_eq!(take_received(), None);
        stash_received(MERCURY_RECV_CAPACITY, Some(bytes));
        stash_received(MERCURY_RECV_CAPACITY, None);
        assert_eq!(take_received(), None);
        stash_received(MERCURY_RECV_CAPACITY, Some(bytes));
        stash_received(0x8000, Some(bytes));
        assert_eq!(take_received(), None);
    }

    /// The game thread finds the key by packet object and sequence number;
    /// the same object with another sequence number (a reused packet) misses.
    #[test]
    fn join_fields_carry_seq_and_the_remembered_fingerprint() {
        remember(0x0abc_d000, 4242, 0xa430_d846_80aa_bd0b);
        let f = join_fields(0x0abc_d000, Some(4242));
        assert_eq!(get(&f, "seq"), Some(json!(4242)));
        assert_eq!(get(&f, "wire_fingerprint"), Some(json!("a430d84680aabd0b")));
        let reused = join_fields(0x0abc_d000, Some(4243));
        assert_eq!(get(&reused, "seq"), Some(json!(4243)));
        assert_eq!(get(&reused, "wire_fingerprint"), None);
        assert!(join_fields(0x0abc_d000, Some(SEQ_UNSET)).is_empty());
        assert!(join_fields(0x0abc_d000, None).is_empty());
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
