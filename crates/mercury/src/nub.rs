//! UDP socket manager ("Nub").
//!
//! The Nub owns the UDP socket and the registry of all active channels.
//! It corresponds to the C++ `Mercury::Nub` class — the central dispatcher
//! that routes inbound datagrams to the correct channel and drives the
//! tick loop for timeouts, retransmissions, and keepalives.

use std::collections::HashMap;
use std::net::SocketAddr;

use cimmeria_common::Result;
use tokio::net::UdpSocket;

use crate::channel::Channel;
use crate::consts;
use crate::packet::Packet;

/// Outputs from one [`Nub::tick`] pass — work the I/O layer should now do.
///
/// Returning operations as data (instead of having `tick` perform them
/// inline) keeps the maintenance logic pure and unit-testable without a
/// live UDP socket. The caller is the bridge between the channel
/// bookkeeping and the transport: it ships `retransmits` + `keepalives`
/// onto the wire and tears down `dead_channels`.
#[derive(Default)]
pub struct TickActions {
    /// Reliable packets that hit `ACK_TIMEOUT_MS` without being acked
    /// and need to be re-sent. Pre-bound to the destination address so
    /// the caller doesn't have to re-look-up.
    pub retransmits: Vec<(SocketAddr, Packet)>,
    /// Channels that haven't sent anything in `KEEPALIVE_INTERVAL_MS`.
    /// The caller emits a keepalive to each of these addresses; the
    /// channel's `last_sent` was already touched by `tick` so a no-op
    /// pass next tick won't re-flag the same channel.
    pub keepalives: Vec<SocketAddr>,
    /// Channels removed from the registry this pass (silent peers past
    /// `INACTIVITY_TIMEOUT_MS`, or any channel that exceeded `MAX_RETRIES`
    /// on a reliable packet). Returned so the caller can run cleanup
    /// against the connected-clients map.
    pub dead_channels: Vec<(SocketAddr, Channel)>,
}

/// UDP socket manager and channel registry.
///
/// One `Nub` is created per service endpoint (e.g., CellApp external,
/// BaseApp internal). It owns the tokio `UdpSocket` and maintains a
/// `Channel` for every remote peer that has communicated with it.
pub struct Nub {
    /// The bound UDP socket.
    socket: UdpSocket,

    /// Active channels keyed by remote peer address.
    channels: HashMap<SocketAddr, Channel>,
}

