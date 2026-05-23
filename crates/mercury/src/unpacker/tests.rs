use super::*;

#[test]
fn single_fragment_completes_immediately() {
    let mut asm = FragmentAssembler::new();
    let result = asm
        .add_fragment(100, 0, 1, Bytes::from_static(b"complete"))
        .unwrap();
    assert_eq!(result.unwrap().as_ref(), b"complete");
    assert_eq!(asm.pending_count(), 0);
}

#[test]
fn multi_fragment_assembly() {
    let mut asm = FragmentAssembler::new();

    // Fragment 0 of 3.
    let r = asm
        .add_fragment(50, 0, 3, Bytes::from_static(b"aaa"))
        .unwrap();
    assert!(r.is_none());

    // Fragment 2 of 3 (out of order).
    let r = asm
        .add_fragment(50, 2, 3, Bytes::from_static(b"ccc"))
        .unwrap();
    assert!(r.is_none());

    // Fragment 1 of 3 (completes the message).
    let r = asm
        .add_fragment(50, 1, 3, Bytes::from_static(b"bbb"))
        .unwrap();
    assert_eq!(r.unwrap().as_ref(), b"aaabbbccc");
    assert_eq!(asm.pending_count(), 0);
}

#[test]
fn invalid_frag_index() {
    let mut asm = FragmentAssembler::new();
    let err = asm
        .add_fragment(1, 5, 3, Bytes::from_static(b"bad"))
        .unwrap_err();
    assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
}

#[test]
fn zero_total_frags() {
    let mut asm = FragmentAssembler::new();
    let err = asm
        .add_fragment(1, 0, 0, Bytes::from_static(b"bad"))
        .unwrap_err();
    assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
}

// ── Receive-path integration via process_parsed ─────────────────

/// Helper: build a fragmented Mercury packet via the public encoder
/// then parse it back. Mirrors what the live receive loop does per
/// fragment when the assembler is wired into the channel.
fn build_then_parse_fragment(
    seq: u32,
    frag_begin: u32,
    frag_end: u32,
    body: &[u8],
) -> ParsedPacket {
    use crate::packet::{build_outgoing_fragmented, parse_incoming};
    let raw = build_outgoing_fragmented(0, body, seq, frag_begin, frag_end, &[]);
    parse_incoming(&raw).unwrap()
}

#[test]
fn process_parsed_passes_through_non_fragmented() {
    use crate::packet::{build_outgoing, parse_incoming, FLAG_HAS_SEQUENCE};
    let raw = build_outgoing(FLAG_HAS_SEQUENCE, b"hello", Some(7), &[], None);
    let parsed = parse_incoming(&raw).unwrap();

    let mut asm = FragmentAssembler::new();
    let body = asm
        .process_parsed(&parsed)
        .unwrap()
        .expect("non-fragmented should pass through");
    assert_eq!(body.as_ref(), b"hello");
    assert_eq!(
        asm.pending_count(),
        0,
        "no reassembly state for pass-through"
    );
}

#[test]
fn process_parsed_reassembles_in_order_3_fragment_bundle() {
    // A hand-built 3-fragment bundle round-trips through parse +
    // assembler integration.
    let mut asm = FragmentAssembler::new();
    let f0 = build_then_parse_fragment(10, 10, 12, b"AAA");
    let f1 = build_then_parse_fragment(11, 10, 12, b"BBB");
    let f2 = build_then_parse_fragment(12, 10, 12, b"CCC");

    assert!(asm.process_parsed(&f0).unwrap().is_none());
    assert!(asm.process_parsed(&f1).unwrap().is_none());
    let body = asm
        .process_parsed(&f2)
        .unwrap()
        .expect("third fragment completes");
    assert_eq!(body.as_ref(), b"AAABBBCCC");
    assert_eq!(asm.pending_count(), 0);
}

