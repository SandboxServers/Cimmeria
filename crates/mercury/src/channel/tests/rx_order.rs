//! `Channel::receive_parsed` — the in-order reliable receive gate (NA38).
//!
//! Each test pins one row of the SGW client's `queueAckForPacket`
//! behaviour (`ghidra://SGW.exe@0x0158cba0`); see `channel/rx_order.rs`.

use super::*;
use crate::channel::RxOutcome;
use crate::packet::{
    build_outgoing, build_outgoing_fragmented, parse_incoming, ParsedPacket, FLAG_HAS_SEQUENCE,
    FLAG_ON_CHANNEL, FLAG_RELIABLE, SEQUENCE_MASK,
};
use std::net::SocketAddr;

fn addr() -> SocketAddr {
    "127.0.0.1:9000".parse().unwrap()
}

fn reliable(seq: u32, body: &[u8]) -> ParsedPacket {
    let flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE | FLAG_RELIABLE;
    parse_incoming(&build_outgoing(flags, body, Some(seq), &[], None)).unwrap()
}

fn unreliable(seq: u32, body: &[u8]) -> ParsedPacket {
    let flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE;
    parse_incoming(&build_outgoing(flags, body, Some(seq), &[], None)).unwrap()
}

fn anchored_at(seq: u32) -> Channel {
    let mut ch = Channel::new(addr());
    ch.anchor_rx_seq(seq);
    ch
}

fn bodies(d: &crate::channel::RxDelivery) -> Vec<&[u8]> {
    d.bundles.iter().map(|b| b.as_ref()).collect()
}

#[test]
fn a_gap_holds_later_reliable_packets_until_the_retransmit_fills_it() {
    let mut ch = anchored_at(0);

    let d0 = ch.receive_parsed(reliable(0, b"create")).unwrap();
    assert_eq!(d0.outcome, RxOutcome::InOrder);
    assert_eq!(bodies(&d0), vec![b"create".as_slice()]);

    // seq 1 is lost; 2 and 3 arrive first.
    for (seq, body) in [(2, b"cascade".as_slice()), (3, b"update".as_slice())] {
        let d = ch.receive_parsed(reliable(seq, body)).unwrap();
        assert_eq!(
            d.outcome,
            RxOutcome::Buffered,
            "seq {seq} is behind the gap"
        );
        assert!(d.bundles.is_empty(), "nothing may be delivered past a gap");
        assert_eq!(d.ack, Some(seq), "a buffered packet is still acked");
    }

    let filled = ch.receive_parsed(reliable(1, b"appearance")).unwrap();
    assert_eq!(filled.outcome, RxOutcome::InOrder);
    assert_eq!(
        bodies(&filled),
        vec![
            b"appearance".as_slice(),
            b"cascade".as_slice(),
            b"update".as_slice()
        ],
        "the gap-filler releases everything behind it, in sequence order"
    );
    assert_eq!(ch.expected_rx_seq, 4);
    assert!(ch.rx_window.is_empty());
}

#[test]
fn duplicates_are_dropped_but_acked() {
    let mut ch = anchored_at(0);
    ch.receive_parsed(reliable(0, b"a")).unwrap();
    ch.receive_parsed(reliable(2, b"c")).unwrap();

    // Already delivered: below the window.
    let below = ch.receive_parsed(reliable(0, b"a")).unwrap();
    assert_eq!(below.outcome, RxOutcome::Duplicate);
    assert!(below.bundles.is_empty());
    assert_eq!(below.ack, Some(0), "the client re-acks what it already has");

    // Already buffered: same slot.
    let buffered = ch.receive_parsed(reliable(2, b"c")).unwrap();
    assert_eq!(buffered.outcome, RxOutcome::Duplicate);
    assert!(buffered.bundles.is_empty());

    let filled = ch.receive_parsed(reliable(1, b"b")).unwrap();
    assert_eq!(
        bodies(&filled),
        vec![b"b".as_slice(), b"c".as_slice()],
        "the buffered duplicate must not appear twice"
    );
}

