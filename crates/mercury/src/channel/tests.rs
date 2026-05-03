//! Channel tests — extracted from channel/mod.rs to keep that file under
//! the 500-line soft cap. Logically a single test module; `#[cfg(test)]`
//! is applied at the parent's `mod tests;` declaration.

use super::*;
use std::net::SocketAddr;

#[test]
fn new_channel_starts_connecting() {
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let ch = Channel::new(addr);
        assert_eq!(ch.state, ChannelState::Connecting);
        assert_eq!(ch.next_tx_seq, 0);
        assert_eq!(ch.expected_rx_seq, 0);
        assert_eq!(ch.remote_addr, addr);
        assert!(ch.tx_window.is_empty());
        assert!(ch.rx_window.is_empty());
    }

    #[test]
    fn fresh_channel_not_timed_out() {
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let ch = Channel::new(addr);
        assert!(!ch.is_timed_out());
    }

    /// Helper: create a minimal valid Packet for channel tests.
    fn test_packet() -> Packet {
        use bytes::Bytes;
        use crate::packet::PacketFlags;

        Packet::new(
            PacketFlags::default(),
            0, // sequence is overwritten by send_packet
            Bytes::from_static(&[0xDE, 0xAD]),
        )
    }

    fn test_addr() -> SocketAddr {
        "127.0.0.1:9000".parse().unwrap()
    }

    #[test]
    fn mark_reliable_stores_packet() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();

        assert_eq!(ch.tx_window.len(), 1);
        // send_packet stamps seq=0 (the first sequence number).
        assert_eq!(ch.tx_window[0].packet.sequence, 0);
        assert_eq!(ch.tx_window[0].retransmit_count, 0);
    }

    #[test]
    fn on_ack_removes_packet() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        assert_eq!(ch.tx_window.len(), 1);

        // Cumulative ACK for seq=0 should drain the window.
        ch.process_acks(0).unwrap();
        assert!(ch.tx_window.is_empty());
    }

    #[test]
    fn check_retransmits_returns_expired() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();

        // Backdate last_sent by 800ms — well past the 700ms ACK_TIMEOUT_MS.
        ch.tx_window[0].last_sent =
            std::time::Instant::now() - std::time::Duration::from_millis(800);

        let retransmits = ch.check_timeouts();
        assert_eq!(retransmits.len(), 1);
        assert_eq!(retransmits[0].sequence, 0);
        // check_timeouts bumps the retransmit counter.
        assert_eq!(ch.tx_window[0].retransmit_count, 1);
    }

    #[test]
    fn check_retransmits_empty_when_fresh() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();

        // Immediately after send, the packet is well within the 700ms timeout.
        let retransmits = ch.check_timeouts();
        assert!(retransmits.is_empty());
        assert_eq!(ch.tx_window[0].retransmit_count, 0);
    }

    // ── Split last_activity into last_sent / last_received ─────────────

    #[test]
    fn send_packet_updates_only_last_sent() {
        let mut ch = Channel::new(test_addr());
        // Backdate both clocks to a known past time so we can detect which
        // one moves forward.
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.send_packet(test_packet()).unwrap();

        assert!(ch.last_sent > baseline, "send_packet must reset last_sent");
        assert_eq!(ch.last_received, baseline, "send_packet must NOT touch last_received");
    }

    #[test]
    fn receive_packet_updates_only_last_received() {
        use bytes::Bytes;
        use crate::packet::PacketFlags;

        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        // Inbound packet at expected_rx_seq=0.
        let pkt = Packet::new(PacketFlags::default(), 0, Bytes::from_static(&[0xAB]));
        ch.receive_packet(pkt).unwrap();

        assert!(ch.last_received > baseline, "receive_packet must reset last_received");
        assert_eq!(ch.last_sent, baseline, "receive_packet must NOT touch last_sent");
    }

    #[test]
    fn process_acks_updates_only_last_received() {
        let mut ch = Channel::new(test_addr());
        // Need a packet in flight for the ACK to drain.
        ch.send_packet(test_packet()).unwrap();

        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.process_acks(0).unwrap();

        // ACK is peer-originated data — counts as receive, not send.
        assert!(ch.last_received > baseline, "process_acks must reset last_received");
        assert_eq!(ch.last_sent, baseline, "process_acks must NOT touch last_sent");
    }

    #[test]
    fn keepalive_due_when_we_havent_sent_in_a_while() {
        let mut ch = Channel::new(test_addr());
        // Force last_sent into the past beyond KEEPALIVE_INTERVAL_MS, but
        // keep last_received fresh (peer is talking to us).
        ch.last_sent = std::time::Instant::now()
            - std::time::Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);
        ch.last_received = std::time::Instant::now();

        // Inbound peer traffic must NOT suppress our send-side keepalive
        // — NAT entries time out per direction.
        assert!(ch.keepalive_due(),
            "keepalive must be due based on OUR send-side silence, regardless of peer activity");
    }

    #[test]
    fn keepalive_not_due_right_after_sending() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        // Just sent; no keepalive needed for at least KEEPALIVE_INTERVAL_MS.
        assert!(!ch.keepalive_due());
    }

    #[test]
    fn is_timed_out_fires_on_silent_peer_even_if_we_keep_sending() {
        let mut ch = Channel::new(test_addr());
        // Simulate "we keep blasting world updates at a dead client":
        // last_sent is fresh, but last_received is stale past the
        // configured INACTIVITY_TIMEOUT_MS.
        ch.last_sent = std::time::Instant::now();
        ch.last_received = std::time::Instant::now()
            - std::time::Duration::from_millis(consts::INACTIVITY_TIMEOUT_MS + 100);

        // Conflated `last_activity` would have been refreshed by our own
        // sends — so dead clients would never be reaped. Splitting the
        // clocks closes that hole.
        assert!(ch.is_timed_out(),
            "is_timed_out must trigger on peer silence regardless of our outgoing traffic");
    }

    #[test]
    fn is_timed_out_does_not_fire_when_peer_is_chatty() {
        let mut ch = Channel::new(test_addr());
        // Both sides active recently — nothing to disconnect.
        ch.last_sent = std::time::Instant::now();
        ch.last_received = std::time::Instant::now();
        assert!(!ch.is_timed_out());
    }

    #[test]
    fn touch_sent_only_moves_send_clock() {
        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.touch_sent();

        assert!(ch.last_sent > baseline, "touch_sent must move last_sent");
        assert_eq!(ch.last_received, baseline, "touch_sent must NOT move last_received");
    }

    #[test]
    fn touch_received_only_moves_receive_clock() {
        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.touch_received();

        assert!(ch.last_received > baseline, "touch_received must move last_received");
        assert_eq!(ch.last_sent, baseline, "touch_received must NOT move last_sent");
    }

    #[test]
    fn check_timeouts_bumps_last_sent_when_retransmitting() {
        // A channel that's actively retransmitting a lost reliable packet
        // is putting bytes on the wire — the keepalive helper must observe
        // that activity and not emit redundant pings on top of the retries.
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        // Backdate both: the entry's last_sent so check_timeouts sees it as
        // expired, AND Channel.last_sent so we can detect whether it moves.
        let backdate = std::time::Instant::now() - std::time::Duration::from_millis(800);
        ch.tx_window[0].last_sent = backdate;
        ch.last_sent = backdate;

        let retransmits = ch.check_timeouts();
        assert_eq!(retransmits.len(), 1, "expired packet should be retransmitted");
        assert!(ch.last_sent > backdate,
            "check_timeouts emitting retransmits must bump Channel.last_sent");
    }

    #[test]
    fn check_timeouts_does_not_bump_last_sent_when_no_retransmits() {
        // A no-op pass (nothing expired) shouldn't perturb the keepalive
        // timer — otherwise the periodic tick would silently keep the
        // channel "active" forever.
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;

        let retransmits = ch.check_timeouts();
        assert!(retransmits.is_empty());
        assert_eq!(ch.last_sent, baseline,
            "no-op check_timeouts must leave last_sent untouched");
    }

    // ── Per-channel fragment reassembly ──────────────────────────────

    /// Build then parse a fragmented Mercury packet — same helper shape
    /// `unpacker::tests` uses, duplicated here because the inner module
    /// is private.
    fn build_then_parse_fragment(
        seq: u32,
        frag_begin: u32,
        frag_end: u32,
        body: &[u8],
    ) -> crate::packet::ParsedPacket {
        use crate::packet::{build_outgoing_fragmented, parse_incoming};
        let raw = build_outgoing_fragmented(0, body, seq, frag_begin, frag_end, &[]);
        parse_incoming(&raw).unwrap()
    }

    #[test]
    fn reassemble_parsed_passes_through_non_fragmented() {
        use crate::packet::{build_outgoing, parse_incoming, FLAG_HAS_SEQUENCE};
        let mut ch = Channel::new(test_addr());
        let raw = build_outgoing(FLAG_HAS_SEQUENCE, b"hello", Some(7), &[], None);
        let parsed = parse_incoming(&raw).unwrap();

        let body = ch.reassemble_parsed(&parsed).unwrap()
            .expect("non-fragmented should pass through");
        assert_eq!(body.as_ref(), b"hello");
    }

    #[test]
    fn reassemble_parsed_completes_3_fragment_bundle() {
        let mut ch = Channel::new(test_addr());
        let f0 = build_then_parse_fragment(10, 10, 12, b"AAA");
        let f1 = build_then_parse_fragment(11, 10, 12, b"BBB");
        let f2 = build_then_parse_fragment(12, 10, 12, b"CCC");

        assert!(ch.reassemble_parsed(&f0).unwrap().is_none());
        assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
        let body = ch.reassemble_parsed(&f2).unwrap().expect("third fragment completes");
        assert_eq!(body.as_ref(), b"AAABBBCCC");
    }

    #[test]
    fn reassemble_parsed_bumps_last_received() {
        // Receive-side observation must move last_received so the
        // peer-silence detector sees fragment activity as keepalive-
        // equivalent. Without this, a peer streaming a large bundle of
        // fragments would still look idle until the bundle assembled.
        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_received = baseline;
        ch.last_sent = baseline;

        let f0 = build_then_parse_fragment(20, 20, 21, b"part-one");
        ch.reassemble_parsed(&f0).unwrap();

        assert!(ch.last_received > baseline, "fragment receive must move last_received");
        assert_eq!(ch.last_sent, baseline, "fragment receive must NOT move last_sent");
    }

    #[test]
    fn reassemble_parsed_isolates_per_channel_state() {
        // Two channels with overlapping fragment seq ranges must NOT
        // share reassembly buffers — that's the whole point of putting
        // the assembler on the channel rather than the Nub.
        let mut a = Channel::new("127.0.0.1:8001".parse().unwrap());
        let mut b = Channel::new("127.0.0.1:8002".parse().unwrap());

        let a0 = build_then_parse_fragment(50, 50, 51, b"a-part-1");
        let a1 = build_then_parse_fragment(51, 50, 51, b"a-part-2");
        // b uses the SAME seq range (50..=51) — an unscoped assembler
        // would conflate b's fragments with a's, flush a partial bundle
        // early, or error on conflicting total_frags.
        let b0 = build_then_parse_fragment(50, 50, 51, b"BBB-1");

        assert!(a.reassemble_parsed(&a0).unwrap().is_none());
        // b's fragment must not affect a's pending state.
        assert!(b.reassemble_parsed(&b0).unwrap().is_none());
        let a_body = a.reassemble_parsed(&a1).unwrap().expect("a's bundle completes");
        assert_eq!(a_body.as_ref(), b"a-part-1a-part-2",
            "channel a must reassemble its own fragments without b's interference");
    }

    #[test]
    fn cleanup_stale_fragments_drops_partial_bundles() {
        let mut ch = Channel::new(test_addr());
        let f0 = build_then_parse_fragment(40, 40, 42, b"only-one");
        ch.reassemble_parsed(&f0).unwrap();

        ch.cleanup_stale_fragments(std::time::Duration::ZERO);

        // Subsequent re-receipt of f0 must start fresh — if the partial
        // bundle wasn't reaped, the assembler would silently treat the
        // re-arrival as a duplicate-fragment dedup and never complete.
        let f1 = build_then_parse_fragment(41, 40, 42, b"two");
        let f2 = build_then_parse_fragment(42, 40, 42, b"three");
        assert!(ch.reassemble_parsed(&f0).unwrap().is_none());
        assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
        let body = ch.reassemble_parsed(&f2).unwrap().expect("post-cleanup bundle completes");
        assert_eq!(body.as_ref(), b"only-onetwothree");
    }

    #[test]
    fn sliding_window_rejects_overflow() {
        let mut ch = Channel::new(test_addr());

        // Fill the TX window to capacity (TX_WINDOW_SIZE = 45).
        for _ in 0..consts::TX_WINDOW_SIZE {
            ch.send_packet(test_packet()).unwrap();
        }
        assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);

        // The 46th packet must be rejected.
        let result = ch.send_packet(test_packet());
        assert!(result.is_err());

        // Window size unchanged — the rejected packet was not inserted.
        assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);
    }
