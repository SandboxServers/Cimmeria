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
    /// Pure logic: collects retransmits, schedules keepalives, prunes
    /// dead channels — but does no I/O itself. The returned [`TickActions`]
    /// is a punch list for the I/O layer (which today lives in
    /// `services/src/base/connect_loop.rs`, not in this crate).
    ///
    /// **Pruning runs first**, so a channel that is both dead AND has
    /// pending retransmits doesn't have its retransmits queued for a
    /// channel that's about to disappear. After prune, the remaining
    /// channels are checked for retransmits; finally, anything whose
    /// `last_sent` clock has aged past `KEEPALIVE_INTERVAL_MS` is
    /// scheduled for a keepalive emit.
    ///
    /// `tick` touches each living channel's `last_sent` for the keepalive
    /// schedule it emits (via [`Channel::touch_sent`]), so the very next
    /// tick won't re-flag the same channels. The caller is responsible
    /// for actually putting the keepalive bytes on the wire.
    pub fn tick(&mut self) -> TickActions {
        let mut actions = TickActions::default();

        // 1. Prune dead first — drains the registry so we don't queue
        // retransmits/keepalives for channels that are about to go.
        actions.dead_channels = self.prune_dead_channels();

        // 2. Collect retransmits for all surviving channels. This also
        // bumps each channel's `last_sent` (via check_timeouts), so any
        // channel actively retransmitting won't ALSO get scheduled for
        // a keepalive in step 3.
        for (addr, channel) in self.channels.iter_mut() {
            for pkt in channel.check_timeouts() {
                actions.retransmits.push((*addr, pkt));
            }
        }

        // 3. Schedule keepalives. Touch `last_sent` on the channel
        // optimistically — the caller is going to emit the keepalive,
        // and even if it fails the next tick will re-flag the channel.
        // Without the eager touch, two ticks fired close together would
        // double-schedule the same address.
        for (addr, channel) in self.channels.iter_mut() {
            if channel.keepalive_due() {
                channel.touch_sent();
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
    async fn tick_does_not_double_schedule_keepalive() {
        // After tick emits a keepalive request, it touches last_sent.
        // The next tick — fired immediately — must not re-flag the same
        // address; otherwise a tight-loop tick caller (e.g., busy
        // service draining) would double-emit.
        let mut nub = nub().await;
        let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        let ch = nub.get_or_create_channel(addr);
        ch.last_sent = Instant::now()
            - Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);

        let first = nub.tick();
        assert_eq!(first.keepalives.len(), 1);

        // Second tick right after the first — last_sent was just touched.
        let second = nub.tick();
        assert!(second.keepalives.is_empty(), "keepalive must not re-fire on adjacent tick");
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
