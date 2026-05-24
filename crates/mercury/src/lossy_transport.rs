//! A [`BidirectionalTransport`] wrapper that applies seeded
//! filters to the underlying transport:
//!
//! - **Send side**: drop / latency / duplicate (full coverage).
//! - **Recv side**: drop / latency. Duplicate is not honored on
//!   recv — the wrapper would need to buffer a recently-delivered
//!   packet to re-deliver it on the next call, which would change
//!   the recv-API timing semantics. Use send-side duplicate from
//!   the peer wrapper to simulate duplicates the receiver
//!   observes.
//!
//! Production code never reaches for this — it's exposed under the
//! `test-support` feature for services-layer chaos integration
//! tests that want to exercise the real recv loop against a wire
//! profile (LAN, transatlantic, mobile) it wouldn't see in unit
//! tests.
//!
//! # Why this exists alongside `LoopbackSession`
//!
//! [`crate::test_harness::LoopbackSession`] gives you a paired
//! Channel-level testbed. `LossyTransport` is one rung higher: it
//! wraps the actual `BidirectionalTransport` the services-layer
//! recv loop uses, so spinning up a real `BaseService` against a
//! `LossyTransport` exercises every handler in the real send/recv
//! path under chaos. Use `LoopbackSession` to test the Mercury
//! protocol; use `LossyTransport` to test that handlers built on
//! top of it survive lossy wire conditions.

use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::transport::{BidirectionalTransport, Transport};

/// Pre-baked latency / loss profiles for common deployment shapes.
#[derive(Debug, Clone, Copy)]
pub enum LossyProfile {
    /// 0.5 ms one-way latency, 0% loss, 0% duplicate. Default for
    /// "real socket, no chaos" baseline runs.
    Lan,
    /// 15 ms one-way latency, 0.1% loss. Models a US east → west
    /// circuit during normal operation.
    Domestic,
    /// 60 ms one-way latency, 0.5% loss. Models a transatlantic
    /// circuit — the lomiada path. The single-packet drops the
    /// chaos infra was built to reproduce live in this regime.
    Transatlantic,
    /// 80 ms one-way latency with substantial jitter, 1% loss.
    /// Models LTE / 5G mobile connectivity during a partial signal
    /// drop.
    Mobile,
}

/// Configurable filter applied to one direction of the wire.
///
/// `drop_per_thousand` and `duplicate_per_thousand` are clamped to
/// `[0, 1000]` at construction (via [`Self::new`], the profile
/// presets, and the builder methods). Out-of-range values used to
/// silently always-drop / always-dup because the internal compare
/// is `roll < threshold`; clamping makes the upper bound explicit.
#[derive(Debug, Clone)]
pub struct LossyConfig {
    /// One-way latency to inject before each send/recv completion.
    pub latency: Duration,
    /// Probabilistic drop, expressed as `(numerator, 1000)` — e.g.
    /// `5` = 0.5%. Clamped to `[0, 1000]` at construction.
    pub drop_per_thousand: u32,
    /// Probabilistic duplicate (emit twice), same encoding.
    /// Clamped to `[0, 1000]` at construction.
    pub duplicate_per_thousand: u32,
    /// RNG seed for deterministic replay across runs.
    pub rng_seed: u64,
}

/// Upper clamp on the per-thousand fields. Exposed for tests that
/// want to assert the clamp boundary.
pub const MAX_PER_THOUSAND: u32 = 1000;

impl LossyConfig {
    /// Construct a config with explicit values. `drop_per_thousand`
    /// and `duplicate_per_thousand` are clamped to `[0, 1000]`.
    pub fn new(
        latency: Duration,
        drop_per_thousand: u32,
        duplicate_per_thousand: u32,
        rng_seed: u64,
    ) -> Self {
        Self {
            latency,
            drop_per_thousand: drop_per_thousand.min(MAX_PER_THOUSAND),
            duplicate_per_thousand: duplicate_per_thousand.min(MAX_PER_THOUSAND),
            rng_seed,
        }
    }

