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
use crate::packet::Bytes;

/// Outputs from one [`Nub::tick`] pass — work the I/O layer should now do.
///
/// Returning operations as data (instead of having `tick` perform them
/// inline) keeps the maintenance logic pure and unit-testable without a
/// live UDP socket. The caller is the bridge between the channel
/// bookkeeping and the transport: it ships `retransmits` + `keepalives`
/// onto the wire and tears down `dead_channels`.
#[derive(Default)]
pub struct TickActions {
    /// Reliable packet datagrams (already encrypted) that hit their
    /// channel's adaptive RTO without being acked and need to go back
    /// on the wire. Pre-bound to the destination address so the caller
    /// `socket.send_to`s each pair directly — no re-encryption needed.
    pub retransmits: Vec<(SocketAddr, Bytes)>,
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

    // Outbound encode-and-send is handled by
    // [`crate::transport::UdpTransport`]; the recv loop in
    // `services/src/base/connect_loop/mod.rs` owns the inbound
    // `recv_from`/decode path and keeps the concrete `UdpSocket`. The Nub
    // itself owns only pure Mercury logic — channel registry, `tick`,
    // fragment reassembly. The former `Nub::send_to`/`Nub::recv_from`
    // unimplemented stubs (#57) were removed in favor of that split; the
    // recv-side end-to-end harness is tracked by #352.

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
    ///   2. Collect retransmits via `check_timeouts` (which bumps each
    ///      entry's `retransmit_count` + the channel's `last_sent`).
    ///      Channels actively retransmitting won't be flagged for a
    ///      keepalive in step 3.
    ///   3. Schedule keepalives for channels whose `last_sent` aged
    ///      past `KEEPALIVE_INTERVAL_MS`. Tick does NOT eagerly touch
    ///      the clock here — the caller is responsible for calling
    ///      [`Channel::touch_sent`] after the keepalive actually goes
    ///      on the wire. If the I/O layer drops the action, the next
    ///      tick re-flags the same address rather than silently
    ///      suppressing the keepalive for a full interval.
    ///
    /// **Not done:** there is intentionally no fragment-reassembly
    /// sweep here. Per `mercury-wire-format` spec §2.4.1 R13 + §2.10
    /// S6, abandoned reassemblies are evicted only when a new
    /// overlapping bundle arrives (handled inside
    /// [`FragmentAssembler::add_fragment`]) or when the channel itself
    /// is torn down. An earlier implementation ran a 30s periodic
    /// sweep that silently dropped in-progress reassemblies the client
    /// would have kept; that is gone.
    ///
    /// **Caller contract:** service the actions promptly. Tick won't
    /// double-emit retransmits within the same `ACK_TIMEOUT_MS` window
    /// (check_timeouts already advanced their entry clocks), but it WILL
    /// re-flag keepalives every interval until the caller calls
    /// `touch_sent` after a successful send.
    pub fn tick(&mut self) -> TickActions {
        // 1. Prune dead first.
        let mut actions = TickActions {
            dead_channels: self.prune_dead_channels(),
            ..TickActions::default()
        };

        // 2. Collect retransmits. check_timeouts bumps last_sent on the
        // channel so step 3's keepalive_due check sees that activity.
        //
        // (Fragment-reassembly stale-cleanup is deliberately NOT done here.
        // The SGW client has no 30-second periodic sweep — stale partial
        // bundles are evicted only when a NEW overlapping bundle arrives,
        // or when the channel itself is torn down. Per
        // `mercury-wire-format` spec §2.4.1 R13 + §2.10 S6. An older
        // implementation here ran a 30s sweep and silently evicted
        // in-progress reassemblies the client would have kept; that's
        // gone now.)
        for (addr, channel) in self.channels.iter_mut() {
            for pkt in channel.check_timeouts() {
                actions.retransmits.push((*addr, pkt));
            }
        }

        // 3. Schedule keepalives. Caller is expected to emit the bytes
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

    use crate::packet::{Packet, PacketFlags};

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
        ch.last_sent = Instant::now() - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

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
        ch.last_sent = Instant::now() - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

        let first = nub.tick();
        assert_eq!(first.keepalives, vec![addr]);

        // No touch_sent in between — adjacent tick still flags the same
        // address (caller hasn't confirmed the send).
        let second = nub.tick();
        assert_eq!(
            second.keepalives,
            vec![addr],
            "without caller touch_sent, tick must re-flag the dropped action"
        );

        // Now simulate the caller emitting + acknowledging the send.
        nub.get_channel_mut(&addr).unwrap().touch_sent();
        let third = nub.tick();
        assert!(
            third.keepalives.is_empty(),
            "after caller touch_sent, tick must not re-schedule"
        );
    }

