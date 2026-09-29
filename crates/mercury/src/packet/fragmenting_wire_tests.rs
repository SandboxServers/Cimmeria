//! Wire-format guards for the fragment encoder against the SGW client's
//! bundle iterator (header-in-one-packet rule, see [`super::fragmenting`]).
//!
//! Each test runs real `build_fragmented_bundle` output through
//! [`crate::client_model`], so reverting the header guard in
//! `plan_fragments` to a raw 1300-byte split fails them.

use crate::client_model::{
    unpack_packet_bodies, unpack_plaintext_packets, unpack_reliable_stream, ClientAbort,
};
use crate::packet::{
    build_fragmented_bundle, fragment_count, parse_incoming, FLAG_RELIABLE, FRAGMENT_BODY_SIZE,
};

/// One entity-method message: `[0x85][len u16][payload of len bytes]`.
fn word_msg(payload_len: usize) -> Vec<u8> {
    let mut m = vec![0x85];
    m.extend_from_slice(&(payload_len as u16).to_le_bytes());
    m.extend(std::iter::repeat_n(0x11u8, payload_len));
    m
}

/// `(msg_id, payload)` of a [`word_msg`].
fn word_expected(payload_len: usize) -> (u8, Vec<u8>) {
    (0x85, vec![0x11u8; payload_len])
}

/// A body whose second message header lands at `first_header_at`, at least
/// `min_len` bytes long, plus the messages it encodes.
fn framed_body(first_header_at: usize, min_len: usize) -> (Vec<u8>, Vec<(u8, Vec<u8>)>) {
    let mut body = word_msg(first_header_at - 3);
    let mut expected = vec![word_expected(first_header_at - 3)];
    let mut n = 0usize;
    while body.len() < min_len {
        let len = 40 + (n * 13) % 50;
        body.extend(word_msg(len));
        expected.push(word_expected(len));
        n += 1;
    }
    (body, expected)
}

fn fragment_plain(body: &[u8]) -> Vec<Vec<u8>> {
    build_fragmented_bundle(FLAG_RELIABLE, body, 100, &[], |plaintext| {
        plaintext.to_vec()
    })
    .0
}

/// The failure shape of #838: the raw 1300-byte split leaves a header's first
/// byte or two at the end of a fragment. The client aborts there: everything
/// before the header is dispatched, nothing after.
#[test]
fn raw_split_at_a_header_aborts_the_client_after_the_prefix() {
    for at in [FRAGMENT_BODY_SIZE - 1, FRAGMENT_BODY_SIZE - 2] {
        let (body, expected) = framed_body(at, FRAGMENT_BODY_SIZE * 3);
        let raw: Vec<&[u8]> = body.chunks(FRAGMENT_BODY_SIZE).collect();
        let got = unpack_packet_bodies(&raw);
        assert_eq!(
            got.abort,
            Some(ClientAbort::HeaderStraddlesPackets {
                packet: 0,
                offset: at
            }),
            "header at {at}"
        );
        assert_eq!(got.messages.len(), 1, "only the message before the header");
        assert!(got.messages.len() < expected.len());
    }
}

/// The fix: the same bodies through `build_fragmented_bundle` reach the
/// client model whole.
#[test]
fn fragmented_bundle_never_leaves_a_header_across_packets() {
    for at in (FRAGMENT_BODY_SIZE - 6)..=FRAGMENT_BODY_SIZE {
        let (body, expected) = framed_body(at, FRAGMENT_BODY_SIZE * 3);
        let packets = fragment_plain(&body);
        assert!(packets.len() >= 3, "header at {at}");
        let got = unpack_plaintext_packets(&packets);
        assert_eq!(got.abort, None, "header at {at}");
        assert_eq!(got.messages, expected, "header at {at}");
    }
}

/// The straddle can land at any boundary, not only the first: sweeping the
/// message size walks the 14 boundaries of a 15-fragment bundle across every
/// header offset.
#[test]
fn every_boundary_of_a_many_fragment_bundle_is_header_safe() {
    for payload in 20..80usize {
        let mut body = Vec::new();
        let mut expected = Vec::new();
        while body.len() < FRAGMENT_BODY_SIZE * 15 {
            body.extend(word_msg(payload));
            expected.push(word_expected(payload));
        }
        let packets = fragment_plain(&body);
        assert!(packets.len() >= 15);
        let got = unpack_plaintext_packets(&packets);
        assert_eq!(got.abort, None, "payload {payload}");
        assert_eq!(got.messages, expected, "payload {payload}");
    }
}

/// Mixed constant-length and word-length messages of pseudo-random size, in
/// the shape of 24 NPC cascades: every message arrives, in order.
#[test]
fn mixed_framing_stream_of_24_npc_shape_delivers_every_message() {
    let mut state = 0x9E37_79B9u32;
    let mut next = move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 16) as usize
    };
    let mut body = Vec::new();
    let mut expected: Vec<(u8, Vec<u8>)> = Vec::new();
    for _ in 0..24 {
        for _ in 0..15 {
            if next() % 5 == 0 {
                // entityInvisible: Constant(5).
                body.push(0x0B);
                body.extend([1, 2, 3, 4, 5]);
                expected.push((0x0B, vec![1, 2, 3, 4, 5]));
            } else {
                let len = 4 + next() % 60;
                body.extend(word_msg(len));
                expected.push(word_expected(len));
            }
        }
    }
    let packets = fragment_plain(&body);
    let got = unpack_plaintext_packets(&packets);
    assert_eq!(got.abort, None);
    assert_eq!(got.messages, expected);
}

/// Footer contract the client's reassembly needs: one `lastFrag` for the
/// group, `firstFrag` = lowest seq, contiguous seqs, ACKs on fragment 0 only,
/// every body within the packet budget.
#[test]
fn fragment_footers_are_one_group_with_contiguous_seqs() {
    let (body, _) = framed_body(FRAGMENT_BODY_SIZE - 1, FRAGMENT_BODY_SIZE * 4);
    let (packets, consumed) =
        build_fragmented_bundle(FLAG_RELIABLE, &body, 5000, &[7, 8], |p| p.to_vec());
    assert_eq!(consumed as usize, packets.len());
    assert_eq!(packets.len(), fragment_count(&body));
    for (i, raw) in packets.iter().enumerate() {
        let p = parse_incoming(raw).unwrap();
        assert_eq!(p.frag_begin, Some(5000));
        assert_eq!(p.frag_end, Some(5000 + packets.len() as u32 - 1));
        assert_eq!(p.seq_id, Some(5000 + i as u32));
        assert!(p.body.len() <= FRAGMENT_BODY_SIZE);
        assert_eq!(p.acks.is_empty(), i != 0, "ACKs ride fragment 0 only");
    }
}

/// A fragment group followed by a single-packet bundle: the client model
/// sees two bundles, the second whole.
#[test]
fn fragment_group_then_single_packet_bundle() {
    let (big, big_expected) = framed_body(FRAGMENT_BODY_SIZE - 2, FRAGMENT_BODY_SIZE * 3);
    let mut packets = fragment_plain(&big);
    let next_seq = 100 + packets.len() as u32;
    let (small, _) =
        build_fragmented_bundle(FLAG_RELIABLE, &word_msg(30), next_seq, &[], |p| p.to_vec());
    packets.extend(small);
    let bundles = unpack_reliable_stream(&packets);
    assert_eq!(bundles.len(), 2);
    assert_eq!(bundles[0].messages, big_expected);
    assert_eq!(bundles[1].messages, vec![word_expected(30)]);
}