    /// Pick a config from a [`LossyProfile`] preset.
    pub fn from_profile(profile: LossyProfile) -> Self {
        match profile {
            LossyProfile::Lan => Self::new(Duration::from_micros(500), 0, 0, 0),
            LossyProfile::Domestic => Self::new(Duration::from_millis(15), 1, 0, 0),
            LossyProfile::Transatlantic => Self::new(Duration::from_millis(60), 5, 0, 0),
            LossyProfile::Mobile => Self::new(Duration::from_millis(80), 10, 2, 0),
        }
    }

    /// Override the RNG seed. Returns a new config; **must be
    /// called before wrapping into a `LossyTransport`** because the
    /// transport snapshots the seed into its `ChaCha20Rng` at
    /// construction. Calling `with_seed` on a `LossyConfig` that
    /// has already been used to build a `LossyTransport` will not
    /// reseed the live RNG.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng_seed = seed;
        self
    }
}

/// [`BidirectionalTransport`] wrapper that applies a [`LossyConfig`]
/// independently to the send and recv directions.
///
/// Wraps any inner [`BidirectionalTransport`]. Production code uses
/// [`crate::transport::UdpTransport`] as the inner.
pub struct LossyTransport {
    inner: Arc<dyn BidirectionalTransport>,
    send_config: LossyConfig,
    recv_config: LossyConfig,
    send_rng: Mutex<ChaCha20Rng>,
    recv_rng: Mutex<ChaCha20Rng>,
}

impl LossyTransport {
    /// Wrap `inner` with one config applied to **both** send and
    /// recv directions (the typical setup — a single lossy wire
    /// affects both sides identically).
    pub fn new_symmetric(inner: Arc<dyn BidirectionalTransport>, config: LossyConfig) -> Self {
        Self::new_asymmetric(inner, config.clone(), config)
    }

    /// Wrap `inner` with independent configs for each direction.
    /// Required for lomiada-style asymmetric loss scenarios where
    /// client→server ACKs are dropped but server→client packets
    /// flow fine.
    pub fn new_asymmetric(
        inner: Arc<dyn BidirectionalTransport>,
        send: LossyConfig,
        recv: LossyConfig,
    ) -> Self {
        let send_rng = Mutex::new(ChaCha20Rng::seed_from_u64(send.rng_seed));
        let recv_rng = Mutex::new(ChaCha20Rng::seed_from_u64(recv.rng_seed));
        Self {
            inner,
            send_config: send,
            recv_config: recv,
            send_rng,
            recv_rng,
        }
    }

    fn should_drop(config: &LossyConfig, rng: &mut ChaCha20Rng) -> bool {
        if config.drop_per_thousand == 0 {
            return false;
        }
        rng.random_range(0..1000u32) < config.drop_per_thousand
    }

    fn should_duplicate(config: &LossyConfig, rng: &mut ChaCha20Rng) -> bool {
        if config.duplicate_per_thousand == 0 {
            return false;
        }
        rng.random_range(0..1000u32) < config.duplicate_per_thousand
    }
}

#[async_trait]
impl Transport for LossyTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        let (drop_this, duplicate, latency) = {
            let mut rng = self.send_rng.lock().expect("send_rng poisoned");
            let drop_this = Self::should_drop(&self.send_config, &mut rng);
            let duplicate = if !drop_this {
                Self::should_duplicate(&self.send_config, &mut rng)
            } else {
                false
            };
            (drop_this, duplicate, self.send_config.latency)
        };

        if !latency.is_zero() {
            tokio::time::sleep(latency).await;
        }

        if drop_this {
            // Mimic a wire-side drop: return success on the byte
            // count (kernel's view) but never put bytes on the wire.
            return Ok(bytes.len());
        }

        let n = self.inner.send_to(bytes, addr).await?;
        if duplicate {
            // Best-effort dup; ignore the secondary I/O result so a
            // duplicate failure doesn't propagate as the primary
            // send's outcome.
            let _ = self.inner.send_to(bytes, addr).await;
        }
        Ok(n)
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

/// Maximum drop-and-retry iterations inside `recv_from` before
/// returning an error. Guards against the infinite-loop pathology
/// when a test misconfigures recv-side drop at exactly 1000/1000.
/// Set deliberately large so realistic loss rates (≤10%) never
/// trip it — at 10% you'd need 64 consecutive drops to hit the
/// cap, probability ≈ 10⁻⁶⁴.
const MAX_RECV_DROP_LOOPS: u32 = 64;

