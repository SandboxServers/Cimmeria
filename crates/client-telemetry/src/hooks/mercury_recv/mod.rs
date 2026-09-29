//! What the client's Mercury receive path did with a datagram, a fragment
//! group and a bundle: the portable half of the
//! `client.mercury.packet_in`, `client.mercury.fragment` and
//! `client.mercury.bundle` events.
//!
//! Everything here reads client memory through [`Mem`](super::entity_trace::map::Mem)
//! or classifies values the detours already read, so it runs and is tested
//! off the DLL target. The detours themselves are
//! `hooks::inline_hooks::mercury_recv`. The layouts and rules are recovered
//! in `docs/reverse-engineering/findings/client-mercury-receive-path.md`.
//!
//! - [`tail`]: the footers of one datagram, parsed the way
//!   `processFilteredPacket` / `processPacket` strip them, and the gate that
//!   decides whether a packet earns an event.
//! - [`window`]: what the reliable window (`queueAckForPacket`) did with a
//!   sequence number, from the channel's counters before and after.
//! - [`fragment`]: the channel's open fragment group before and after
//!   `processPacket`, and what that says happened to the fragment.
//! - [`bundle`]: the packet chain of one bundle and the message loop over it.

pub(crate) mod bundle;
pub(crate) mod fragment;
pub(crate) mod iterator;
pub(crate) mod report;
pub(crate) mod tail;
pub(crate) mod window;

use serde_json::Value;

/// Fields of one event, in emit order.
pub(crate) type Fields = Vec<(&'static str, Value)>;

/// Client `Mercury::Packet` field offsets (refcounted; `data[0]` is the
/// flags byte).
pub(crate) mod packet {
    /// `next` packet of a chain.
    pub const NEXT: u32 = 0x08;
    /// Data length: flags byte included, shrinks as footers are stripped.
    pub const LEN: u32 = 0x24;
    /// Sequence number, once its footer was stripped.
    pub const SEQ: u32 = 0x44;
    /// Start of the datagram bytes; `data[0]` is the flags byte.
    pub const DATA: u32 = 0x54;
}

/// `ChannelInternal` field offsets (the `this` of `queueAckForPacket`, the
/// channel argument of `processPacket`).
pub(crate) mod channel {
    /// How far ahead of `inSeqAt` a reliable packet may arrive.
    pub const WINDOW: u32 = 0x30;
    /// Packets buffered ahead of `inSeqAt`.
    pub const BUFFERED: u32 = 0x48;
    /// Next expected reliable sequence number (`0x10000000` = unset).
    pub const IN_SEQ_AT: u32 = 0x50;
    /// The open fragment group, or `0`.
    pub const FRAG_GROUP: u32 = 0x124;
}

/// Fragment group field offsets.
pub(crate) mod group {
    /// `lastFrag` of the bundle the group collects.
    pub const LAST: u32 = 0x00;
    /// Fragments still missing.
    pub const REMAINING: u32 = 0x04;
    /// Head of the seq-sorted packet list (linked through `Packet+8`).
    pub const LIST: u32 = 0x10;
}

/// `Mercury::Nub` counters.
pub(crate) mod nub {
    /// Bad-packet counter, bumped on almost every drop path.
    pub const BAD_PACKETS: u32 = 0xf8;
    /// Messages dispatched.
    pub const DISPATCHED: u32 = 0x10c;
    /// Bundles whose loop stopped with messages left.
    pub const BUNDLES_ABORTED: u32 = 0x118;
}

/// The sequence-number mask and the "unset" sentinel.
pub(crate) const SEQ_MASK: u32 = 0x0fff_ffff;
pub(crate) const SEQ_UNSET: u32 = 0x1000_0000;

/// Longest datagram read for parsing; Mercury packets are at most 1453
/// payload bytes plus footers.
pub(crate) const MAX_DATAGRAM: usize = 2048;

/// Most packets followed along a chain or a group list before giving up, so
/// a corrupt `next` pointer cannot loop.
pub(crate) const MAX_CHAIN: usize = 128;

/// `after - before`, wrapping; `None` if either read failed.
pub(crate) fn delta(before: Option<u32>, after: Option<u32>) -> Option<u32> {
    Some(after?.wrapping_sub(before?))
}
