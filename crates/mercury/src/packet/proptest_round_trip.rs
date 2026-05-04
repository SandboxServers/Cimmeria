//! Property-based round-trip tests for the Mercury packet codec.
//!
//! The example-driven tests in `packet/replay_smoke.rs` and the inline
//! `packet::tests` module pin a hand-picked matrix of shapes. proptest
//! widens that coverage to the full valid input space: it generates
//! random valid `(flags, body, seq_id, acks, first_req_offset)` /
//! `(flags, body, seq_id, frag_begin, frag_end, acks)` tuples,
//! round-trips them through `build_outgoing` ↔ `parse_incoming`, and
//! shrinks any failure to a minimal counterexample. Catches edge cases
//! the hand-picked matrix didn't think to write — empty acks at every
//! flag combo, large seq_id values near the `NULL_SEQUENCE` boundary,
//! large `first_req_offset` values that span the full u16 range, etc.
//!
//! Note on the "varint round-trip" item from #123 Group E: the codebase
//! has no varint encoding (none of the wire fields are LEB128 / similar
//! variable-length). The closest analog is the packet round-trip
//! itself — variable-length footer presence depending on flag bits —
//! and that's what this module pins.

use super::{
    build_outgoing, build_outgoing_fragmented, parse_incoming, FLAG_FRAGMENTED, FLAG_HAS_ACKS,
    FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE, FLAG_INDEXED, FLAG_ON_CHANNEL, FLAG_PIGGYBACK,
    FLAG_RELIABLE, NULL_SEQUENCE,
};
use proptest::prelude::*;

/// Generate a flag byte carrying a random subset of *passthrough*
/// bits — those the parser doesn't use to drive footer stripping
/// (RELIABLE, ON_CHANNEL, INDEXED, PIGGYBACK). The caller OR's in
/// the footer-presence bits (HAS_SEQUENCE, HAS_ACKS, HAS_REQUESTS,
/// FRAGMENTED) based on which optional fields are `Some`, so the
/// flag bits and the field set always agree.
fn arb_passthrough_flags() -> impl Strategy<Value = u8> {
    let passthrough_bits = FLAG_RELIABLE | FLAG_ON_CHANNEL | FLAG_INDEXED | FLAG_PIGGYBACK;
    (0u8..=passthrough_bits).prop_map(move |b| b & passthrough_bits)
}

/// Body bytes — bounded so a regression doesn't fuzz forever on
/// 1-MiB payloads. Mercury's MAX_BODY is 1348 but the round-trip
/// shape doesn't change above ~256 bytes; cap there for runtime.
fn arb_body() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(any::<u8>(), 0..=256)
}

/// Ack list, sized 1..=20 because empty triggers the "no FLAG_HAS_ACKS"
/// branch which is generated separately. The full u32 range is
/// allowed — including values above NULL_SEQUENCE — so a parser
/// regression that special-cases out-of-range acks would surface.
fn arb_acks() -> impl Strategy<Value = Vec<u32>> {
    proptest::collection::vec(any::<u32>(), 1..=20)
}

