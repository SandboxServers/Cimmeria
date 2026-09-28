//! One ACK retires the one packet it names (`channel::ack`), and the
//! transmit-hole watchdog built on it.
//!
//! The regression these guard: the ACK path used to be a cumulative drain,
//! so an ACK for #1149 retired a lost #1148 that the SGW client was still
//! waiting for. The client acks packets it buffers behind a gap
//! (`queueAckForPacket`, `ghidra://SGW.exe@0x0158cba0`; the Lomiada capture
//! shows it acking #1149..#1358 while #1148 never arrived), so the lost
//! packet was never resent and the client's reliable stream stayed wedged.

use super::*;
use crate::packet::Packet;
use crate::test_harness::TestClock;
use std::sync::Arc;
use std::time::Duration;

fn clocked() -> (Channel, Arc<TestClock>) {
    let clock = Arc::new(TestClock::new());
    let ch = Channel::with_clock("127.0.0.1:9000".parse().unwrap(), clock.clone());
    (ch, clock)
}

/// Register seqs as if each had gone on the wire, with distinct bytes so a
/// retransmit can be matched to its packet.
fn register(ch: &mut Channel, seqs: impl IntoIterator<Item = u32>) {
    for seq in seqs {
        let mut pkt = Packet::new(Default::default(), seq, bytes::Bytes::new());
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from(format!("pkt-{seq}")))
            .unwrap();
    }
}

fn outstanding(ch: &Channel) -> Vec<u32> {
    ch.tx_window
        .iter()
        .chain(ch.unsent_packets.iter())
        .map(|e| e.packet.sequence)
        .collect()
}

const JUST_UNDER: Duration = Duration::from_millis(consts::TX_HOLE_WARN_MS - 100);
const JUST_OVER: Duration = Duration::from_millis(consts::TX_HOLE_WARN_MS + 100);

/// The core guard. A cumulative drain retires 1148 here and the test fails.
#[test]
fn ack_of_a_later_packet_leaves_the_lost_one_outstanding() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    register(&mut ch, 1148..=1151);

    // 1148 was lost; the client acks what it buffered behind it.
    assert_eq!(ch.process_ack_footer(&[1149]), 1);
    assert_eq!(ch.process_ack_footer(&[1151, 1150]), 2);

    assert_eq!(
        outstanding(&ch),
        vec![1148],
        "only the packet the peer never acked may stay outstanding"
    );
}

/// The lost packet must actually be resent: the retransmit scan still
/// sees it after every later packet was acked.
#[test]
fn lost_packet_is_retransmitted_after_later_packets_are_acked() {
    let (mut ch, clock) = clocked();
    register(&mut ch, 1148..=1152);
    ch.process_ack_footer(&[1149, 1150, 1151, 1152]);

    clock.advance(Duration::from_secs(5));
    let resent = ch.check_timeouts();
    assert_eq!(
        resent,
        vec![bytes::Bytes::from("pkt-1148")],
        "the retransmit scan must resend exactly the lost packet"
    );

    // Its ACK finally clears it.
    assert!(ch.process_ack(1148));
    assert!(outstanding(&ch).is_empty());
}

/// Footer order is irrelevant: newest-first, oldest-first or shuffled.
#[test]
fn footer_order_does_not_matter() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    register(&mut ch, 10..=14);
    assert_eq!(ch.process_ack_footer(&[14, 11, 13, 10, 12]), 5);
    assert!(outstanding(&ch).is_empty());
}

/// A duplicate ACK, or one naming nothing we sent, retires nothing.
#[test]
fn duplicate_and_unknown_acks_retire_nothing() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    register(&mut ch, 1..=3);
    assert!(ch.process_ack(2));
    assert!(!ch.process_ack(2), "second ACK of the same packet");
    assert!(!ch.process_ack(99), "ACK of a packet never sent");
    assert_eq!(outstanding(&ch), vec![1, 3]);
}

/// A later packet acked while it sits in the deferred queue is retired
/// there; an earlier lost one in the TX window is untouched.
#[test]
fn deferred_queue_ack_retires_only_that_entry() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    let window = consts::TX_WINDOW_SIZE as u32;
    register(&mut ch, 0..window + 4);
    assert_eq!(ch.unsent_packets.len(), 4);

    // Everything but seq 0 arrives and is acked, queue entries included.
    let acks: Vec<u32> = (1..window + 4).collect();
    assert_eq!(ch.process_ack_footer(&acks), acks.len());
    assert_eq!(outstanding(&ch), vec![0]);
}

/// The ACK that drains a window slot promotes the oldest queued entry,
/// whose `last_sent` is kept so it can be resent once its RTO passes.
#[test]
fn freed_slot_promotes_a_lost_queued_packet_into_the_retransmit_scan() {
    let (mut ch, clock) = clocked();
    let window = consts::TX_WINDOW_SIZE as u32;
    register(&mut ch, 0..=window);
    // `window` itself (the one queued packet) is lost; the window's
    // packets all arrive.
    let acks: Vec<u32> = (0..window).collect();
    ch.process_ack_footer(&acks);
    assert_eq!(ch.tx_window.len(), 1, "the queued packet was promoted");
    assert!(ch.unsent_packets.is_empty());

    clock.advance(Duration::from_secs(5));
    assert_eq!(
        ch.check_timeouts(),
        vec![bytes::Bytes::from(format!("pkt-{window}"))]
    );
}