#[test]
fn unreliable_packets_are_delivered_on_arrival_even_behind_a_gap() {
    let mut ch = anchored_at(0);
    ch.receive_parsed(reliable(1, b"held")).unwrap();

    let d = ch.receive_parsed(unreliable(7, b"position")).unwrap();
    assert_eq!(d.outcome, RxOutcome::Unordered);
    assert_eq!(bodies(&d), vec![b"position".as_slice()]);
    assert_eq!(d.ack, None, "unreliable packets owe no ack");
    assert_eq!(
        ch.expected_rx_seq, 0,
        "the unreliable counter is independent"
    );
}

#[test]
fn a_packet_beyond_the_window_is_dropped_unacked() {
    let mut ch = anchored_at(0);
    let far = consts::RX_WINDOW_SIZE as u32;
    let d = ch.receive_parsed(reliable(far, b"too far")).unwrap();
    assert_eq!(d.outcome, RxOutcome::OutOfWindow);
    assert!(d.bundles.is_empty());
    assert_eq!(
        d.ack, None,
        "unacked, so the sender keeps retransmitting it"
    );
    assert!(ch.rx_window.is_empty());

    let edge = ch.receive_parsed(reliable(far - 1, b"edge")).unwrap();
    assert_eq!(edge.outcome, RxOutcome::Buffered, "the last slot is inside");
}

#[test]
fn the_receive_window_is_the_clients_512() {
    // `Channel` ctor writes 0x200 to +0x2c; `queueAckForPacket` compares
    // against the copy at ChannelInternal+0x30.
    assert_eq!(consts::RX_WINDOW_SIZE, 0x200);
}

#[test]
fn an_unanchored_channel_adopts_the_first_reliable_sequence() {
    // The client's `inSeqAt` starts at SEQ_NULL and takes the first seq.
    let mut ch = Channel::new(addr());
    assert!(!ch.rx_anchored());
    let d = ch.receive_parsed(reliable(41, b"first")).unwrap();
    assert_eq!(d.outcome, RxOutcome::InOrder);
    assert_eq!(bodies(&d), vec![b"first".as_slice()]);
    assert!(ch.rx_anchored());
    assert_eq!(ch.expected_rx_seq, 42);
}

#[test]
fn an_anchored_channel_recovers_a_lost_first_packet() {
    let mut ch = anchored_at(0);
    let early = ch.receive_parsed(reliable(1, b"second")).unwrap();
    assert_eq!(
        early.outcome,
        RxOutcome::Buffered,
        "seq 1 must wait for the lost seq 0, not become the start"
    );
    let first = ch.receive_parsed(reliable(0, b"first")).unwrap();
    assert_eq!(
        bodies(&first),
        vec![b"first".as_slice(), b"second".as_slice()]
    );
}

#[test]
fn ordering_survives_the_28_bit_sequence_wrap() {
    let mut ch = anchored_at(SEQUENCE_MASK);
    let ahead = ch.receive_parsed(reliable(0, b"after-wrap")).unwrap();
    assert_eq!(ahead.outcome, RxOutcome::Buffered);
    let last = ch
        .receive_parsed(reliable(SEQUENCE_MASK, b"before-wrap"))
        .unwrap();
    assert_eq!(
        bodies(&last),
        vec![b"before-wrap".as_slice(), b"after-wrap".as_slice()]
    );
    assert_eq!(ch.expected_rx_seq, 1);
    let old = ch
        .receive_parsed(reliable(SEQUENCE_MASK, b"before-wrap"))
        .unwrap();
    assert_eq!(old.outcome, RxOutcome::Duplicate, "pre-wrap seq is behind");
}

#[test]
fn a_reliable_fragmented_bundle_arriving_reversed_is_delivered_once() {
    let mut ch = anchored_at(10);
    let parts: [&[u8]; 3] = [b"AAA", b"BBB", b"CCC"];
    let frag = |i: u32| {
        let flags = FLAG_ON_CHANNEL | FLAG_RELIABLE;
        parse_incoming(&build_outgoing_fragmented(
            flags,
            parts[i as usize],
            10 + i,
            10,
            12,
            &[],
        ))
        .unwrap()
    };
    assert!(ch.receive_parsed(frag(2)).unwrap().bundles.is_empty());
    assert!(ch.receive_parsed(frag(1)).unwrap().bundles.is_empty());
    let done = ch.receive_parsed(frag(0)).unwrap();
    assert_eq!(bodies(&done), vec![b"AAABBBCCC".as_slice()]);
}
