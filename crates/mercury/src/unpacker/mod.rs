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
//! 3. **Cap eviction** — the channel already holds
//!    [`crate::consts::MAX_PENDING_FRAGMENTED_BUNDLES`] incomplete bundles when
//!    a new one arrives, or a fragment would push the held payload past
//!    [`crate::consts::MAX_PENDING_FRAGMENT_BYTES`]. The bundle that arrived
//!    earliest is evicted (never the one the incoming fragment belongs
//!    to). A single bundle that alone exceeds the byte cap is dropped.
//!    Each drop is counted in [`FragmentCapHits`], which the owning
//!    `Channel` turns into a rate-limited WARN.
//! 4. **Channel teardown** — the owning `Channel` is dropped, taking
//!    the `FragmentAssembler` with it.
//!
//! There is **no time-based stale sweep**. Per `mercury-wire-format`
//! spec §2.4.1 R13 + §2.10 S6, the SGW client never implemented one;
//! a slow sender (transatlantic link with loss) could legitimately
//! take longer than any reasonable sweep interval to finish a bundle,
//! and a sweep would silently drop it mid-reassembly. The caps bound
//! memory instead of time: they never touch a lone slow bundle.
//!
//! ## Why evict the earliest rather than refuse the newest
//!
//! Refusing new bundles at the cap would let partials left behind by
//! lost unreliable fragments (which nothing else ever removes) block
//! every later bundle on the channel for its whole life, including
//! reliable ones whose fragments the receive window has already acked
//! and the sender will never resend. Evicting the earliest arrival keeps
//! the channel able to reassemble: a legitimate in-progress bundle is
//! only lost if 16 newer bundles open while it waits, which in-order
//! reliable delivery does not produce.
//!
//! ## Cost per fragment
//!
//! Pending bundles are kept in a `BTreeMap` keyed by `first_seq` (masked
//! to 28 bits). A bundle spans at most [`crate::consts::MAX_FRAGMENTS`]
//! sequence numbers, so only keys within `MAX_FRAGMENTS - 1` before the
//! incoming range can overlap it; the overlap checks look at that key
//! window alone. Arrival order lives in a second `BTreeMap` for the cap
//! eviction. Each fragment costs O(log pending), not O(pending).

mod pending;

use std::collections::BTreeMap;

use bytes::Bytes;
use cimmeria_common::{CimmeriaError, Result};

use crate::consts::{MAX_FRAGMENTS, MAX_PENDING_FRAGMENTED_BUNDLES, MAX_PENDING_FRAGMENT_BYTES};
use crate::packet::{ParsedPacket, SEQUENCE_MASK};

use pending::PendingMessage;
pub use pending::{FragmentCapHits, FragmentCapReason};

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

// ── FragmentAssembler ───────────────────────────────────────────────────────

/// Reassembles fragmented Mercury messages.
///
/// Keyed by the sequence number of the first fragment in each message.
/// Once all fragments arrive, the complete payload is returned.
pub struct FragmentAssembler {
    /// In-progress reassembly buffers, keyed by first-fragment sequence
    /// masked to 28 bits.
    pending: BTreeMap<u32, PendingMessage>,
    /// Arrival ticket → `pending` key, oldest first. Drives cap eviction.
    by_arrival: BTreeMap<u64, u32>,
    /// Next arrival ticket to hand out.
    next_ticket: u64,
    /// Sum of [`PendingMessage::bytes`] over `pending`.
    pending_bytes: usize,
    /// Cap drops since the last [`Self::take_cap_hits`].
    cap_hits: FragmentCapHits,
}