proptest! {
    #![proptest_config(ProptestConfig {
        // 256 cases is plenty for the small input space and keeps the
        // workspace `cargo test` runtime under one second per property.
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// Round-trip property: every valid non-fragmented packet that
    /// `build_outgoing` produces is parsed by `parse_incoming` back
    /// to the same flags / body / seq_id / acks / first_req_offset.
    /// The flag bits and footer values are independently generated
    /// (with the parser-relevant flag bits derived from which
    /// optional fields are `Some`), so a regression that conflates
    /// two of the footer slots would surface as a counterexample
    /// proptest will shrink to a minimal failing case.
    #[test]
    fn build_outgoing_round_trips_through_parse_incoming(
        passthrough in arb_passthrough_flags(),
        body in arb_body(),
        seq_id in proptest::option::of(0u32..=NULL_SEQUENCE - 1),
        acks in proptest::option::of(arb_acks()),
        first_req_offset in proptest::option::of(any::<u16>()),
    ) {
        // Derive the footer-presence flag bits from the optional
        // field choices so the parser's footer-stripping matches
        // what build_outgoing actually wrote.
        let mut flags = passthrough;
        if seq_id.is_some() { flags |= FLAG_HAS_SEQUENCE; }
        if acks.is_some()   { flags |= FLAG_HAS_ACKS; }
        if first_req_offset.is_some() { flags |= FLAG_HAS_REQUESTS; }
        // Don't accidentally turn on FLAG_FRAGMENTED via passthrough
        // bits — fragments use a different builder, covered below.
        flags &= !FLAG_FRAGMENTED;

        let acks_slice: &[u32] = acks.as_deref().unwrap_or(&[]);
        let raw = build_outgoing(flags, &body, seq_id, acks_slice, first_req_offset);
        let parsed = parse_incoming(&raw)
            .expect("any (flags, footers) tuple build_outgoing accepts must round-trip");

        prop_assert_eq!(parsed.flags, flags, "flags byte round-trip");
        prop_assert_eq!(parsed.body.as_ref(), body.as_slice(), "body round-trip");
        prop_assert_eq!(parsed.seq_id, seq_id, "seq_id round-trip");
        prop_assert_eq!(
            parsed.acks,
            acks.unwrap_or_default(),
            "ack list round-trip in order"
        );
        prop_assert_eq!(
            parsed.first_req_offset,
            first_req_offset,
            "first_req_offset round-trip"
        );
        // Non-fragmented frames must never carry frag footers.
        prop_assert_eq!(parsed.frag_begin, None);
        prop_assert_eq!(parsed.frag_end, None);
    }

    /// Companion property for the fragmented builder. Mirrors the
    /// packet shape produced by `build_fragmented_bundle` (frag_begin
    /// / frag_end pair innermost, seq_id outside, optional acks
    /// outermost). The example test in `replay_smoke.rs` covers one
    /// shape; this extends to the full space of valid inputs.
    #[test]
    fn fragmented_packet_round_trips_through_parse_incoming(
        passthrough in arb_passthrough_flags(),
        body in arb_body(),
        seq_id in 0u32..=NULL_SEQUENCE - 1,
        // frag_begin <= frag_end is the only legal range. Sample
        // frag_begin then add a small offset; widening the range
        // doesn't change the codec's behaviour because the parser
        // only stores both values as opaque u32s.
        frag_begin in 0u32..=10_000,
        frag_extent in 0u32..=64,
    ) {
        let frag_end = frag_begin + frag_extent;
        // build_outgoing_fragmented sets FLAG_FRAGMENTED and
        // FLAG_HAS_SEQUENCE itself. Pass passthrough bits but mask
        // FLAG_HAS_ACKS — see comment in the bare-fragment shape note
        // in replay_smoke.rs (acks would need FLAG_HAS_ACKS in the
        // caller-supplied flags too, exercised separately by the
        // first-fragment-of-a-bundle test in packet::tests).
        let flags = passthrough & !FLAG_HAS_ACKS;

        let raw = build_outgoing_fragmented(flags, &body, seq_id, frag_begin, frag_end, &[]);
        let parsed = parse_incoming(&raw).expect("fragmented packet must parse");

        prop_assert!(parsed.flags & FLAG_FRAGMENTED != 0);
        prop_assert!(parsed.flags & FLAG_HAS_SEQUENCE != 0);
        prop_assert_eq!(parsed.body.as_ref(), body.as_slice(), "body round-trip");
        prop_assert_eq!(parsed.seq_id, Some(seq_id), "seq_id round-trip");
        prop_assert_eq!(parsed.frag_begin, Some(frag_begin), "frag_begin round-trip");
        prop_assert_eq!(parsed.frag_end, Some(frag_end), "frag_end round-trip");
        prop_assert!(parsed.acks.is_empty(), "no acks on bare fragment");
    }
}
