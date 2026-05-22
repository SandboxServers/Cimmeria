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
    ///   `seq_id`/`frag_begin`/`frag_end`, a modular fragment count
    ///   exceeding `MAX_FRAGMENTS`, or this packet's seq outside the
    ///   declared range. Wire-arriving `frag_end < frag_begin` is
    ///   treated as a legitimate 28-bit-space wrap, not an error
    ///   (matches `add_fragment`'s modular semantics).
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

        // Modular fragment-count derivation. Sequence numbers live in
        // a 28-bit ring (spec §1.7 + §2.4 R4), so a wire-arriving bundle
        // with `frag_end < frag_begin` in u32 is a legitimate wrap
        // (e.g. begin=0x0FFFFFFE, end=0x00000001, total=4 across the
        // wrap boundary). Reject only when the implied total exceeds
        // `MAX_FRAGMENTS` — under modular arithmetic every (begin, end)
        // pair represents *some* range; a garbage range like begin=10,
        // end=5 implies a ~268M-fragment wrap, naturally caught by the
        // cap. Uses the same `SEQUENCE_MASK` arithmetic as
        // `add_fragment`'s `ranges_overlap_mod28` / `is_strictly_newer_mod28`
        // so the two entry points handle wraparound identically.
        let total_u64 = (end.wrapping_sub(begin) & SEQUENCE_MASK) as u64 + 1;
        if total_u64 > MAX_FRAGMENTS as u64 {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "fragment range {begin}..={end} ({total_u64} fragments) exceeds MAX_FRAGMENTS {MAX_FRAGMENTS}"
            )));
        }
        let total_frags = total_u64 as u8;

        // seq must lie within the modular range — otherwise we'd map
        // to a nonsensical fragment index. Modular subtraction handles
        // the wrap case (e.g. for a [0x0FFFFFFE..=0x00000001] bundle,
        // seq=0x00000000 → idx=2).
        let idx_u32 = seq.wrapping_sub(begin) & SEQUENCE_MASK;
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
mod tests;
