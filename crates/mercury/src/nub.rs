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
    /// This checks all channels for:
    /// - Retransmission timeouts (re-send unacked reliable packets)
    /// - Dead channels (exceeded max retries)
    /// - Keepalive obligations
    ///
    /// Should be called periodically from the service's event loop.
    pub async fn tick(&mut self) -> Result<()> {
        todo!("Nub::tick — iterate channels, retransmit, prune dead, send keepalives")
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
