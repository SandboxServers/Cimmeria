//! Receive-stall watchdog (`Channel::check_rx_stall`, NA38) and the
//! adopt-first start that keeps a non-zero client stream from stalling.

use super::*;
use crate::packet::{
    build_outgoing, parse_incoming, ParsedPacket, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE,
};
use crate::test_harness::TestClock;
use std::sync::Arc;
use std::time::Duration;

fn reliable(seq: u32) -> ParsedPacket {
    let flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE | FLAG_RELIABLE;
    parse_incoming(&build_outgoing(flags, b"x", Some(seq), &[], None)).unwrap()
}

fn clocked() -> (Channel, Arc<TestClock>) {
    let clock = Arc::new(TestClock::new());
    let ch = Channel::with_clock("127.0.0.1:9000".parse().unwrap(), clock.clone());
    (ch, clock)
}

const JUST_UNDER: Duration = Duration::from_millis(consts::RX_STALL_WARN_MS - 100);
const JUST_OVER: Duration = Duration::from_millis(consts::RX_STALL_WARN_MS + 100);

#[test]
fn a_stream_starting_at_1234_is_delivered_normally() {
    // A fresh channel must take whatever sequence the peer starts on.
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    let first = ch.receive_parsed(reliable(1234)).unwrap();
    assert_eq!(
        first.bundles.len(),
        1,
        "seq 1234 must be delivered, not parked"
    );
    assert_eq!(first.ack, Some(1234));
    ch.receive_parsed(reliable(1236)).unwrap();
    let filled = ch.receive_parsed(reliable(1235)).unwrap();
    assert_eq!(filled.bundles.len(), 2, "1235 then the buffered 1236");
    assert_eq!(ch.expected_rx_seq, 1237);
    assert!(ch.rx_window.is_empty());
}

#[test]
fn a_gap_left_unfilled_past_the_threshold_is_reported_once_then_throttled() {
    let (mut ch, clock) = clocked();
    ch.receive_parsed(reliable(0)).unwrap();
    ch.receive_parsed(reliable(2)).unwrap(); // seq 1 never comes

    clock.advance(JUST_UNDER);
    assert_eq!(
        ch.check_rx_stall(),
        None,
        "under the threshold is normal loss"
    );

    clock.advance(JUST_OVER - JUST_UNDER);
    let stall = ch.check_rx_stall().expect("the stuck gap must be reported");
    assert_eq!(stall.expected, 1);
    assert_eq!((stall.first_buffered, stall.last_buffered), (2, 2));
    assert_eq!(stall.buffered, 1);
    assert_eq!(stall.depth, 2, "the gap slot plus seq 2");
    assert!(stall.first_warning);
    assert!(stall.stalled_for >= Duration::from_millis(consts::RX_STALL_WARN_MS));
    assert_eq!(ch.rx_stalls, 1);

    assert_eq!(ch.check_rx_stall(), None, "the next tick is throttled");

    clock.advance(Duration::from_millis(consts::RX_STALL_REWARN_MS));
    let again = ch.check_rx_stall().expect("a still-open gap re-warns");
    assert!(!again.first_warning);
    assert_eq!(ch.rx_stalls, 1, "one gap counts once");

    // The gap is never skipped: seq 2 is still waiting on seq 1.
    assert_eq!(ch.expected_rx_seq, 1);
}

#[test]
fn a_filled_gap_is_not_reported() {
    let (mut ch, clock) = clocked();
    ch.receive_parsed(reliable(0)).unwrap();
    ch.receive_parsed(reliable(2)).unwrap();
    clock.advance(JUST_UNDER);
    ch.receive_parsed(reliable(1)).unwrap();
    clock.advance(Duration::from_secs(30));
    assert_eq!(ch.check_rx_stall(), None);
    assert_eq!(ch.rx_stalls, 0);
}

#[test]
fn a_new_gap_behind_a_filled_one_restarts_the_clock() {
    let (mut ch, clock) = clocked();
    for seq in [0, 2, 4] {
        ch.receive_parsed(reliable(seq)).unwrap();
    }
    clock.advance(JUST_UNDER);
    // Filling seq 1 releases 1 and 2; seq 3 is now the blocking gap.
    ch.receive_parsed(reliable(1)).unwrap();
    clock.advance(JUST_UNDER);
    assert_eq!(
        ch.check_rx_stall(),
        None,
        "seq 3's gap is younger than the threshold"
    );
    clock.advance(JUST_OVER - JUST_UNDER);
    assert_eq!(ch.check_rx_stall().map(|s| s.expected), Some(3));
}
