//! Per-entry retransmit caps (`channel::retransmit_cap`, #842).
//!
//! A capped entry is resent at most `cap` times and then dropped from the
//! window unacked; uncapped entries keep the resend-until-acked contract.

use super::*;
use crate::channel::AbandonedPacket;
use crate::packet::Packet;
use crate::test_harness::TestClock;
use std::sync::Arc;
use std::time::Duration;

/// Longer than the RTO ceiling (4 s), so every advance expires every entry.
const PAST_MAX_RTO: Duration = Duration::from_secs(5);

fn clocked() -> (Channel, Arc<TestClock>) {
    let clock = Arc::new(TestClock::new());
    let ch = Channel::with_clock("127.0.0.1:9000".parse().unwrap(), clock.clone());
    (ch, clock)
}

fn packet(seq: u32) -> Packet {
    Packet::new(Default::default(), seq, bytes::Bytes::new())
}

fn bytes_of(seq: u32) -> bytes::Bytes {
    bytes::Bytes::from(format!("pkt-{seq}"))
}

/// Run `rounds` RTO expiries and count how often each packet was resent.
fn resend_counts(ch: &mut Channel, clock: &TestClock, rounds: usize, seqs: &[u32]) -> Vec<usize> {
    let mut counts = vec![0; seqs.len()];
    for _ in 0..rounds {
        clock.advance(PAST_MAX_RTO);
        for raw in ch.check_timeouts() {
            if let Some(i) = seqs.iter().position(|s| bytes_of(*s) == raw) {
                counts[i] += 1;
            }
        }
    }
    counts
}

/// A capped packet the peer never acks is resent exactly `cap` times,
/// then dropped and reported once. An uncapped packet beside it keeps
/// being resent.
#[test]
fn capped_entry_stops_after_its_cap_and_uncapped_keeps_going() {
    let (mut ch, clock) = clocked();
    ch.register_sent_packet_capped(packet(1), bytes_of(1), 3)
        .unwrap();
    ch.register_sent_packet(packet(3), bytes_of(3)).unwrap();

    let counts = resend_counts(&mut ch, &clock, 10, &[1, 3]);

    assert_eq!(
        counts[0], 3,
        "the capped packet is resent exactly cap times"
    );
    assert_eq!(
        counts[1], 10,
        "the uncapped packet is resent on every expiry"
    );
    assert_eq!(
        ch.take_abandoned(),
        vec![AbandonedPacket {
            seq: 1,
            retransmit_count: 3
        }]
    );
    assert!(ch.take_abandoned().is_empty(), "reported once");
    assert!(ch.tx_window.iter().all(|e| e.packet.sequence != 1));
}

/// An ACK before the cap retires the entry normally: nothing abandoned.
#[test]
fn acked_capped_entry_is_never_abandoned() {
    let (mut ch, clock) = clocked();
    ch.register_sent_packet_capped(packet(1), bytes_of(1), 2)
        .unwrap();
    clock.advance(PAST_MAX_RTO);
    assert_eq!(ch.check_timeouts(), vec![bytes_of(1)]);
    assert!(ch.process_ack(1));

    resend_counts(&mut ch, &clock, 5, &[1]);
    assert!(ch.take_abandoned().is_empty());
}

/// Dropping the packet a transmit hole sits on moves the hole off it, so
/// an abandoned packet cannot keep `tx_hole_stall` warning for the rest
/// of the session.
#[test]
fn abandoning_the_hole_packet_closes_the_hole() {
    let (mut ch, clock) = clocked();
    ch.register_sent_packet_capped(packet(1), bytes_of(1), 1)
        .unwrap();
    ch.register_sent_packet(packet(3), bytes_of(3)).unwrap();
    ch.process_ack_footer(&[3]);
    assert_eq!(ch.open_tx_hole().map(|(s, _)| s), Some(1));

    resend_counts(&mut ch, &clock, 2, &[1]);

    assert_eq!(ch.take_abandoned().len(), 1);
    assert_eq!(ch.open_tx_hole(), None);
}
