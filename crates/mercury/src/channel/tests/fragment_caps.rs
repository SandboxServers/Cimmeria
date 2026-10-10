//! Per-channel reassembly caps through the live receive gate
//! (`Channel::receive_parsed`), including unreliable fragmented packets
//! (flags `0x60`), which skip the reliable receive window.

use super::*;
use crate::packet::{
    build_outgoing_fragmented, parse_incoming, ParsedPacket, FLAG_FRAGMENTED, FLAG_HAS_SEQUENCE,
    FLAG_RELIABLE,
};
use crate::test_harness::TestClock;
use std::sync::Arc;
use std::time::Duration;

/// An unreliable fragment: flags `0x60` (`FLAG_FRAGMENTED |
/// FLAG_HAS_SEQUENCE`), no `FLAG_RELIABLE`.
fn unreliable_fragment(seq: u32, begin: u32, end: u32, body: &[u8]) -> ParsedPacket {
    let pkt = parse_incoming(&build_outgoing_fragmented(0, body, seq, begin, end, &[])).unwrap();
    assert_eq!(pkt.flags, FLAG_FRAGMENTED | FLAG_HAS_SEQUENCE);
    pkt
}

fn reliable_fragment(seq: u32, begin: u32, end: u32, body: &[u8]) -> ParsedPacket {
    parse_incoming(&build_outgoing_fragmented(
        FLAG_RELIABLE,
        body,
        seq,
        begin,
        end,
        &[],
    ))
    .unwrap()
}

fn clocked() -> (Channel, Arc<TestClock>) {
    let clock = Arc::new(TestClock::new());
    let ch = Channel::with_clock("127.0.0.1:9000".parse().unwrap(), clock.clone());
    (ch, clock)
}

#[test]
fn unreliable_fragment_flood_is_held_within_both_caps() {
    let (mut ch, _clock) = clocked();
    let body = vec![0x5A; 1_300];
    // 2,000 disjoint 64-fragment ranges, one fragment each, none completed.
    for i in 0..2_000u32 {
        let begin = i * 100;
        let d = ch
            .receive_parsed(unreliable_fragment(begin, begin, begin + 63, &body))
            .unwrap();
        assert_eq!(d.outcome, RxOutcome::Unordered);
        assert!(d.bundles.is_empty());
        assert!(ch.pending_fragment_bundles() <= consts::MAX_PENDING_FRAGMENTED_BUNDLES);
        assert!(ch.pending_fragment_bytes() <= consts::MAX_PENDING_FRAGMENT_BYTES);
    }
    assert_eq!(
        ch.pending_fragment_bundles(),
        consts::MAX_PENDING_FRAGMENTED_BUNDLES
    );
    assert_eq!(
        ch.fragment_cap_drops,
        2_000u64.saturating_sub(consts::MAX_PENDING_FRAGMENTED_BUNDLES as u64)
    );
}

#[test]
fn unreliable_fragmented_bundle_is_still_accepted_and_reassembled() {
    // 0x60 stays accepted: a well-formed unreliable bundle reassembles,
    // even right after a flood has filled the caps.
    let (mut ch, _clock) = clocked();
    for i in 0..100u32 {
        let begin = i * 100;
        ch.receive_parsed(unreliable_fragment(begin, begin, begin + 1, b"junk"))
            .unwrap();
    }
    let begin = 50_000;
    assert!(ch
        .receive_parsed(unreliable_fragment(begin + 1, begin, begin + 1, b"world"))
        .unwrap()
        .bundles
        .is_empty());
    let d = ch
        .receive_parsed(unreliable_fragment(begin, begin, begin + 1, b"hello "))
        .unwrap();
    assert_eq!(d.bundles.len(), 1);
    assert_eq!(d.bundles[0].as_ref(), b"hello world");
    assert_eq!(d.bundle_seqs, vec![Some(begin)]);
}

#[test]
fn reliable_bundle_completes_with_the_caps_full_of_unreliable_partials() {
    // A reliable bundle whose fragments the receive window has already
    // acked must not be refused because orphans fill the caps.
    let (mut ch, _clock) = clocked();
    for i in 0..(consts::MAX_PENDING_FRAGMENTED_BUNDLES as u32 * 4) {
        let begin = 1_000_000 + i * 100;
        ch.receive_parsed(unreliable_fragment(begin, begin, begin + 9, b"orphan"))
            .unwrap();
    }
    assert!(ch
        .receive_parsed(reliable_fragment(0, 0, 2, b"a"))
        .unwrap()
        .bundles
        .is_empty());
    assert!(ch
        .receive_parsed(reliable_fragment(1, 0, 2, b"b"))
        .unwrap()
        .bundles
        .is_empty());
    let d = ch.receive_parsed(reliable_fragment(2, 0, 2, b"c")).unwrap();
    assert_eq!(d.outcome, RxOutcome::InOrder);
    assert_eq!(d.bundles.len(), 1);
    assert_eq!(d.bundles[0].as_ref(), b"abc");
}

#[test]
fn cap_warning_is_rate_limited_per_channel() {
    let (mut ch, clock) = clocked();
    let cap = consts::MAX_PENDING_FRAGMENTED_BUNDLES as u32;
    let mut next = 0u32;
    let mut open = |ch: &mut Channel, n: u32| {
        for _ in 0..n {
            let begin = next * 100;
            next += 1;
            ch.receive_parsed(unreliable_fragment(begin, begin, begin + 1, b"x"))
                .unwrap();
        }
    };

    open(&mut ch, cap);
    assert!(ch.fragment_cap_warned_at.is_none(), "no cap hit yet");

    open(&mut ch, 1);
    let first = ch.fragment_cap_warned_at.expect("first hit warns");
    assert!(ch.fragment_cap_unreported.is_empty());

    // Further hits inside the window are counted, not logged.
    clock.advance(Duration::from_millis(
        consts::FRAGMENT_CAP_WARN_INTERVAL_MS - 1,
    ));
    open(&mut ch, 50);
    assert_eq!(ch.fragment_cap_warned_at, Some(first));
    assert_eq!(ch.fragment_cap_unreported.count_evictions, 50);

    // The next hit after the window warns and carries the backlog.
    clock.advance(Duration::from_millis(1));
    open(&mut ch, 1);
    assert!(ch.fragment_cap_warned_at > Some(first));
    assert!(ch.fragment_cap_unreported.is_empty());
    assert_eq!(ch.fragment_cap_drops, 52);
}