#[async_trait]
impl BidirectionalTransport for LossyTransport {
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        // Recv path mirrors send: drop / latency / duplicate, but
        // duplicate manifests as "the same delivered packet would be
        // observed twice" — we can't replicate that without buffering,
        // so the recv side currently honors drop + latency only and
        // logs the duplicate decision as a no-op (documented limitation).
        //
        // The drop loop caps at MAX_RECV_DROP_LOOPS to avoid an
        // infinite loop if a test misconfigures 100% recv-side drop.
        // Realistic loss (≤10%) is unaffected.
        for _ in 0..MAX_RECV_DROP_LOOPS {
            let (n, addr) = self.inner.recv_from(buf).await?;
            let drop_this = {
                let mut rng = self.recv_rng.lock().expect("recv_rng poisoned");
                Self::should_drop(&self.recv_config, &mut rng)
            };
            if drop_this {
                continue;
            }
            if !self.recv_config.latency.is_zero() {
                tokio::time::sleep(self.recv_config.latency).await;
            }
            return Ok((n, addr));
        }
        Err(io::Error::other(format!(
            "LossyTransport::recv_from dropped {MAX_RECV_DROP_LOOPS} consecutive packets — \
             recv-side drop_per_thousand may be misconfigured at or near 100%; \
             current config: {}/{MAX_PER_THOUSAND}",
            self.recv_config.drop_per_thousand,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::UdpTransport;
    use tokio::net::UdpSocket;

    fn make_inner(port: u16) -> Arc<dyn BidirectionalTransport> {
        let socket = std::net::UdpSocket::bind(format!("127.0.0.1:{port}")).unwrap();
        socket.set_nonblocking(true).unwrap();
        let tokio_socket = UdpSocket::from_std(socket).unwrap();
        Arc::new(UdpTransport::new(Arc::new(tokio_socket)))
    }

    #[tokio::test]
    async fn lossy_lan_profile_round_trips_a_datagram() {
        let receiver = make_inner(0);
        let recv_addr = receiver.local_addr().unwrap();

        let sender_inner = make_inner(0);
        let lossy = LossyTransport::new_symmetric(
            sender_inner,
            LossyConfig::from_profile(LossyProfile::Lan),
        );

        let payload = b"lossy-lan";
        let n = lossy.send_to(payload, recv_addr).await.unwrap();
        assert_eq!(n, payload.len());

        let mut buf = [0u8; 64];
        let (len, _) = receiver.recv_from(&mut buf).await.unwrap();
        assert_eq!(&buf[..len], payload);
    }

    #[tokio::test]
    async fn recv_side_drop_loops_until_a_packet_passes() {
        // Inner receiver: real UDP socket. Wrapped in LossyTransport
        // that drops 50% of receives.
        let inner = make_inner(0);
        let recv_addr = inner.local_addr().unwrap();
        let lossy = LossyTransport::new_symmetric(
            inner,
            LossyConfig {
                latency: Duration::ZERO,
                drop_per_thousand: 500, // 50%
                duplicate_per_thousand: 0,
                rng_seed: 42,
            },
        );

        // Sender: plain UDP socket; we want the chaos to act only on
        // the recv side of the lossy transport.
        let sender_sock = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        sender_sock.set_nonblocking(true).unwrap();
        let sender = UdpSocket::from_std(sender_sock).unwrap();

        // Send 20 packets in a tight loop.
        for i in 0..20u32 {
            sender
                .send_to(format!("p-{i}").as_bytes(), recv_addr)
                .await
                .unwrap();
        }

        // The lossy recv loops past drops and returns the first
        // surviving packet. With seeded 50% loss over 20 packets,
        // at least one will get through.
        let mut buf = [0u8; 64];
        let result = tokio::time::timeout(Duration::from_secs(2), lossy.recv_from(&mut buf))
            .await
            .expect("recv must yield a non-dropped packet within timeout");
        let (n, _) = result.unwrap();
        assert!(n > 0, "delivered packet must have content");
    }

    #[tokio::test]
    async fn drop_100pct_never_delivers() {
        let receiver = make_inner(0);
        let recv_addr = receiver.local_addr().unwrap();

        let sender_inner = make_inner(0);
        let cfg = LossyConfig::new(Duration::ZERO, 1000, 0, 1);
        let lossy = LossyTransport::new_symmetric(sender_inner, cfg);

        for i in 0..10 {
            let _ = lossy.send_to(format!("p-{i}").as_bytes(), recv_addr).await;
        }

        let mut buf = [0u8; 64];
        let timed_out =
            tokio::time::timeout(Duration::from_millis(100), receiver.recv_from(&mut buf))
                .await
                .is_err();
        assert!(timed_out, "100% drop must deliver nothing");
    }

    #[tokio::test]
    async fn recv_100pct_drop_returns_err_not_infinite_loop() {
        // Sender: plain UDP — wire delivers every packet. Receiver:
        // wrapped in 100% recv-drop LossyTransport.
        let inner = make_inner(0);
        let recv_addr = inner.local_addr().unwrap();
        let lossy =
            LossyTransport::new_symmetric(inner, LossyConfig::new(Duration::ZERO, 1000, 0, 0));

        let sender_sock = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        sender_sock.set_nonblocking(true).unwrap();
        let sender = UdpSocket::from_std(sender_sock).unwrap();

        // Stream more than MAX_RECV_DROP_LOOPS packets so the recv
        // loop has plenty of arrivals to chew on.
        for i in 0..(MAX_RECV_DROP_LOOPS + 10) {
            sender
                .send_to(format!("p-{i}").as_bytes(), recv_addr)
                .await
                .unwrap();
        }

        let mut buf = [0u8; 64];
        let err = tokio::time::timeout(Duration::from_secs(3), lossy.recv_from(&mut buf))
            .await
            .expect("recv must return Err, not infinite-loop until timeout")
            .expect_err("100% drop must yield an error after MAX_RECV_DROP_LOOPS retries");
        let msg = err.to_string();
        assert!(
            msg.contains("dropped") && msg.contains("consecutive"),
            "error must explain the cause; got: {msg}"
        );
    }

    #[test]
    fn lossy_config_clamps_per_thousand_above_1000() {
        let cfg = LossyConfig::new(Duration::ZERO, 2000, 9999, 0);
        assert_eq!(cfg.drop_per_thousand, MAX_PER_THOUSAND);
        assert_eq!(cfg.duplicate_per_thousand, MAX_PER_THOUSAND);
    }

    #[tokio::test]
    async fn asymmetric_config_drops_only_one_direction() {
        // Lomiada shape: outbound (server→client) fine, inbound
        // (client→server acks) dropped.
        let inner = make_inner(0);
        let lossy = LossyTransport::new_asymmetric(
            inner,
            LossyConfig::new(Duration::ZERO, 0, 0, 1), // send: lossless
            LossyConfig::new(Duration::ZERO, 1000, 0, 1), // recv: all-drop
        );
        let peer_inner = make_inner(0);
        let peer_addr = peer_inner.local_addr().unwrap();
        let our_addr = lossy.local_addr().unwrap();

        // Send 5 packets in our direction — every one arrives at peer.
        for i in 0..5u32 {
            lossy
                .send_to(format!("out-{i}").as_bytes(), peer_addr)
                .await
                .unwrap();
        }

        // The peer observed every send.
        let mut buf = [0u8; 64];
        let mut received = 0;
        for _ in 0..5 {
            match tokio::time::timeout(Duration::from_millis(200), peer_inner.recv_from(&mut buf))
                .await
            {
                Ok(Ok(_)) => received += 1,
                _ => break,
            }
        }
        assert_eq!(received, 5, "outbound direction is lossless");

        // The peer sends back. Recv-side 100% drop should make us
        // never observe them. Stream more than MAX_RECV_DROP_LOOPS so
        // the loop hits the guard and errors.
        let peer_sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        peer_sender.set_nonblocking(true).unwrap();
        let peer_sender = UdpSocket::from_std(peer_sender).unwrap();
        for i in 0..(MAX_RECV_DROP_LOOPS + 10) {
            peer_sender
                .send_to(format!("in-{i}").as_bytes(), our_addr)
                .await
                .unwrap();
        }
        let result = tokio::time::timeout(Duration::from_secs(3), lossy.recv_from(&mut buf)).await;
        assert!(
            matches!(result, Ok(Err(_))),
            "100% recv-side drop must yield Err (asymmetric drop direction working)"
        );
    }
}
