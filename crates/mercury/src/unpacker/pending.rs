//! One in-progress reassembly, and the cap-hit tally the assembler keeps.

use bytes::{BufMut, Bytes, BytesMut};

/// Tracks the in-progress reassembly of a single fragmented message.
#[derive(Debug)]
pub(super) struct PendingMessage {
    /// Total number of fragments expected.
    pub(super) total_fragments: u8,
    /// Received fragment payloads, indexed by fragment number.
    fragments: Vec<Option<Bytes>>,
    /// How many fragments have been received so far.
    pub(super) received_count: u8,
    /// Payload bytes held by this message, counted against
    /// [`crate::consts::MAX_PENDING_FRAGMENT_BYTES`].
    pub(super) bytes: usize,
    /// Arrival ticket: the order in which bundles were first seen on the
    /// channel. The cap evicts the lowest ticket first.
    pub(super) admitted: u64,
}

impl PendingMessage {
    pub(super) fn new(total_fragments: u8, admitted: u64) -> Self {
        Self {
            total_fragments,
            fragments: (0..total_fragments).map(|_| None).collect(),
            received_count: 0,
            bytes: 0,
            admitted,
        }
    }

    /// Whether fragment `index` has already been stored.
    pub(super) fn has(&self, index: u8) -> bool {
        self.fragments
            .get(index as usize)
            .is_some_and(|slot| slot.is_some())
    }

    /// Insert a fragment. Returns `true` if the message is now complete.
    pub(super) fn insert(&mut self, index: u8, data: Bytes) -> bool {
        let idx = index as usize;
        if idx >= self.fragments.len() {
            return false;
        }
        if self.fragments[idx].is_none() {
            self.bytes += data.len();
            self.fragments[idx] = Some(data);
            self.received_count += 1;
        }
        self.received_count == self.total_fragments
    }

    /// Assemble the complete message from all fragments in order.
    pub(super) fn assemble(self) -> Bytes {
        let mut buf = BytesMut::with_capacity(self.bytes);
        for frag in self.fragments.into_iter().flatten() {
            buf.put_slice(&frag);
        }
        buf.freeze()
    }
}

/// Why the assembler dropped an incomplete bundle to stay inside its caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentCapReason {
    /// A new bundle arrived with [`crate::consts::MAX_PENDING_FRAGMENTED_BUNDLES`]
    /// already open; the earliest-arrived one was evicted.
    BundleCount,
    /// A fragment would have pushed the held payload past
    /// [`crate::consts::MAX_PENDING_FRAGMENT_BYTES`]; the earliest-arrived
    /// other bundle was evicted.
    PendingBytes,
    /// One bundle alone would exceed the byte cap; it was dropped.
    OversizeBundle,
}

impl FragmentCapReason {
    /// Stable label for structured logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BundleCount => "bundle_count_cap",
            Self::PendingBytes => "pending_bytes_cap",
            Self::OversizeBundle => "oversize_bundle",
        }
    }
}

/// Bundles the assembler dropped to stay inside its caps since the tally
/// was last taken. See [`super::FragmentAssembler::take_cap_hits`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FragmentCapHits {
    /// Evicted because the bundle-count cap was reached.
    pub count_evictions: u64,
    /// Evicted because the pending-bytes cap was reached.
    pub byte_evictions: u64,
    /// Dropped because a single bundle exceeded the byte cap.
    pub oversize_drops: u64,
    /// The most recent drop: reason, `first_seq`, fragments received, and
    /// fragments expected.
    pub last: Option<(FragmentCapReason, u32, u8, u8)>,
}

impl FragmentCapHits {
    /// Total bundles dropped.
    pub fn total(&self) -> u64 {
        self.count_evictions + self.byte_evictions + self.oversize_drops
    }

    /// True when nothing was dropped.
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// Fold `other` into `self`, keeping `other`'s most recent drop.
    pub fn merge(&mut self, other: FragmentCapHits) {
        self.count_evictions += other.count_evictions;
        self.byte_evictions += other.byte_evictions;
        self.oversize_drops += other.oversize_drops;
        if other.last.is_some() {
            self.last = other.last;
        }
    }

    pub(super) fn record(&mut self, reason: FragmentCapReason, msg: &PendingMessage, seq: u32) {
        match reason {
            FragmentCapReason::BundleCount => self.count_evictions += 1,
            FragmentCapReason::PendingBytes => self.byte_evictions += 1,
            FragmentCapReason::OversizeBundle => self.oversize_drops += 1,
        }
        self.last = Some((reason, seq, msg.received_count, msg.total_fragments));
    }
}
