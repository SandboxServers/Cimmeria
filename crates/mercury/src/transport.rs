//! The outbound side of the Mercury transport, plus an opt-in
//! bidirectional extension for the recv loop and chaos-testing.
//!
//! Handler code in `cimmeria-services` emits wire bytes by calling
//! [`Transport::send_to`]. By depending on the trait object
//! `&Arc<dyn Transport>` instead of a concrete `Arc<UdpSocket>`, every send
//! site becomes injectable — production wires in [`UdpTransport`], tests wire
//! in a recording fake ([`crate::test_transport::TestTransport`], behind the
//! `test-support` feature) and assert on the exact `(SocketAddr, bytes)` pairs
//! that fanned out.
//!
//! For the **recv** loop, the [`BidirectionalTransport`] super-trait extends
//! [`Transport`] with `recv_from`. Production runs [`UdpTransport`] (which
//! implements both); chaos tests run a `LossyTransport` that applies seeded
//! drop + latency filters in both directions (plus send-side duplicate;
//! recv-side duplicate is a documented limitation). Handlers continue to
//! see only `&Arc<dyn Transport>` so the send-side ADR is unchanged; the
//! recv seam is widened just enough to let the chaos apparatus wrap it.

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
        let result = self.socket.send_to(bytes, addr).await;
        // mercury.packet recording lives here (the actual production
        // hot path) rather than in `Channel::send_packet`. The richer
        // recording in Channel records `seq`/`flags` but Channel
        // isn't called in production — only in tests. Without this
        // helper at the transport layer, the entire `mercury.packet`
        // log stream is empty in SigNoz.
        tracing::info!(
            target: "mercury.packet",
            dir = "out",
            transport = "udp",
            len = bytes.len(),
            peer = %addr,
            ok = result.is_ok(),
            "mercury_packet"
        );
        result
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
}

/// Extension of [`Transport`] that also exposes the **inbound** side
/// of the wire. Used by the recv loop in
/// `crates/services/src/base/connect_loop/mod.rs` and by the
/// chaos-testing `LossyTransport`. Production handlers still take
/// `&Arc<dyn Transport>` (send-only); only the recv loop holds a
/// `BidirectionalTransport`.
///
/// The extension is opt-in so the byte-exact fan-out test seam
/// (`TestTransport`) doesn't need to grow a `recv_from` it has no
/// use for.
#[async_trait]
pub trait BidirectionalTransport: Transport {
    /// Receive a datagram into `buf`. Returns `(bytes_written,
    /// source_addr)`. Errors propagate from the underlying I/O.
    /// Same shape as [`tokio::net::UdpSocket::recv_from`].
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
}

#[async_trait]
impl BidirectionalTransport for UdpTransport {
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let result = self.socket.recv_from(buf).await;
        if let Ok((len, addr)) = &result {
            // See the send_to comment above — production hot path.
            tracing::info!(
                target: "mercury.packet",
                dir = "in",
                transport = "udp",
                len = *len,
                peer = %addr,
                "mercury_packet"
            );
        }
        result
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
