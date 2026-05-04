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

use std::collections::HashMap;
use std::time::Instant;

use bytes::{BufMut, Bytes, BytesMut};
use cimmeria_common::{CimmeriaError, Result};

use crate::consts::MAX_FRAGMENTS;
use crate::packet::ParsedPacket;

/// Tracks the in-progress reassembly of a single fragmented message.
#[derive(Debug)]
struct PendingMessage {
    /// Total number of fragments expected.
    total_fragments: u8,
    /// Received fragment payloads, indexed by fragment number.
    fragments: Vec<Option<Bytes>>,
    /// How many fragments have been received so far.
    received_count: u8,
    /// When the first fragment of this message was received.
    started_at: Instant,
}

impl PendingMessage {
    fn new(total_fragments: u8) -> Self {
        Self {
            total_fragments,
            fragments: (0..total_fragments).map(|_| None).collect(),
            received_count: 0,
            started_at: Instant::now(),
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

    /// Remove reassembly entries that have been pending longer than `max_age`.
    ///
    /// Call this periodically to prevent memory leaks from fragments that
    /// will never complete (e.g., lost UDP packets for unreliable messages).
    pub fn cleanup_stale(&mut self, max_age: std::time::Duration) {
        let now = Instant::now();
        self.pending.retain(|seq, msg| {
            let age = now.duration_since(msg.started_at);
            if age > max_age {
                tracing::debug!(
                    first_seq = seq,
                    received = msg.received_count,
                    total = msg.total_fragments,
                    "discarding stale fragment reassembly"
                );
                false
            } else {
                true
            }
        });
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
    fn process_parsed_times_out_incomplete_set() {
        // Acceptance criterion: cleanup with a missing fragment after the
        // bounded window discards the partial set without panicking.
        let mut asm = FragmentAssembler::new();
        let f0 = build_then_parse_fragment(40, 40, 42, b"only-one");
        assert!(asm.process_parsed(&f0).unwrap().is_none());
        assert_eq!(asm.pending_count(), 1);

        asm.cleanup_stale(std::time::Duration::ZERO);
        assert_eq!(asm.pending_count(), 0, "stale partial set must be reaped");
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

    #[test]
    fn cleanup_stale_entries() {
        let mut asm = FragmentAssembler::new();
        // Add one fragment of a 3-fragment message.
        asm.add_fragment(10, 0, 3, Bytes::from_static(b"partial"))
            .unwrap();
        assert_eq!(asm.pending_count(), 1);

        // Cleanup with zero duration should remove it.
        asm.cleanup_stale(std::time::Duration::ZERO);
        assert_eq!(asm.pending_count(), 0);
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

    /// `cleanup_stale` removes only entries older than `max_age` —
    /// fresh entries from the same call to `add_fragment` must
    /// survive. Pin the per-entry age check so a regression that
    /// drops the `started_at` comparison and clears the whole map
    /// can't slip through.
    #[test]
    fn cleanup_stale_leaves_fresh_entries_alone() {
        let mut asm = FragmentAssembler::new();
        asm.add_fragment(10, 0, 3, Bytes::from_static(b"a"))
            .unwrap();
        asm.add_fragment(20, 0, 3, Bytes::from_static(b"b"))
            .unwrap();
        assert_eq!(asm.pending_count(), 2);
        // 1-hour max_age — both entries are fresh, neither should be
        // reaped.
        asm.cleanup_stale(std::time::Duration::from_secs(3600));
        assert_eq!(asm.pending_count(), 2);
    }
}
