//! Fragment reassembly.
//!
//! When a Mercury message exceeds `MAX_BODY`, it is split across multiple
//! packets each marked with `FLAG_FRAGMENTED`. The `FragmentAssembler`
//! collects these fragments and reconstructs the original message once
//! all pieces have arrived.
//!
//! Fragment header within a fragmented packet body:
//! ```text
//! [u8  fragment_index]   — 0-based index of this fragment
//! [u8  total_fragments]  — total number of fragments in the message
//! [u32 LE first_seq]     — sequence number of the first fragment (reassembly key)
//! [remaining bytes]      — fragment payload
//! ```
//!
//! ## Reassembly lifecycle
//!
//! A partial reassembly persists until one of:
//! 1. **Completion** — every fragment of the bundle has arrived and the
//!    assembled body is returned to the caller.
//! 2. **Arrival-triggered eviction** — a new fragmented bundle arrives
//!    whose sequence range overlaps this one *and whose `first_seq` is
//!    strictly newer* in 28-bit modular sequence space. The older
//!    bundle is treated as abandoned and dropped. A late straggler from
//!    an already-evicted bundle is itself dropped (does not displace
//!    the newer bundle that took over). Matches the SGW client behavior
//!    documented at `ghidra://SGW.exe@0x01b18868`.
//! 3. **Channel teardown** — the owning `Channel` is dropped, taking
//!    the `FragmentAssembler` with it.
//!
//! There is **no time-based stale sweep**. Per `mercury-wire-format`
//! spec §2.4.1 R13 + §2.10 S6, the SGW client never implemented one;
//! a slow sender (transatlantic link with loss) could legitimately
//! take longer than any reasonable sweep interval to finish a bundle,
//! and a sweep would silently drop it mid-reassembly.
//!
//! ## Memory implications
//!
//! Because there is no sweep, orphan partial reassemblies sit in the
//! per-channel assembler until either an overlapping bundle arrives or
//! the channel is torn down. Worst-case footprint per channel is bounded
//! by `MAX_FRAGMENTS` (the per-bundle cap) × the number of distinct
//! `first_seq` keys currently in flight. A malicious peer that opens
//! many partial bundles without completing them pins memory until the
//! channel's existing dead-peer detection (`is_timed_out`) reaps the
//! channel — which is the only safe upper bound under the spec's
//! contract. Channels in practice live minutes-to-hours; this footprint
//! is acceptable and there are easier DoS vectors against a busy server.

use std::collections::HashMap;

use bytes::{BufMut, Bytes, BytesMut};
use cimmeria_common::{CimmeriaError, Result};

use crate::consts::MAX_FRAGMENTS;
use crate::packet::{ParsedPacket, SEQUENCE_MASK};

/// 28-bit modular "is `later` strictly after `earlier`" comparison.
///
/// Sequence numbers live in a 28-bit ring (`mercury-wire-format` spec
/// §1.7 + §2.4 R4). The half-range cutoff (`1 << 27`) decides
/// forward-vs-backward direction across the wraparound at `SEQUENCE_MASK`.
/// Returns `false` when `later == earlier`.
fn is_strictly_newer_mod28(later: u32, earlier: u32) -> bool {
    let diff = later.wrapping_sub(earlier) & SEQUENCE_MASK;
    diff != 0 && diff < 0x0800_0000
}

/// 28-bit modular "do ranges `[a_begin, a_end]` and `[b_begin, b_end]`
/// overlap?" Each range is treated as a contiguous arc on the 28-bit
/// sequence-number ring, with the begin→end direction defined by the
/// modular distance. Because every Mercury bundle is capped at
/// `MAX_FRAGMENTS` fragments and `MAX_FRAGMENTS << (1 << 27)`, no
/// legitimate range exceeds the half-range cutoff, so this test is
/// unambiguous in practice.
fn ranges_overlap_mod28(a_begin: u32, a_end: u32, b_begin: u32, b_end: u32) -> bool {
    let a_len = a_end.wrapping_sub(a_begin) & SEQUENCE_MASK;
    let b_len = b_end.wrapping_sub(b_begin) & SEQUENCE_MASK;
    let b_in_a = (b_begin.wrapping_sub(a_begin) & SEQUENCE_MASK) <= a_len;
    let a_in_b = (a_begin.wrapping_sub(b_begin) & SEQUENCE_MASK) <= b_len;
    b_in_a || a_in_b
}

/// Tracks the in-progress reassembly of a single fragmented message.
#[derive(Debug)]
struct PendingMessage {
    /// Total number of fragments expected.
    total_fragments: u8,
    /// Received fragment payloads, indexed by fragment number.
    fragments: Vec<Option<Bytes>>,
    /// How many fragments have been received so far.
    received_count: u8,
}

impl PendingMessage {
    fn new(total_fragments: u8) -> Self {
        Self {
            total_fragments,
            fragments: (0..total_fragments).map(|_| None).collect(),
            received_count: 0,
        }
    }