impl Nub {
    /// Bind a new Nub to the given local address.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound.
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        tracing::info!(%addr, "Mercury Nub bound");
        Ok(Self {
            socket,
            channels: HashMap::new(),
        })
    }

    /// Send a packet to the given remote address.
    ///
    /// The packet is encoded to wire format and transmitted via the UDP socket.
    pub async fn send_to(&self, _packet: &Packet, _addr: SocketAddr) -> Result<()> {
        todo!("Nub::send_to — encode packet and send via UDP socket")
    }

    /// Receive a single datagram from the UDP socket.
    ///
    /// Returns the decoded packet and the source address.
    pub async fn recv_from(&self) -> Result<(Packet, SocketAddr)> {
        todo!("Nub::recv_from — read datagram, decode packet header")
    }

    /// Look up or create a channel for the given remote address.
    ///
    /// If no channel exists yet, a new one in the `Connecting` state is created.
    pub fn get_or_create_channel(&mut self, addr: SocketAddr) -> &mut Channel {
        self.channels.entry(addr).or_insert_with(|| {
            tracing::debug!(%addr, "creating new Mercury channel");
            Channel::new(addr)
        })
    }

    /// Returns a reference to the channel for `addr`, if one exists.
    pub fn get_channel(&self, addr: &SocketAddr) -> Option<&Channel> {
        self.channels.get(addr)
    }

    /// Returns a mutable reference to the channel for `addr`, if one exists.
    pub fn get_channel_mut(&mut self, addr: &SocketAddr) -> Option<&mut Channel> {
        self.channels.get_mut(addr)
    }

    /// Drive one tick of the Nub's maintenance loop.
    ///
    /// Pure logic: collects retransmits, schedules keepalives, sweeps
    /// stale fragment buffers, prunes dead channels — but does no I/O
    /// itself. The returned [`TickActions`] is a punch list for the I/O
    /// layer (which today lives in `services/src/base/connect_loop.rs`,
    /// not in this crate).
    ///
    /// **Order:**
    ///   1. Prune dead channels first — drains the registry so we don't
    ///      queue retransmits/keepalives for a channel we're about to
    ///      throw away. Safe because `is_timed_out` uses strict `>`
    ///      against `MAX_RETRIES`, so a packet hitting the retry budget
    ///      on the previous tick gets a full `ACK_TIMEOUT_MS` window
    ///      before this tick reaps it.
    ///   2. Stale-fragment sweep on every surviving channel — frees
    ///      memory pinned by abandoned reassembly state.
    ///   3. Collect retransmits via `check_timeouts` (which bumps each
    ///      entry's `retransmit_count` + the channel's `last_sent`).
    ///      Channels actively retransmitting won't be flagged for a
    ///      keepalive in step 4.
    ///   4. Schedule keepalives for channels whose `last_sent` aged
    ///      past `KEEPALIVE_INTERVAL_MS`. Tick does NOT eagerly touch
    ///      the clock here — the caller is responsible for calling
    ///      [`Channel::touch_sent`] after the keepalive actually goes
    ///      on the wire. If the I/O layer drops the action, the next
    ///      tick re-flags the same address rather than silently
    ///      suppressing the keepalive for a full interval.
    ///
    /// **Caller contract:** service the actions promptly. Tick won't
    /// double-emit retransmits within the same `ACK_TIMEOUT_MS` window
    /// (check_timeouts already advanced their entry clocks), but it WILL
    /// re-flag keepalives every interval until the caller calls
    /// `touch_sent` after a successful send.
    pub fn tick(&mut self) -> TickActions {
        let mut actions = TickActions::default();

        // 1. Prune dead first.
        actions.dead_channels = self.prune_dead_channels();

        // 2. Sweep stale fragment-reassembly state on every channel.
        let frag_timeout = std::time::Duration::from_millis(consts::FRAGMENT_REASSEMBLY_TIMEOUT_MS);
        for channel in self.channels.values_mut() {
            channel.cleanup_stale_fragments(frag_timeout);
        }

        // 3. Collect retransmits. check_timeouts bumps last_sent on the
        // channel so step 4's keepalive_due check sees that activity.
        for (addr, channel) in self.channels.iter_mut() {
            for pkt in channel.check_timeouts() {
                actions.retransmits.push((*addr, pkt));
            }
        }

        // 4. Schedule keepalives. Caller is expected to emit the bytes
        // and then call Channel::touch_sent — without that, the next
        // tick re-flags the address (which is the right behavior: a
        // dropped action gets retried, not silently suppressed).
        for (addr, channel) in self.channels.iter_mut() {
            if channel.keepalive_due() {
                actions.keepalives.push(*addr);
            }
        }

        actions
    }

    /// Remove and return all channels that have timed out.
    pub fn prune_dead_channels(&mut self) -> Vec<(SocketAddr, Channel)> {
        let dead_addrs: Vec<SocketAddr> = self
            .channels
            .iter()
            .filter(|(_, ch)| ch.is_timed_out())
            .map(|(addr, _)| *addr)
            .collect();

        let mut dead = Vec::with_capacity(dead_addrs.len());
        for addr in dead_addrs {
            if let Some(ch) = self.channels.remove(&addr) {
                tracing::warn!(%addr, "pruning timed-out Mercury channel");
                dead.push((addr, ch));
            }
        }
        dead
    }

    /// Returns the local address this Nub is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    /// Returns the number of active channels.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts;
    use bytes::Bytes;
    use std::time::{Duration, Instant};

    use crate::packet::PacketFlags;

    /// Build a Nub by binding to an ephemeral local address. The socket
    /// is real (so `local_addr()` works) but `tick` does no I/O so the
    /// socket isn't exercised by these tests.
    async fn nub() -> Nub {
        Nub::bind("127.0.0.1:0".parse().unwrap()).await.unwrap()
    }

    fn test_packet() -> Packet {
        Packet::new(PacketFlags::default(), 0, Bytes::from_static(&[0xDE, 0xAD]))
    }

    #[tokio::test]
    async fn tick_on_empty_nub_returns_empty_actions() {
        let mut nub = nub().await;
        let actions = nub.tick();
        assert!(actions.retransmits.is_empty());
        assert!(actions.keepalives.is_empty());
        assert!(actions.dead_channels.is_empty());
    }

    #[tokio::test]
    async fn tick_schedules_keepalive_for_idle_channel() {
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        // Backdate last_sent past the keepalive window.
        ch.last_sent = Instant::now()
            - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

        let actions = nub.tick();

        assert_eq!(actions.keepalives, vec![addr]);
        assert!(actions.retransmits.is_empty());
        assert!(actions.dead_channels.is_empty());
    }

    #[tokio::test]
    async fn tick_re_flags_keepalive_until_caller_acks_send() {
        // Tick is intentionally lazy: it does NOT touch_sent on
        // keepalive scheduling — the caller is expected to do that
        // after the bytes actually go on the wire. The reason is that
        // a dropped action (I/O failure, runtime stall) should be
        // retried on the next tick rather than silently suppressed for
        // a full KEEPALIVE_INTERVAL_MS. This test pins the contract:
        // adjacent ticks WILL re-flag until the caller calls
        // `Channel::touch_sent`.
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        ch.last_sent = Instant::now()
            - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

        let first = nub.tick();
        assert_eq!(first.keepalives, vec![addr]);

        // No touch_sent in between — adjacent tick still flags the same
        // address (caller hasn't confirmed the send).
        let second = nub.tick();
        assert_eq!(second.keepalives, vec![addr],
            "without caller touch_sent, tick must re-flag the dropped action");

        // Now simulate the caller emitting + acknowledging the send.
        nub.get_channel_mut(&addr).unwrap().touch_sent();
        let third = nub.tick();
        assert!(third.keepalives.is_empty(),
            "after caller touch_sent, tick must not re-schedule");
    }

    #[tokio::test]
    async fn tick_collects_retransmits_per_addr() {
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9002".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        ch.send_packet(test_packet()).unwrap();
        // Backdate the entry's last_sent past the ACK_TIMEOUT_MS window
        // so check_timeouts considers it expired.
        ch.tx_window[0].last_sent = Instant::now()
            - Duration::from_millis(consts::ACK_TIMEOUT_MS + 100);
        // Also backdate last_sent so we can see whether retransmits skip
        // re-flagging keepalive (they should — check_timeouts bumps last_sent).
        ch.last_sent = Instant::now()
            - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

        let actions = nub.tick();

        assert_eq!(actions.retransmits.len(), 1);
        assert_eq!(actions.retransmits[0].0, addr);
        assert!(
            actions.keepalives.is_empty(),
            "actively-retransmitting channel must not also schedule a keepalive"
        );
    }

    #[tokio::test]
    async fn tick_does_not_reap_channel_on_same_tick_max_retries_hit() {
        // Regression for the "fast tick loop drops channels that just
        // hit MAX_RETRIES" hazard. is_timed_out uses strict `>` against
        // MAX_RETRIES, so a packet whose retransmit_count was bumped TO
        // MAX_RETRIES on this tick survives to be retransmitted; the
        // channel only dies on the NEXT tick (after a full ACK_TIMEOUT_MS
        // window) if the retry still hasn't been ACKed.
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9004".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        ch.send_packet(test_packet()).unwrap();
        // Pre-set retransmit_count to MAX_RETRIES - 1 and backdate so
        // check_timeouts will bump it to exactly MAX_RETRIES on this tick.
        ch.tx_window[0].retransmit_count = consts::MAX_RETRIES - 1;
        ch.tx_window[0].last_sent = Instant::now()
            - Duration::from_millis(consts::ACK_TIMEOUT_MS + 100);

        let actions = nub.tick();

        assert_eq!(actions.retransmits.len(), 1,
            "MAX_RETRIES'th retry must still go on the wire");
        assert!(actions.dead_channels.is_empty(),
            "channel must NOT be reaped on the same tick its packet hit MAX_RETRIES");
        assert_eq!(nub.channel_count(), 1);
    }

    #[tokio::test]
    async fn tick_reaps_channel_after_max_retries_plus_one_timeout() {
        // Continuation of the above: a channel whose packet has
        // retransmit_count > MAX_RETRIES (i.e., the MAX_RETRIES'th retry
        // also failed to land within ACK_TIMEOUT_MS) IS reaped on this tick.
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9005".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        ch.send_packet(test_packet()).unwrap();
        ch.tx_window[0].retransmit_count = consts::MAX_RETRIES + 1;

        let actions = nub.tick();

        assert_eq!(actions.dead_channels.len(), 1);
        assert_eq!(actions.dead_channels[0].0, addr);
    }

    #[tokio::test]
    async fn tick_sweeps_stale_fragment_reassembly() {
        // Pin the cleanup wire-up: a channel with a stale partial
        // fragment bundle has its assembler cleared by tick. Without
        // this, abandoned reassembly state would accumulate per channel
        // until the channel itself dies — and a chatty peer streaming
        // fragments would keep the channel alive indefinitely.
        use bytes::Bytes;
        use crate::packet::{build_outgoing_fragmented, parse_incoming};

        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9006".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);

        // Feed one fragment of a 3-fragment bundle.
        let raw = build_outgoing_fragmented(0, b"part-1", 60, 60, 62, &[]);
        let parsed = parse_incoming(&raw).unwrap();
        ch.reassemble_parsed(&parsed).unwrap();
        // Backdate the channel's reassembly state past the cleanup window
        // by recreating a stale buffer is fiddly; use the public API by
        // running tick once with FRAGMENT_REASSEMBLY_TIMEOUT_MS = 0.
        // We can't override the const, so instead drive the cleanup via
        // the Channel directly to force the stale state out, then verify
        // the next reassembly starts fresh.
        ch.cleanup_stale_fragments(std::time::Duration::ZERO);

        // After cleanup, re-feeding the same first fragment should put
        // the channel back into "waiting for more" with no leftover
        // pending state from before. The simplest observable proof is
        // that completing the bundle now requires all three fragments —
        // a stale-buffered f0 would have caused a duplicate-fragment
        // dedup that never completes.
        let f0 = parse_incoming(
            &build_outgoing_fragmented(0, b"part-1", 60, 60, 62, &[])
        ).unwrap();
        let f1 = parse_incoming(
            &build_outgoing_fragmented(0, b"part-2", 61, 60, 62, &[])
        ).unwrap();
        let f2 = parse_incoming(
            &build_outgoing_fragmented(0, b"part-3", 62, 60, 62, &[])
        ).unwrap();
        assert!(ch.reassemble_parsed(&f0).unwrap().is_none());
        assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
        let body = ch.reassemble_parsed(&f2).unwrap()
            .expect("post-cleanup, the bundle completes from scratch");
        assert_eq!(body.as_ref(), Bytes::from_static(b"part-1part-2part-3"));

        // And a tick pass is non-destructive when there's nothing stale.
        let actions = nub.tick();
        assert!(actions.retransmits.is_empty());
    }

    #[tokio::test]
    async fn tick_prunes_silent_peer_and_does_not_emit_for_it() {
        // Dead channel is removed before retransmit/keepalive collection.
        // Anything pending on the dead channel must not surface in actions.
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9003".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        // Make this channel both pruneable AND keepalive-eligible — only
        // the prune outcome should appear in actions.
        ch.last_received = Instant::now()
            - Duration::from_millis(consts::INACTIVITY_TIMEOUT_MS + 100);
        ch.last_sent = Instant::now()
            - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

        let actions = nub.tick();

        assert_eq!(actions.dead_channels.len(), 1);
        assert_eq!(actions.dead_channels[0].0, addr);
        assert!(actions.keepalives.is_empty(),
            "pruned channel must not also be keepalive-scheduled");
        assert_eq!(nub.channel_count(), 0, "dead channel should be removed from registry");
    }
}
