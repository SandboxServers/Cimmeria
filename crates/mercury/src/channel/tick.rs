//! Tick-driver contract: the [`TickActions`] punch list a driver collects
//! when fanning a single tick pass across a collection of [`Channel`]s.
//!
//! [`Channel`]: super::Channel

use std::net::SocketAddr;

use bytes::Bytes;

use super::Channel;

/// Punch list returned by a tick pass over a collection of channels.
///
/// Cimmeria's actual tick driver lives in
/// `crates/services/src/base/tick_sync.rs` — a per-session task that drives
/// one channel per loop iteration via [`Channel::check_timeouts`],
/// [`Channel::keepalive_due`], and [`Channel::is_timed_out`] directly. This
/// type exists to document the bridge contract for any future driver that
/// needs to fan a single tick across many channels (e.g., a registry-style
/// loop). The fields are the I/O punch list; ownership of the actual send /
/// teardown lives with the caller.
///
/// **Ordering invariant the caller must preserve** (mirrors C++ BigWorld's
/// `BaseNub::processPendingEvents` order, and the per-session loop in
/// `tick_sync.rs` matches it implicitly because it only sees one channel):
///
///   1. **Prune dead channels first.** [`Channel::is_timed_out`] uses
///      strict `>` against `consts::MAX_RETRIES`, so a packet whose
///      retransmit_count was bumped TO `MAX_RETRIES` on the previous tick
///      gets a full `ACK_TIMEOUT_MS` window to land before this tick reaps
///      the channel. Pruning before collecting retransmits prevents queuing
///      work for a channel we're about to throw away.
///   2. **Collect retransmits** via [`Channel::check_timeouts`]. That call
///      bumps each touched entry's `retransmit_count` and the channel's
///      `last_sent` — so a channel actively retransmitting will NOT also be
///      flagged for a keepalive in step 3.
///   3. **Schedule keepalives** via [`Channel::keepalive_due`]. The
///      contract is intentionally lazy: a tick that emits a keepalive does
///      NOT call [`Channel::touch_sent`] eagerly — the caller is expected
///      to call it after the bytes actually go on the wire. If the I/O
///      layer drops the action, the next tick re-flags the same channel
///      rather than silently suppressing the keepalive for a full interval.
///
/// **Not done at this layer:** there is intentionally no fragment-reassembly
/// sweep. Per `mercury-wire-format` spec §2.4.1 R13 + §2.10 S6, abandoned
/// reassemblies are evicted only when a new overlapping bundle arrives
/// (handled inside [`crate::unpacker::FragmentAssembler::add_fragment`]) or
/// when the channel itself is torn down. An earlier implementation ran a
/// 30s periodic sweep that silently dropped in-progress reassemblies the
/// client would have kept; that is gone.
#[derive(Default)]
pub struct TickActions {
    /// Reliable packet datagrams (already encrypted) that hit their
    /// channel's adaptive RTO without being acked and need to go back
    /// on the wire. Pre-bound to the destination address so the caller
    /// `socket.send_to`s each pair directly — no re-encryption needed.
    pub retransmits: Vec<(SocketAddr, Bytes)>,
    /// Channels that haven't sent anything in `KEEPALIVE_INTERVAL_MS`.
    /// The caller emits a keepalive to each of these addresses and is
    /// expected to call [`Channel::touch_sent`] after the bytes land on
    /// the wire — a dropped action will be re-flagged on the next tick
    /// rather than silently suppressed for a full interval.
    pub keepalives: Vec<SocketAddr>,
    /// Channels removed from the registry this pass (silent peers past
    /// `INACTIVITY_TIMEOUT_MS`, or any channel that exceeded
    /// `MAX_RETRIES` on a reliable packet). Returned so the caller can
    /// run cleanup against per-session state outside the channel itself.
    pub dead_channels: Vec<(SocketAddr, Channel)>,
}