    /// Insert a fragment. Returns `true` if the message is now complete.
    fn insert(&mut self, index: u8, data: Bytes) -> bool {
        let idx = index as usize;
        if idx >= self.fragments.len() {
            return false;
        }
        if self.fragments[idx].is_none() {
            self.fragments[idx] = Some(data);
            self.received_count += 1;
        }
        self.received_count == self.total_fragments
    }

    /// Assemble the complete message from all fragments in order.
    fn assemble(self) -> Bytes {
        let total_len: usize = self
            .fragments
            .iter()
            .filter_map(|f| f.as_ref())
            .map(|f| f.len())
            .sum();

        let mut buf = BytesMut::with_capacity(total_len);
        for frag in self.fragments.into_iter().flatten() {
            buf.put_slice(&frag);
        }
        buf.freeze()
    }
}

// ── FragmentAssembler ───────────────────────────────────────────────────────

/// Reassembles fragmented Mercury messages.
///
/// Keyed by the sequence number of the first fragment in each message.
/// Once all fragments arrive, the complete payload is returned.
pub struct FragmentAssembler {
    /// In-progress reassembly buffers, keyed by first-fragment sequence.
    pending: HashMap<u32, PendingMessage>,
}

impl FragmentAssembler {
    /// Create a new assembler with no pending messages.
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    /// Add a fragment to the assembler.
    ///
    /// # Arguments
    ///
    /// - `first_seq` — Sequence number of the first fragment (reassembly key).
    /// - `frag_index` — 0-based index of this fragment within the message.
    /// - `total_frags` — Total number of fragments that make up the message.
    /// - `data` — This fragment's payload bytes.
    ///
    /// # Returns
    ///
    /// `Some(complete_payload)` if this was the final missing fragment,
    /// `None` if more fragments are still needed.
    pub fn add_fragment(
        &mut self,
        first_seq: u32,
        frag_index: u8,
        total_frags: u8,
        data: Bytes,
    ) -> Result<Option<Bytes>> {
        if total_frags == 0 {
            return Err(CimmeriaError::FragmentReassembly(
                "total_frags must be > 0".into(),
            ));
        }
        if total_frags as usize > MAX_FRAGMENTS {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "total_frags {} exceeds MAX_FRAGMENTS {}",
                total_frags, MAX_FRAGMENTS
            )));
        }
        if frag_index >= total_frags {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "frag_index {} >= total_frags {}",
                frag_index, total_frags
            )));
        }

        // Arrival-triggered eviction. See the module doc's "Reassembly
        // lifecycle" section for the full contract; the short version:
        //
        // - The incoming bundle is *stale* (drop silently) if its range
        //   overlaps any in-progress reassembly whose `first_seq` is
        //   strictly *newer* in 28-bit modular sequence space. This
        //   catches the "late straggler from an already-evicted older
        //   bundle" case — without it, the eviction would be symmetric
        //   and the assembler would oscillate between the two ranges
        //   every time a delayed fragment arrived.
        //
        // - Otherwise, every in-progress reassembly whose range
        //   overlaps this one *and whose `first_seq` is strictly older*
        //   is evicted. The new bundle takes over the overlapping
        //   sequence space.
        //
        // The same-`first_seq` case (a peer that re-declares
        // conflicting `total_frags` for a key it's already mid-
        // reassembly on) is a distinct protocol violation handled
        // below as a hard reject, not an eviction.
        let new_begin = first_seq;
        let new_end = first_seq.wrapping_add(total_frags as u32 - 1);

        // Pre-scan: if any overlapping entry is strictly newer than
        // the incoming bundle, this fragment is itself stale.
        let incoming_is_stale = self.pending.iter().any(|(&existing_seq, msg)| {
            if existing_seq == first_seq {
                return false;
            }
            let existing_end = existing_seq.wrapping_add(msg.total_fragments as u32 - 1);
            if !ranges_overlap_mod28(new_begin, new_end, existing_seq, existing_end) {
                return false;
            }
            is_strictly_newer_mod28(existing_seq, first_seq)
        });
        if incoming_is_stale {
            tracing::debug!(
                first_seq,
                last_seq = new_end,
                "Ignoring stale fragment from older overlapping bundle (newer bundle already in flight)"
            );
            return Ok(None);
        }

        // Evict every overlapping entry whose `first_seq` is strictly
        // older than the incoming bundle's. `retain()` is single-pass
        // and avoids the collect+remove dance that would otherwise
        // touch each pending entry twice under burst conditions.
        let evicting_first_seq = first_seq;
        let evicting_last_seq = new_end;
        self.pending.retain(|&existing_seq, msg| {
            if existing_seq == evicting_first_seq {
                return true;
            }
            let existing_end = existing_seq.wrapping_add(msg.total_fragments as u32 - 1);
            if !ranges_overlap_mod28(new_begin, new_end, existing_seq, existing_end) {
                return true;
            }
            if !is_strictly_newer_mod28(evicting_first_seq, existing_seq) {
                return true;
            }
            // Strictly-older overlapping bundle: evict. The completion
            // percentage tells operators whether this was a normal
            // abandonment (low pct) or a suspicious near-complete drop
            // (high pct → possible sender-side bug / loss-driven restart).
            let completion_pct = (msg.received_count as u32 * 100) / msg.total_fragments as u32;
            tracing::debug!(
                evicted_first_seq = existing_seq,
                evicted_last_seq = existing_end,
                evicted_received = msg.received_count,
                evicted_total = msg.total_fragments,
                evicted_completion_pct = completion_pct,
                evicted_by_first_seq = evicting_first_seq,
                evicted_by_last_seq = evicting_last_seq,
                "Discarding abandoned stale overlapping fragmented bundle from seq {} to {}",
                existing_seq,
                existing_end,
            );
            false
        });

        let pending = self
            .pending
            .entry(first_seq)
            .or_insert_with(|| PendingMessage::new(total_frags));

        // Sanity: total_frags must match what we saw on the first fragment.
        if pending.total_fragments != total_frags {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "conflicting total_frags for seq {}: expected {}, got {}",
                first_seq, pending.total_fragments, total_frags
            )));
        }

        if pending.insert(frag_index, data) {
            // All fragments received — assemble and remove from pending.
            let msg = self.pending.remove(&first_seq).unwrap();
            Ok(Some(msg.assemble()))
        } else {
            Ok(None)
        }
    }

    /// Returns the number of messages currently being reassembled.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Feed a freshly-parsed Mercury packet into the assembler.
    ///
    /// Non-fragmented packets pass the body through verbatim so the
    /// receive path doesn't have to branch on `is_fragmented()`.
    /// Fragmented packets buffer until the bundle is complete; the
    /// `FragmentAssembler` reassembles in arrival-independent order.
    ///
    /// Returns:
    /// - `Ok(Some(body))` when the packet is non-fragmented OR when this
    ///   fragment completes the bundle.
    /// - `Ok(None)` when the packet is one of N fragments and we're still
    ///   waiting for the rest.
    /// - `Err(_)` for malformed fragment metadata: missing
    ///   `seq_id`/`frag_begin`/`frag_end`, inverted range, fragment count
    ///   exceeding `MAX_FRAGMENTS`, or this packet's seq outside the
    ///   declared range.
    ///
    /// Mapping from parser footers to assembler keys:
    /// - reassembly key = `frag_begin` (the bundle's anchor seq)
    /// - total fragments = `frag_end - frag_begin + 1`
    /// - this fragment's index = `seq_id - frag_begin`
    pub fn process_parsed(&mut self, pkt: &ParsedPacket) -> Result<Option<Bytes>> {
        if !pkt.is_fragmented() {
            return Ok(Some(pkt.body.clone()));
        }

        let seq = pkt.seq_id.ok_or_else(|| {
            CimmeriaError::FragmentReassembly("FLAG_FRAGMENTED packet missing seq_id footer".into())
        })?;
        let begin = pkt.frag_begin.ok_or_else(|| {
            CimmeriaError::FragmentReassembly(
                "FLAG_FRAGMENTED packet missing frag_begin footer".into(),
            )
        })?;
        let end = pkt.frag_end.ok_or_else(|| {
            CimmeriaError::FragmentReassembly(
                "FLAG_FRAGMENTED packet missing frag_end footer".into(),
            )
        })?;

        if end < begin {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "inverted fragment range: frag_begin={begin}, frag_end={end}"
            )));
        }
        // Promote to u64 before the +1 — `end - begin == u32::MAX`
        // (e.g., begin=0, end=u32::MAX) would overflow and panic in
        // debug / wrap to 0 in release if we computed in u32. The
        // MAX_FRAGMENTS cap below would silently accept that 0.
        let total_u64 = (end as u64) - (begin as u64) + 1;
        // The assembler stores total_fragments as u8 (capped by MAX_FRAGMENTS).
        // Reject anything beyond the cap; the cap exists to bound per-peer
        // reassembly memory.
        if total_u64 > MAX_FRAGMENTS as u64 {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "fragment range {begin}..={end} ({total_u64} fragments) exceeds MAX_FRAGMENTS {MAX_FRAGMENTS}"
            )));
        }
        let total_frags = total_u64 as u8;

        // seq must lie within the range — otherwise we'd map to a
        // nonsensical fragment index. wrapping_sub catches `seq < begin`
        // since the diff would be huge.
        let idx_u32 = seq.wrapping_sub(begin);
        if (idx_u32 as u64) >= total_u64 {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "seq {seq} outside fragment range {begin}..={end}"
            )));
        }
        let frag_index = idx_u32 as u8;

        self.add_fragment(begin, frag_index, total_frags, pkt.body.clone())
    }
}

impl Default for FragmentAssembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
    fn process_parsed_rejects_inverted_range() {
        // frag_end < frag_begin can't represent a real range and would
        // underflow the fragment-count math. Caught at the helper.
        let pkt = build_then_parse_fragment(5, 10, 4, b"bad");

        let mut asm = FragmentAssembler::new();
        let err = asm.process_parsed(&pkt).unwrap_err();
        assert!(matches!(err, CimmeriaError::FragmentReassembly(_)));
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
}
