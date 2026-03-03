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