impl FragmentAssembler {
    /// Create a new assembler with no pending messages.
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            by_arrival: BTreeMap::new(),
            next_ticket: 0,
            pending_bytes: 0,
            cap_hits: FragmentCapHits::default(),
        }
    }

    /// Add a fragment to the assembler.
    ///
    /// # Arguments
    ///
    /// - `first_seq` — Sequence number of the first fragment (reassembly key).
    ///   Only its low 28 bits are used, matching the modular comparisons.
    /// - `frag_index` — 0-based index of this fragment within the message.
    /// - `total_frags` — Total number of fragments that make up the message.
    /// - `data` — This fragment's payload bytes.
    ///
    /// # Returns
    ///
    /// `Some(complete_payload)` if this was the final missing fragment,
    /// `None` if more fragments are still needed, or if the fragment was
    /// dropped (stale, or its bundle exceeded the byte cap on its own).
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
        let key = first_seq & SEQUENCE_MASK;
        let new_end = key.wrapping_add(total_frags as u32 - 1) & SEQUENCE_MASK;
        let overlapping = self.overlapping_keys(key, new_end);

        if overlapping
            .iter()
            .any(|&existing| is_strictly_newer_mod28(existing, key))
        {
            tracing::debug!(
                first_seq = key,
                last_seq = new_end,
                "Ignoring stale fragment from older overlapping bundle (newer bundle already in flight)"
            );
            return Ok(None);
        }

        // Every remaining overlapping entry is strictly older: evict it.
        for existing in overlapping {
            if let Some(msg) = self.remove(existing) {
                // The completion percentage tells operators whether this
                // was a normal abandonment (low pct) or a suspicious
                // near-complete drop (high pct → possible sender-side bug
                // / loss-driven restart).
                let existing_end = existing.wrapping_add(msg.total_fragments as u32 - 1);
                let completion_pct = (msg.received_count as u32 * 100) / msg.total_fragments as u32;
                tracing::debug!(
                    evicted_first_seq = existing,
                    evicted_last_seq = existing_end,
                    evicted_received = msg.received_count,
                    evicted_total = msg.total_fragments,
                    evicted_completion_pct = completion_pct,
                    evicted_by_first_seq = key,
                    evicted_by_last_seq = new_end,
                    "Discarding abandoned stale overlapping fragmented bundle from seq {} to {}",
                    existing,
                    existing_end,
                );
            }
        }

        // Admission of a new bundle: at the count cap, evict the
        // earliest arrival to make room.
        if !self.pending.contains_key(&key) {
            while self.pending.len() >= MAX_PENDING_FRAGMENTED_BUNDLES {
                if !self.evict_earliest_except(key, FragmentCapReason::BundleCount) {
                    break;
                }
            }
            let ticket = self.next_ticket;
            self.next_ticket += 1;
            self.by_arrival.insert(ticket, key);
            self.pending
                .insert(key, PendingMessage::new(total_frags, ticket));
        }

        let Some(pending) = self.pending.get(&key) else {
            return Ok(None);
        };
        // Sanity: total_frags must match what we saw on the first fragment.
        if pending.total_fragments != total_frags {
            return Err(CimmeriaError::FragmentReassembly(format!(
                "conflicting total_frags for seq {}: expected {}, got {}",
                key, pending.total_fragments, total_frags
            )));
        }
        // A duplicate stores nothing and cannot complete the bundle
        // (a complete bundle is removed the moment it completes).
        if pending.has(frag_index) {
            return Ok(None);
        }

        // Byte cap: make room by evicting earlier arrivals; if this
        // bundle alone would exceed the cap, drop it.
        let len = data.len();
        while self.pending_bytes + len > MAX_PENDING_FRAGMENT_BYTES {
            if !self.evict_earliest_except(key, FragmentCapReason::PendingBytes) {
                if let Some(msg) = self.remove(key) {
                    self.cap_hits
                        .record(FragmentCapReason::OversizeBundle, &msg, key);
                }
                return Ok(None);
            }
        }

        // Store a copy so the held allocation is exactly the counted
        // bytes, not the whole datagram buffer `data` may be a view of.
        let data = Bytes::copy_from_slice(&data);
        let Some(pending) = self.pending.get_mut(&key) else {
            return Ok(None);
        };
        let complete = pending.insert(frag_index, data);
        self.pending_bytes += len;
        if complete {
            // All fragments received — assemble and remove from pending.
            Ok(self.remove(key).map(PendingMessage::assemble))
        } else {
            Ok(None)
        }
    }

    /// Keys of pending bundles whose range overlaps `[key, new_end]`,
    /// excluding `key` itself.
    ///
    /// An existing bundle spans at most `MAX_FRAGMENTS` sequence numbers,
    /// so it can only overlap if its `first_seq` lies within
    /// `MAX_FRAGMENTS - 1` before `key` or inside the incoming range. The
    /// candidate window is therefore at most `2 × MAX_FRAGMENTS - 1` keys
    /// wide, split in two where it crosses the 28-bit wrap.
    fn overlapping_keys(&self, key: u32, new_end: u32) -> Vec<u32> {
        let lo = key.wrapping_sub(MAX_FRAGMENTS as u32 - 1) & SEQUENCE_MASK;
        let candidates: Vec<u32> = if lo <= new_end {
            self.pending.range(lo..=new_end).map(|(&k, _)| k).collect()
        } else {
            self.pending
                .range(lo..=SEQUENCE_MASK)
                .chain(self.pending.range(0..=new_end))
                .map(|(&k, _)| k)
                .collect()
        };
        candidates
            .into_iter()
            .filter(|&existing| existing != key)
            .filter(|&existing| {
                let msg = &self.pending[&existing];
                let existing_end = existing.wrapping_add(msg.total_fragments as u32 - 1);
                ranges_overlap_mod28(key, new_end, existing, existing_end)
            })
            .collect()
    }

    /// Evict the earliest-arrived pending bundle other than `keep`,
    /// recording the drop. Returns `false` when there is none.
    fn evict_earliest_except(&mut self, keep: u32, reason: FragmentCapReason) -> bool {
        let Some(victim) = self
            .by_arrival
            .values()
            .copied()
            .find(|&candidate| candidate != keep)
        else {
            return false;
        };
        if let Some(msg) = self.remove(victim) {
            tracing::debug!(
                target: "mercury.fragment_caps",
                reason = reason.as_str(),
                evicted_first_seq = victim,
                evicted_received = msg.received_count,
                evicted_total = msg.total_fragments,
                evicted_bytes = msg.bytes,
                "Evicting incomplete fragmented bundle to stay within the reassembly caps"
            );
            self.cap_hits.record(reason, &msg, victim);
        }
        true
    }

    /// Remove a pending bundle and its bookkeeping.
    fn remove(&mut self, key: u32) -> Option<PendingMessage> {
        let msg = self.pending.remove(&key)?;
        self.by_arrival.remove(&msg.admitted);
        self.pending_bytes -= msg.bytes;
        Some(msg)
    }

    /// Returns the number of messages currently being reassembled.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Fragment payload bytes currently held by incomplete bundles.
    pub fn pending_bytes(&self) -> usize {
        self.pending_bytes
    }

    /// Bundles dropped to stay inside the caps since the last call, and
    /// reset the tally.
    pub fn take_cap_hits(&mut self) -> FragmentCapHits {
        std::mem::take(&mut self.cap_hits)
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
mod cap_tests;
#[cfg(test)]
mod tests;
