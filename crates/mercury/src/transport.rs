//! The outbound side of the Mercury transport.
//!
//! Handler code in `cimmeria-services` emits wire bytes by calling
//! [`Transport::send_to`]. By depending on the trait object
//! `&Arc<dyn Transport>` instead of a concrete `Arc<UdpSocket>`, every send
//! site becomes injectable — production wires in [`UdpTransport`], tests wire
//! in a recording fake ([`crate::test_transport::TestTransport`], behind the
//! `test-support` feature) and assert on the exact `(SocketAddr, bytes)` pairs
//! that fanned out.
//!
//! The trait deliberately covers only the **outbound** direction. The recv
//! loop in `services/src/base/connect_loop/mod.rs` keeps its concrete
//! [`tokio::net::UdpSocket`] because handlers don't read from the socket —
//! they only emit. End-to-end recv-side testing is the responsibility of the
//! Tier 2 Mercury loopback harness, not this trait. See
//! `docs/architecture/transport-trait.md` for the full rationale.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::UdpSocket;

/// The outbound side of the Mercury transport.
///
/// Implementations:
/// - [`UdpTransport`] — production wrapper around [`tokio::net::UdpSocket`].
/// - [`crate::test_transport::TestTransport`] — records every
///   `(SocketAddr, Vec<u8>)` send so tests can assert byte-exact fan-out
///   (behind the `test-support` feature).
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send `bytes` to `addr`. Errors propagate from the underlying I/O.
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize>;

    /// The address this transport is bound to. For
    /// [`crate::test_transport::TestTransport`] this is a synthetic loopback
    /// address chosen at construction.
    fn local_addr(&self) -> io::Result<SocketAddr>;
}

/// Production transport: a thin wrapper around an `Arc<UdpSocket>`.
///
/// Constructed once per service endpoint in the recv loop, which keeps the
/// concrete socket for `recv_from` and hands a `&Arc<dyn Transport>` down to
/// every handler for the send side.
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
}

impl UdpTransport {
    /// Wrap an already-bound shared UDP socket.
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self { socket }
    }
}

#[async_trait]
impl Transport for UdpTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.socket.send_to(bytes, addr).await
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn udp_transport_round_trips_a_datagram_on_loopback() {
        // Bind two real loopback sockets; send through the UdpTransport
        // wrapper and confirm the bytes arrive at the peer. This is the one
        // place the production wrapper is exercised against a real socket;
        // handler unit tests use TestTransport instead.
        let receiver = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let recv_addr = receiver.local_addr().unwrap();

        let sender_sock = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let transport = UdpTransport::new(Arc::clone(&sender_sock));

        let payload = b"mercury-payload";
        let n = transport.send_to(payload, recv_addr).await.unwrap();
        assert_eq!(n, payload.len());

        let mut buf = [0u8; 64];
        let (len, from) = receiver.recv_from(&mut buf).await.unwrap();
        assert_eq!(&buf[..len], payload);
        assert_eq!(from, sender_sock.local_addr().unwrap());
    }

    #[tokio::test]
    async fn udp_transport_local_addr_matches_underlying_socket() {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let expected = socket.local_addr().unwrap();
        let transport = UdpTransport::new(socket);
        assert_eq!(transport.local_addr().unwrap(), expected);
    }
}
