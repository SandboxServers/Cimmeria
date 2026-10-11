//! [`Channel::health`] (TG-MER-07): the snapshot `session.end` logs.

use super::*;
use crate::test_harness::TestClock;
use std::sync::Arc;
use std::time::Duration;

/// Longer than the RTO ceiling (4 s), so one advance expires every entry.
const PAST_MAX_RTO: Duration = Duration::from_secs(5);

fn clocked() -> (Channel, Arc<TestClock>) {
    let clock = Arc::new(TestClock::new());
    let ch = Channel::with_clock("127.0.0.1:9000".parse().unwrap(), clock.clone());
    (ch, clock)
}

fn send(ch: &mut Channel, seq: u32) {
    let packet = Packet::new(Default::default(), seq, bytes::Bytes::new());
    ch.register_sent_packet(packet, bytes::Bytes::from(format!("pkt-{seq}")))
        .unwrap();
}

/// A fresh channel reports nothing outstanding and no RTT yet.
#[test]
fn fresh_channel_health_is_empty() {
    let (ch, _clock) = clocked();
    assert_eq!(ch.health(), ChannelHealth::default());
}

/// Two unacked packets, one RTO expiry: both count as outstanding, each
/// resend counts once, and the oldest ages from its first send, not from
/// the retransmit that refreshed `last_sent`.
#[test]
fn health_counts_outstanding_retransmits_and_ages() {
    let (mut ch, clock) = clocked();
    send(&mut ch, 1);
    clock.advance(Duration::from_millis(100));
    send(&mut ch, 2);
    clock.advance(PAST_MAX_RTO);
    assert_eq!(ch.check_timeouts().len(), 2);
    clock.advance(Duration::from_millis(50));

    let h = ch.health();
    assert_eq!(h.tx_outstanding, 2);
    assert_eq!(h.retransmits_total, 2);
    assert_eq!(h.oldest_unacked_age_ms, Some(5_150));
    assert_eq!(h.last_rx_age_ms, 5_150);
    assert_eq!(h.srtt_ms, None, "every sample was a retransmit (Karn)");

    // Acking the oldest moves the age to the next one; the lifetime
    // counter does not go down.
    assert!(ch.process_ack(1));
    let h = ch.health();
    assert_eq!(h.tx_outstanding, 1);
    assert_eq!(h.oldest_unacked_age_ms, Some(5_050));
    assert_eq!(h.retransmits_total, 2);
    assert_eq!(h.last_rx_age_ms, 0, "an ACK is peer traffic");

    assert!(ch.process_ack(2));
    let h = ch.health();
    assert_eq!(h.tx_outstanding, 0);
    assert_eq!(h.oldest_unacked_age_ms, None);
}

/// A clean (never resent) ACK gives an RTT sample, so `srtt_ms` appears.
#[test]
fn health_reports_srtt_after_a_clean_sample() {
    let (mut ch, clock) = clocked();
    send(&mut ch, 1);
    clock.advance(Duration::from_millis(40));
    assert!(ch.process_ack(1));
    assert_eq!(ch.health().srtt_ms, Some(40));
}