#[test]
fn process_parsed_reassembles_out_of_order() {
    // UDP: fragments may arrive in any order. The assembler keys by
    // first_seq and indexes by (seq - begin), so insertion order
    // doesn't matter as long as all fragments share the same range.
    let mut asm = FragmentAssembler::new();
    let f0 = build_then_parse_fragment(20, 20, 22, b"xxx");
    let f1 = build_then_parse_fragment(21, 20, 22, b"yyy");
    let f2 = build_then_parse_fragment(22, 20, 22, b"zzz");

    assert!(asm.process_parsed(&f2).unwrap().is_none());
    assert!(asm.process_parsed(&f0).unwrap().is_none());
    let body = asm
        .process_parsed(&f1)
        .unwrap()
        .expect("last (index 1) completes");
    assert_eq!(body.as_ref(), b"xxxyyyzzz");
}

#[test]
fn process_parsed_handles_duplicate_fragments() {
    // UDP can deliver duplicates. A second copy of the same fragment
    // must not double-count toward `received_count` and must not
    // overwrite the already-stored payload.
    let mut asm = FragmentAssembler::new();
    let f0 = build_then_parse_fragment(30, 30, 31, b"hello ");
    let f1 = build_then_parse_fragment(31, 30, 31, b"world");

    assert!(asm.process_parsed(&f0).unwrap().is_none());
    // Duplicate of f0 — assembler should ignore.
    assert!(asm.process_parsed(&f0).unwrap().is_none());
    let body = asm
        .process_parsed(&f1)
        .unwrap()
        .expect("f1 still completes once after dup f0");
    assert_eq!(body.as_ref(), b"hello world");
}

#[test]
fn process_parsed_rejects_fragment_count_above_max() {
    // The cap exists to bound per-peer reassembly memory. Synthesize
    // a packet whose declared range would exceed MAX_FRAGMENTS and
    // verify we error instead of allocating a giant buffer.
    let oversized_end = MAX_FRAGMENTS as u32; // begin=0, end=MAX → MAX+1 fragments
    let pkt = build_then_parse_fragment(0, 0, oversized_end, b"x");

    let mut asm = FragmentAssembler::new();
    let err = asm.process_parsed(&pkt).unwrap_err();
    assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
    assert_eq!(
        asm.pending_count(),
        0,
        "rejected range must not register pending"
    );
}

#[test]
fn process_parsed_rejects_seq_outside_range() {
    // seq < frag_begin (wrapping_sub catches this via huge diff) and
    // seq > frag_end both surface as "outside range" errors. Without
    // this guard, `seq - begin` would index past the fragments vec
    // and the assembler would silently drop the bytes.
    let pkt = build_then_parse_fragment(99, 10, 12, b"way-out");

    let mut asm = FragmentAssembler::new();
    let err = asm.process_parsed(&pkt).unwrap_err();
    assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
}

#[test]
fn process_parsed_handles_u32_max_range_without_overflow() {
    // Pathological/malicious case: frag_begin=0, frag_end=u32::MAX
    // would compute `end - begin + 1 == u32::MAX + 1` and overflow
    // u32. Promoting to u64 lets us detect it via the MAX_FRAGMENTS
    // cap rather than panicking in debug or wrapping to 0 in release.
    // Synthesize the ParsedPacket directly since `build_then_parse_fragment`
    // won't help us — the encoder rejects oversized ranges upstream.
    let pkt = ParsedPacket {
        flags: crate::packet::FLAG_FRAGMENTED | crate::packet::FLAG_HAS_SEQUENCE,
        body: Bytes::from_static(b"x"),
        seq_id: Some(0),
        first_req_offset: None,
        frag_begin: Some(0),
        frag_end: Some(u32::MAX),
        acks: vec![],
    };

    let mut asm = FragmentAssembler::new();
    let err = asm.process_parsed(&pkt).unwrap_err();
    assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
    assert_eq!(
        asm.pending_count(),
        0,
        "rejected packet must not register pending state"
    );
}

#[test]
fn process_parsed_rejects_bogus_range_via_max_fragments_cap() {
    // A non-wrap garbage range like `frag_begin=10, frag_end=4`
    // implies a ~268M-fragment wrap under modular arithmetic
    // (`(4 - 10) & 0x0FFFFFFF + 1 ≈ 268_435_452`). The
    // `MAX_FRAGMENTS` cap rejects it. (Pre-fix this hit a separate
    // "inverted range" gate; now both gates collapse to the
    // modular cap, matching `add_fragment`.)
    let pkt = build_then_parse_fragment(5, 10, 4, b"bad");

    let mut asm = FragmentAssembler::new();
    let err = asm.process_parsed(&pkt).unwrap_err();
    assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
}

