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

use bytes::{Bytes, BytesMut, BufMut};
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
    /// The receive-path integration that issue #121 was filing for: the
    /// `FragmentAssembler` existed but had zero callers, so any inbound
    /// packet larger than a single datagram (initial inventory snapshot,
    /// full ability list, large region descriptor) was silently dropped
    /// or mis-decoded by downstream code that didn't understand the
    /// fragment footers.
    ///
    /// Returns:
    /// - `Ok(Some(body))` when the packet is non-fragmented (body passes
    ///   through verbatim) OR when this packet completes a fragmented
    ///   bundle (body is the assembled payload).
    /// - `Ok(None)` when the packet is one of N fragments and we're still
    ///   waiting for the rest.
    /// - `Err(_)` for malformed fragment metadata: missing
    ///   `seq_id`/`frag_begin`/`frag_end` (the parser should have set them
    ///   together but defensive), inverted range, fragment count exceeding
    ///   `MAX_FRAGMENTS`, or this packet's seq outside the declared range.
    ///
    /// Mapping from parser footers to assembler keys:
    /// - reassembly key = `frag_begin` (the bundle's anchor seq)
    /// - total fragments = `frag_end - frag_begin + 1`
    /// - this fragment's index = `seq_id - frag_begin`
    pub fn process_parsed(&mut self, pkt: &ParsedPacket) -> Result<Option<Bytes>> {
        if !pkt.is_fragmented() {
            // Pass-through: caller hands us non-fragmented packets too so
            // the receive path doesn't have to branch.
            return Ok(Some(pkt.body.clone()));
        }

        let seq = pkt.seq_id.ok_or_else(|| {
            CimmeriaError::FragmentReassembly(
                "FLAG_FRAGMENTED packet missing seq_id footer".into(),
            )
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
        let total_u32 = end - begin + 1;
        // The assembler stores total_fragments as u8 (capped by MAX_FRAGMENTS).
        // Reject anything that would overflow OR exceed the cap before we
        // even try the cast — the cap exists to bound reassembly memory
        // per peer, so a malicious or buggy sender shouldn't be able to
        // push us past it.
        if total_u32 as usize > MAX_FRAGMENTS {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "fragment range {begin}..={end} ({total_u32} fragments) exceeds MAX_FRAGMENTS {MAX_FRAGMENTS}"
            )));
        }
        let total_frags = total_u32 as u8;

        // seq must lie within the range — otherwise we'd map to a
        // nonsensical fragment index. wrapping_sub catches `seq < begin`
        // since the diff would be huge.
        let idx_u32 = seq.wrapping_sub(begin);
        if idx_u32 >= total_u32 {
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

    // ── #121: receive-path integration via process_parsed ────────────

    /// Helper: build a fragmented Mercury packet via the public encoder
    /// then parse it back. Mirrors what the live receive loop will do
    /// per fragment after this PR wires the assembler into channel.rs.
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
        let body = asm.process_parsed(&parsed).unwrap().expect("non-fragmented should pass through");
        assert_eq!(body.as_ref(), b"hello");
        assert_eq!(asm.pending_count(), 0, "no reassembly state for pass-through");
    }

    #[test]
    fn process_parsed_reassembles_in_order_3_fragment_bundle() {
        // The named acceptance criterion from #121: a hand-built 3-fragment
        // bundle round-trips through parse + assembler integration.
        let mut asm = FragmentAssembler::new();
        let f0 = build_then_parse_fragment(10, 10, 12, b"AAA");
        let f1 = build_then_parse_fragment(11, 10, 12, b"BBB");
        let f2 = build_then_parse_fragment(12, 10, 12, b"CCC");

        assert!(asm.process_parsed(&f0).unwrap().is_none());
        assert!(asm.process_parsed(&f1).unwrap().is_none());
        let body = asm.process_parsed(&f2).unwrap().expect("third fragment completes");
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
        let body = asm.process_parsed(&f1).unwrap().expect("last (index 1) completes");
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
        let body = asm.process_parsed(&f1).unwrap().expect("f1 still completes once after dup f0");
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
        assert_eq!(asm.pending_count(), 0, "rejected range must not register pending");
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
}