#[test]
fn in_order_acks_open_no_hole() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    register(&mut ch, 1..=4);
    ch.process_ack_footer(&[1, 2]);
    ch.process_ack_footer(&[3]);
    assert_eq!(ch.tx_holes, 0);
    assert_eq!(ch.open_tx_hole(), None);
}

/// A footer listing its ACKs newest-first does not open a hole on its way
/// through, because the hole is judged once per footer.
#[test]
fn a_newest_first_footer_opens_no_hole() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    register(&mut ch, 1150..=1155);
    ch.process_ack_footer(&[1155, 1154, 1153, 1152, 1151, 1150]);
    assert_eq!(ch.tx_holes, 0);
}

#[test]
fn ack_past_a_missing_packet_opens_a_hole_at_it() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    register(&mut ch, 1148..=1150);
    ch.process_ack_footer(&[1149]);
    assert_eq!(ch.tx_holes, 1);
    assert_eq!(ch.open_tx_hole().map(|(seq, _)| seq), Some(1148));

    // More ACKs past the same gap are the same hole, not new ones.
    ch.process_ack_footer(&[1150]);
    assert_eq!(ch.tx_holes, 1);

    // The resend lands and is acked: closed.
    ch.process_ack_footer(&[1148]);
    assert_eq!(ch.open_tx_hole(), None);
}

/// Two lost packets: when the first is acked the hole moves to the second.
#[test]
fn hole_moves_to_the_next_missing_packet() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    register(&mut ch, 1..=5);
    ch.process_ack_footer(&[2, 4, 5]);
    assert_eq!(ch.open_tx_hole().map(|(seq, _)| seq), Some(1));
    ch.process_ack_footer(&[1]);
    assert_eq!(ch.open_tx_hole().map(|(seq, _)| seq), Some(3));
    assert_eq!(ch.tx_holes, 2);
}

#[test]
fn watchdog_is_quiet_before_the_threshold() {
    let (mut ch, clock) = clocked();
    register(&mut ch, 1..=2);
    ch.process_ack_footer(&[2]);
    clock.advance(JUST_UNDER);
    assert_eq!(ch.check_tx_hole(), None);
    assert_eq!(ch.tx_hole_stalls, 0);
}

#[test]
fn watchdog_warns_once_then_throttles_and_counts_the_hole_once() {
    let (mut ch, clock) = clocked();
    register(&mut ch, 1148..=1150);
    ch.process_ack_footer(&[1149, 1150]);

    clock.advance(JUST_OVER);
    let stall = ch.check_tx_hole().expect("hole open past the threshold");
    assert_eq!(stall.seq, 1148);
    assert_eq!(stall.highest_acked, 1150);
    assert_eq!(stall.outstanding, 1);
    assert!(stall.first_warning);
    assert_eq!(ch.tx_hole_stalls, 1);

    // Throttled until the rewarn interval passes.
    clock.advance(Duration::from_millis(1_000));
    assert_eq!(ch.check_tx_hole(), None);

    clock.advance(Duration::from_millis(consts::TX_HOLE_REWARN_MS));
    let again = ch.check_tx_hole().expect("rewarn after the interval");
    assert!(!again.first_warning);
    assert_eq!(ch.tx_hole_stalls, 1, "one hole counts once");
}

#[test]
fn watchdog_reports_how_often_the_missing_packet_was_resent() {
    let (mut ch, clock) = clocked();
    register(&mut ch, 1..=2);
    ch.process_ack_footer(&[2]);
    clock.advance(Duration::from_secs(5));
    assert_eq!(ch.check_timeouts().len(), 1);
    let stall = ch.check_tx_hole().expect("still open");
    assert_eq!(stall.retransmit_count, 1);
}

#[test]
fn watchdog_goes_quiet_once_the_hole_closes() {
    let (mut ch, clock) = clocked();
    register(&mut ch, 1..=2);
    ch.process_ack_footer(&[2]);
    clock.advance(JUST_OVER);
    assert!(ch.check_tx_hole().is_some());
    ch.process_ack_footer(&[1]);
    clock.advance(Duration::from_millis(consts::TX_HOLE_REWARN_MS + 100));
    assert_eq!(ch.check_tx_hole(), None);
}

/// Sequence wraparound: a hole just below the 28-bit wrap, acked past it.
#[test]
fn hole_tracking_survives_sequence_wrap() {
    let mut ch = Channel::new("127.0.0.1:9000".parse().unwrap());
    let top = crate::packet::SEQUENCE_MASK;
    register(&mut ch, [top - 1, top, 0, 1]);
    ch.process_ack_footer(&[top, 0, 1]);
    assert_eq!(outstanding(&ch), vec![top - 1]);
    assert_eq!(ch.open_tx_hole().map(|(seq, _)| seq), Some(top - 1));
}