/// Regression guard: a wire-arriving fragmented bundle whose range
/// straddles the 28-bit sequence-space wrap MUST be accepted and
/// reassembled, not rejected with "inverted range". Pre-fix,
/// `process_parsed` had a `frag_end < frag_begin` gate that dropped
/// every wrapped bundle before the modular-overlap logic in
/// `add_fragment` could see it. Symmetric with the
/// `overlap_detection_handles_28_bit_sequence_wraparound` test
/// that pins the same behavior at the `add_fragment` layer.
#[test]
fn process_parsed_accepts_28_bit_wrapped_range() {
    // Bundle straddling the 28-bit wrap: begin=0x0FFFFFFE,
    // end=0x00000001, four fragments at seqs
    //   0x0FFFFFFE, 0x0FFFFFFF, 0x00000000, 0x00000001.
    let begin = 0x0FFF_FFFEu32;
    let end = 0x0000_0001u32;
    let seqs = [0x0FFF_FFFEu32, 0x0FFF_FFFF, 0x0000_0000, 0x0000_0001];
    let mut asm = FragmentAssembler::new();
    let mut completed: Option<Bytes> = None;
    for (i, &seq) in seqs.iter().enumerate() {
        let body = format!("f{i}");
        let pkt = build_then_parse_fragment(seq, begin, end, body.as_bytes());
        let r = asm.process_parsed(&pkt).unwrap();
        if i < seqs.len() - 1 {
            assert!(r.is_none(), "fragment {i} must not complete the bundle");
        } else {
            completed = Some(r.expect("last fragment must complete the wrapped bundle"));
        }
    }
    let body = completed.expect("wrapped bundle must complete");
    assert_eq!(body.as_ref(), b"f0f1f2f3");
    assert_eq!(asm.pending_count(), 0, "completed bundle must be removed");
}

/// Two fragments arriving for the same `first_seq` must agree on
/// `total_frags`. A peer (or attacker) sending a second fragment
/// with a different declared count is a protocol violation; the
/// assembler must reject rather than silently re-shape the
/// pending message.
#[test]
fn add_fragment_rejects_conflicting_total_fragments() {
    let mut asm = FragmentAssembler::new();
    // First fragment: declares 3 total.
    asm.add_fragment(50, 0, 3, Bytes::from_static(b"aaa"))
        .unwrap();
    // Second fragment for the SAME first_seq but declares 5 total.
    let err = asm
        .add_fragment(50, 1, 5, Bytes::from_static(b"bbb"))
        .unwrap_err();
    assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
    // Pending entry must remain (rejecting the new fragment must
    // not wipe the in-progress reassembly that a well-behaved peer
    // is still completing).
    assert_eq!(asm.pending_count(), 1);
}

/// Arrival-triggered eviction (`mercury-wire-format` spec §2.4.1 R13
/// and §2.10 S6): a new fragmented bundle whose sequence range overlaps
/// an in-progress reassembly's range — and is keyed on a different
/// `first_seq` — evicts the in-progress one. The SGW client emits the
/// matching log line at binary address `0x01b18868`. There is no
/// periodic stale-sweep; abandoned reassemblies persist in the
/// assembler until either an overlapping bundle arrives, or the channel
/// itself is torn down.
#[test]
fn arrival_of_overlapping_bundle_evicts_in_progress_reassembly() {
    let mut asm = FragmentAssembler::new();
    // First bundle: range [10..=12].
    asm.add_fragment(10, 0, 3, Bytes::from_static(b"abandoned"))
        .unwrap();
    assert_eq!(asm.pending_count(), 1);

    // Second bundle: range [12..=14]. Overlaps the first at seq 12.
    // Adding any fragment of the second bundle must evict the first.
    let r = asm
        .add_fragment(12, 0, 3, Bytes::from_static(b"fresh-0"))
        .unwrap();
    assert!(
        r.is_none(),
        "first fragment of new bundle waits for siblings"
    );
    assert_eq!(
        asm.pending_count(),
        1,
        "old bundle [10..=12] must be evicted; only the new bundle [12..=14] remains"
    );

    // Complete the new bundle to verify it isn't itself corrupted by
    // the eviction logic.
    asm.add_fragment(12, 1, 3, Bytes::from_static(b"fresh-1"))
        .unwrap();
    let body = asm
        .add_fragment(12, 2, 3, Bytes::from_static(b"fresh-2"))
        .unwrap()
        .expect("new bundle completes");
    assert_eq!(body.as_ref(), b"fresh-0fresh-1fresh-2");
}