    #[tokio::test]
    async fn tick_collects_retransmits_per_addr() {
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9002".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        // Use the bytes-bearing path so retransmits actually go to the
        // wire — `send_packet` entries have empty bytes and skip the
        // retransmit emit even though their counter is bumped.
        let mut pkt = test_packet();
        pkt.sequence = 0;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"on-wire"))
            .unwrap();
        // Backdate the entry's last_sent past the current adaptive RTO
        // so check_timeouts considers it expired (#308 adaptive timeout).
        let backdate_by = ch.rto().current() + Duration::from_millis(100);
        ch.tx_window[0].last_sent = Instant::now() - backdate_by;
        // Also backdate last_sent so we can see whether retransmits skip
        // re-flagging keepalive (they should — check_timeouts bumps last_sent).
        ch.last_sent = Instant::now() - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

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
        // Use the bytes-bearing path so the MAX_RETRIES'th retry actually
        // emits bytes on the wire.
        let mut pkt = test_packet();
        pkt.sequence = 0;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"on-wire"))
            .unwrap();
        // Pre-set retransmit_count to MAX_RETRIES - 1 and backdate so
        // check_timeouts will bump it to exactly MAX_RETRIES on this tick.
        ch.tx_window[0].retransmit_count = consts::MAX_RETRIES - 1;
        let backdate_by = ch.rto().current() + Duration::from_millis(100);
        ch.tx_window[0].last_sent = Instant::now() - backdate_by;

        let actions = nub.tick();

        assert_eq!(
            actions.retransmits.len(),
            1,
            "MAX_RETRIES'th retry must still go on the wire"
        );
        assert!(
            actions.dead_channels.is_empty(),
            "channel must NOT be reaped on the same tick its packet hit MAX_RETRIES"
        );
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
    async fn tick_does_not_touch_fragment_reassembly_state() {
        // Inverse contract of the deleted `tick_sweeps_stale_fragment_reassembly`
        // test. The SGW client has no 30-second periodic reassembly sweep
        // (`mercury-wire-format` spec §2.4.1 R13 + §2.10 S6); the Rust `Nub::tick`
        // must NOT touch in-progress reassembly state either. Orphan partial
        // bundles persist until they're either (a) overlapped by a new bundle
        // (handled at the `FragmentAssembler` layer), or (b) reaped via channel
        // teardown.
        //
        // Pin the contract: feed a partial bundle, run tick, observe the
        // partial state is intact and the bundle still completes when the
        // remaining fragments arrive.
        use crate::packet::{build_outgoing_fragmented, parse_incoming};
        use bytes::Bytes;

        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9006".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);

        // Feed f0 of a 3-fragment bundle.
        let f0 = parse_incoming(&build_outgoing_fragmented(0, b"part-1", 60, 60, 62, &[])).unwrap();
        ch.reassemble_parsed(&f0).unwrap();

        // tick must not evict the partial bundle.
        let actions = nub.tick();
        assert!(actions.retransmits.is_empty());

        // f1 + f2 complete the bundle using the still-held f0.
        let f1 = parse_incoming(&build_outgoing_fragmented(0, b"part-2", 61, 60, 62, &[])).unwrap();
        let f2 = parse_incoming(&build_outgoing_fragmented(0, b"part-3", 62, 60, 62, &[])).unwrap();
        // After tick — re-acquire the channel handle (the previous &mut
        // borrow was released when `actions` returned).
        let ch = nub.get_or_create_channel(addr);
        assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
        let body = ch
            .reassemble_parsed(&f2)
            .unwrap()
            .expect("orphan f0 survives tick; f2 completes the bundle");
        assert_eq!(
            body.as_ref(),
            Bytes::from_static(b"part-1part-2part-3"),
            "tick must not have wiped the original f0 payload",
        );
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
        ch.last_received =
            Instant::now() - Duration::from_millis(consts::INACTIVITY_TIMEOUT_MS + 100);
        ch.last_sent = Instant::now() - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

        let actions = nub.tick();

        assert_eq!(actions.dead_channels.len(), 1);
        assert_eq!(actions.dead_channels[0].0, addr);
        assert!(
            actions.keepalives.is_empty(),
            "pruned channel must not also be keepalive-scheduled"
        );
        assert_eq!(
            nub.channel_count(),
            0,
            "dead channel should be removed from registry"
        );
    }
}
