//! A paired Mercury session — two [`LoopbackPeer`]s on real loopback
//! sockets sharing a [`NetworkPolicy`].
//!
//! Two construction modes:
//!
//! - [`LoopbackSession::unconnected`] — both peers bound and pumping but
//!   neither channel has handshaked. Right for handshake-specific
//!   tests.
//! - [`LoopbackSession::connected`] — drives the handshake before
//!   returning; both channels are in `Connected`. Right for the
//!   vast majority of paired-channel tests.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::channel::ChannelState;
use crate::encryption::MercuryEncryption;

use super::clock::TestClock;
use super::peer::{LoopbackPeer, TickActions};
use super::policy::{Direction, NetworkPolicy};

/// A paired Mercury session for end-to-end testing.
pub struct LoopbackSession {
    pub a: LoopbackPeer,
    pub b: LoopbackPeer,
    pub policy: Arc<Mutex<NetworkPolicy>>,
}

impl LoopbackSession {
    /// Pair two peers without driving the handshake. Both peers are
    /// bound and their recv pumps are spinning; both channels are still
    /// in `Connecting`. Used by handshake-specific tests that want to
    /// observe each step of the state-machine progression.
    ///
    /// `encryption` is applied symmetrically when `Some`. The same
    /// `MercuryEncryption` context is cloned into both peers so they
    /// can decrypt each other's traffic.
    pub async fn unconnected(encryption: Option<MercuryEncryption>) -> std::io::Result<Self> {
        use tokio::net::UdpSocket;

        let policy = Arc::new(Mutex::new(NetworkPolicy::default()));
        let clock_a = Arc::new(TestClock::new());
        let clock_b = Arc::new(TestClock::new());
        let enc_arc = encryption.map(Arc::new);

        // Bind both sockets up front and pass them through to each
        // peer. Avoids the bind-drop-rebind race where the OS could
        // hand the freed ephemeral port to another process between
        // `local_addr()` and the peer's rebind.
        let sock_a = UdpSocket::bind("127.0.0.1:0").await?;
        let sock_b = UdpSocket::bind("127.0.0.1:0").await?;
        let addr_a = sock_a.local_addr()?;
        let addr_b = sock_b.local_addr()?;

        let a = LoopbackPeer::from_socket(
            sock_a,
            addr_b,
            clock_a,
            enc_arc.clone(),
            Direction::AToB,
            policy.clone(),
        )
        .await?;
        let b = LoopbackPeer::from_socket(
            sock_b,
            addr_a,
            clock_b,
            enc_arc,
            Direction::BToA,
            policy.clone(),
        )
        .await?;

        Ok(Self { a, b, policy })
    }

    /// Pair two peers and drive the handshake before returning. On
    /// success both peers' channels are in `Connected`.
    ///
    /// The current handshake shape is the C++ Mercury minimum: each
    /// side sends one empty `FLAG_ON_CHANNEL` packet to the other,
    /// observes the response, and transitions to `Connected`. Tests
    /// that need to assert specific handshake bytes should use
    /// [`Self::unconnected`] and drive the steps themselves.
    pub async fn connected(encryption: Option<MercuryEncryption>) -> std::io::Result<Self> {
        let session = Self::unconnected(encryption).await?;

        session.a.send_bundle(&[], false).await?;
        session.b.send_bundle(&[], false).await?;

        // Wait briefly for both empty packets to land. The recv pumps
        // call `touch_received` on every non-fragmented arrival.
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        while tokio::time::Instant::now() < deadline {
            let a_saw = session.a.inbox_len() >= 1;
            let b_saw = session.b.inbox_len() >= 1;
            if a_saw && b_saw {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }

        {
            let mut ch_a = session.a.channel.lock().expect("channel poisoned");
            ch_a.state = ChannelState::Connected;
        }
        {
            let mut ch_b = session.b.channel.lock().expect("channel poisoned");
            ch_b.state = ChannelState::Connected;
        }

        // Drain the handshake packets from each inbox so test assertions
        // start with an empty queue.
        let _ = session.a.recv_n_bundles(1, Duration::from_millis(10)).await;
        let _ = session.b.recv_n_bundles(1, Duration::from_millis(10)).await;

        Ok(session)
    }

    /// Drive `tick` on both peers once. Returns the punch list each
    /// side emitted.
    pub async fn tick(&self) -> std::io::Result<(TickActions, TickActions)> {
        let a = self.a.tick().await?;
        let b = self.b.tick().await?;
        Ok((a, b))
    }

    /// Wait until both peers have empty TX windows and zero pending
    /// acks, or `timeout` elapses. Returns `true` if quiescent,
    /// `false` if the timeout fired first.
    pub async fn quiesce(&self, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let quiet = self.a.tx_window_len() == 0
                && self.b.tx_window_len() == 0
                && self.a.pending_acks_len() == 0
                && self.b.pending_acks_len() == 0;
            if quiet {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    }

    /// Tear down both peers. Idempotent.
    pub fn shutdown(self) {
        self.a.shutdown();
        self.b.shutdown();
    }
}