/// Non-overlapping bundles coexist. A new bundle on a different,
/// non-overlapping sequence range MUST NOT touch the in-progress
/// one — eviction is strictly an "old abandoned bundle whose space
/// the new one is reclaiming" signal, not a "any new fragment
/// invalidates everything else" reset.
#[test]
fn arrival_of_non_overlapping_bundle_leaves_in_progress_alone() {
    let mut asm = FragmentAssembler::new();
    // Bundle A: range [10..=12].
    asm.add_fragment(10, 0, 3, Bytes::from_static(b"a"))
        .unwrap();
    // Bundle B: range [20..=22]. No overlap with A.
    asm.add_fragment(20, 0, 3, Bytes::from_static(b"b"))
        .unwrap();
    assert_eq!(asm.pending_count(), 2, "non-overlapping bundles coexist");
}

/// The inverse of the deleted "stale-sweep reaps partial set" test:
/// an in-progress reassembly that never sees its remaining fragments
/// MUST persist in the assembler indefinitely (until the channel
/// itself is destroyed, which drops the assembler). The SGW client
/// has no 30s sweep; the Rust assembler matches.
#[test]
fn orphan_partial_reassembly_persists_indefinitely() {
    let mut asm = FragmentAssembler::new();
    let f0 = build_then_parse_fragment(40, 40, 42, b"only-one");
    assert!(asm.process_parsed(&f0).unwrap().is_none());
    assert_eq!(asm.pending_count(), 1);

    // No periodic sweep API exists. Time passing alone must NOT
    // reap the entry — pin the lifecycle by sleeping a short
    // interval (would have been reaped at the old 30s threshold;
    // the new contract reaps nothing on time).
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert_eq!(
        asm.pending_count(),
        1,
        "orphan partial reassembly must persist until channel teardown"
    );
}

/// Eviction is asymmetric: a late straggler from an already-evicted
/// older bundle must NOT displace the newer bundle that took over.
/// Without this guarantee the eviction logic would oscillate every
/// time a stale fragment arrived. Pin the asymmetry directly.
#[test]
fn late_fragment_from_evicted_older_bundle_does_not_displace_newer() {
    let mut asm = FragmentAssembler::new();
    // Bundle A `[10..=12]` half-arrives.
    asm.add_fragment(10, 0, 3, Bytes::from_static(b"abandoned-0"))
        .unwrap();
    // Bundle B `[12..=14]` arrives → evicts A.
    asm.add_fragment(12, 0, 3, Bytes::from_static(b"fresh-0"))
        .unwrap();
    assert_eq!(asm.pending_count(), 1);

    // Late straggler from A arrives. A's range `[10..=12]` overlaps
    // B's range `[12..=14]` at seq 12 — but A is older, so the
    // straggler is dropped rather than evicting B.
    let r = asm
        .add_fragment(10, 1, 3, Bytes::from_static(b"late-straggler"))
        .unwrap();
    assert!(
        r.is_none(),
        "stale fragment returns Ok(None) and is dropped"
    );
    assert_eq!(
        asm.pending_count(),
        1,
        "newer bundle must still be the only pending entry; the stale fragment did NOT re-buffer A"
    );

    // B completes cleanly using only B's fragments.
    asm.add_fragment(12, 1, 3, Bytes::from_static(b"fresh-1"))
        .unwrap();
    let body = asm
        .add_fragment(12, 2, 3, Bytes::from_static(b"fresh-2"))
        .unwrap()
        .expect("bundle B completes");
    assert_eq!(body.as_ref(), b"fresh-0fresh-1fresh-2");
}

