use super::wire::{ipv4_peer, ProofCounters};

#[test]
fn mercury_recv_proof_serializes_exact_encrypted_read_without_bytes() {
    let counters = ProofCounters::new();
    counters.installed();
    let peer = ipv4_peer(&[2, 0, 0x1f, 0x90, 127, 0, 0, 1]);
    let proof = counters
        .observe("epoch-1", 7, 1472, 0, b"hello", peer.as_deref())
        .expect("Mercury read");
    let value = serde_json::to_value(proof).unwrap();
    assert_eq!(value["bridge_epoch"], "epoch-1");
    assert_eq!(value["socket"], 7);
    assert_eq!(value["peer"], "127.0.0.1:8080");
    assert_eq!(value["wire_len"], 5);
    assert_eq!(value["wire_fingerprint"], "a430d84680aabd0b");
    assert_eq!(value["gap"], serde_json::Value::Null);
    let serialized = value.to_string();
    assert!(!serialized.contains("hello"));
    assert!(!serialized.contains("wire_bytes"));
    assert_eq!(counters.snapshot().received, 1);
    assert_eq!(counters.snapshot().rejected, 0);
    assert!(counters.snapshot().hook_installed);
}

#[test]
fn mercury_recv_unknown_peer_and_other_socket_are_counted_without_false_peer() {
    let counters = ProofCounters::new();
    assert_eq!(counters.snapshot().gap, Some("hook_unobserved"));
    counters.installed();
    let invalid_sockaddr = ipv4_peer(&[23, 0, 0x1f, 0x90, 127, 0, 0, 1]);
    let proof = counters
        .observe("epoch-1", 7, 1472, 0, b"hello", invalid_sockaddr.as_deref())
        .expect("read retained with unknown peer");
    let value = serde_json::to_value(proof).unwrap();
    assert_eq!(value["peer"], serde_json::Value::Null);
    assert_eq!(value["gap"], "peer_unavailable");
    assert!(counters
        .observe("epoch-1", 8, 0x8000, 0, b"hello", Some("127.0.0.1:8080"))
        .is_none());
    assert!(counters
        .observe("epoch-1", 9, 1472, 1, b"hello", Some("127.0.0.1:8080"))
        .is_none());
    let status = counters.snapshot();
    assert_eq!(status.received, 1);
    assert_eq!(status.rejected, 3);
    assert_eq!(status.gap, None);
}

#[cfg(feature = "lab-bridge")]
#[test]
fn mercury_recv_bridge_ring_overflow_counts_dropped_and_keeps_first_rows() {
    use crate::bridge::events::push_proof_to;
    use crossbeam_channel::bounded;

    let counters = ProofCounters::new();
    counters.installed();
    let (tx, rx) = bounded(2);
    for socket in 1..=3 {
        let proof = counters
            .observe("epoch-1", socket, 1472, 0, b"hello", Some("127.0.0.1:8080"))
            .unwrap();
        assert_eq!(push_proof_to(&tx, &counters, proof, 1234), socket != 3);
    }
    assert_eq!(rx.len(), 2);
    assert_eq!(rx.try_recv().unwrap().fields["socket"], 1);
    assert_eq!(rx.try_recv().unwrap().fields["socket"], 2);
    let status = counters.snapshot();
    assert_eq!(status.received, 3);
    assert_eq!(status.dropped, 1);
}