/// Incoming bundle whose range straddles MULTIPLE existing bundles
/// must be dropped as stale if ANY of them is strictly newer than
/// the incoming. (Equivalently: the asymmetric eviction rule
/// short-circuits on the first newer-existing it finds — the
/// incoming never gets a chance to evict the older ones it
/// overlaps.)
///
/// Multi-bundle eviction in the *opposite* direction (one new
/// bundle evicts many older overlapping ones) is excluded by the
/// asymmetric rule itself: a new bundle that's strictly newer than
/// every existing bundle can only reach backward to overlap any of
/// them if its range spans the gap; but spanning multiple existing
/// non-overlapping ranges requires the new range's start to be
/// older than at least one of them, contradicting the strictly-
/// newer premise. So the "evict many in one pass" shape is
/// theoretically possible only on the channel-teardown path; the
/// arrival path always touches at most a localized cluster.
#[test]
fn incoming_overlapping_multiple_with_any_newer_existing_drops_stale() {
    let mut asm = FragmentAssembler::new();
    // Three older bundles: A=[100..=107], B=[110..=117], C=[120..=127].
    asm.add_fragment(100, 0, 8, Bytes::from_static(b"a"))
        .unwrap();
    asm.add_fragment(110, 0, 8, Bytes::from_static(b"b"))
        .unwrap();
    asm.add_fragment(120, 0, 8, Bytes::from_static(b"c"))
        .unwrap();
    assert_eq!(asm.pending_count(), 3);

    // Incoming spans `[105..=128]` (24 fragments at first_seq=105):
    // - Overlaps A at 105..=107
    // - Overlaps B entirely
    // - Overlaps C at 120..=127
    // Incoming.first_seq=105 is newer than A's 100 but OLDER than
    // both B's 110 and C's 120. The asymmetric rule treats it as
    // stale (newer overlapping bundles already in flight) and
    // drops it, leaving A, B, C intact.
    let r = asm
        .add_fragment(105, 0, 24, Bytes::from_static(b"d"))
        .unwrap();
    assert!(r.is_none(), "stale incoming returns Ok(None)");
    assert_eq!(
        asm.pending_count(),
        3,
        "all three older bundles must remain; the stale incoming is dropped"
    );
}

/// Wraparound case for the modular overlap test. With 28-bit
/// sequence space and `SEQUENCE_MASK = 0x0FFF_FFFF`, two bundles
/// whose ranges straddle the wrap boundary must still detect
/// overlap. Earlier naive `max`/`min` overlap would miss these.
#[test]
fn overlap_detection_handles_28_bit_sequence_wraparound() {
    let mut asm = FragmentAssembler::new();

    // Bundle that crosses the 28-bit wrap: starts at 0x0FFFFFFE,
    // 4 fragments → range [0x0FFFFFFE..=0x00000001].
    let wrap_begin = 0x0FFF_FFFEu32;
    asm.add_fragment(wrap_begin, 0, 4, Bytes::from_static(b"wrap"))
        .unwrap();
    assert_eq!(asm.pending_count(), 1);

    // New bundle at the low end overlaps the wrap-bundle at seq 0.
    // Naive `max`/`min` overlap would compute
    // `max(0x00000000, 0x0FFFFFFE) > min(0x00000003, 0x00000001)`
    // and miss the overlap; modular overlap detects it correctly.
    let new_begin = 0x0000_0000u32;
    asm.add_fragment(new_begin, 0, 4, Bytes::from_static(b"new"))
        .unwrap();

    // The wrap-bundle is older (its `first_seq` is "before" the
    // new bundle's `first_seq` in modular space: 0x0FFFFFFE is
    // 3 steps before 0x00000001 across the wrap). So it gets
    // evicted, leaving only the new bundle.
    assert_eq!(
        asm.pending_count(),
        1,
        "wrap-crossing overlap must be detected and evict the older bundle"
    );
}
