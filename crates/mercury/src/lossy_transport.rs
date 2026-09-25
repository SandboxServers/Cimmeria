//! A [`BidirectionalTransport`] wrapper that applies seeded
//! filters to the underlying transport:
//!
//! - **Send side**: drop / latency+jitter / duplicate / reorder (full
//!   coverage — `reorder_buffer_size` mirrors
//!   `test_harness::policy::NetworkPolicy`'s multi-packet reorder buffer:
//!   hold N sends, flush in reverse arrival order when full).
//! - **Recv side**: drop / latency+jitter. Duplicate and reorder are not
//!   honored on recv — the wrapper would need to buffer recently-delivered
//!   packets to re-deliver/reshuffle them on a later call, which would
//!   change the recv-API timing semantics. Use send-side duplicate/reorder
//!   from the peer wrapper to simulate what the receiver observes.
//! - **Deterministic burst drop**: [`LossyTransport::drop_next_sends`]
//!   forces the next N sends to drop unconditionally, independent of
//!   `drop_per_thousand` — for tests that need to hit one specific packet
//!   (e.g. "the reliable bundle carrying a specific AoI introduction")
//!   rather than relying on a probabilistic roll to eventually land on it.
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
use std::sync::atomic::{AtomicU32, Ordering};
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
    /// Additional random latency on top of `latency`, uniformly
    /// distributed in `[0, jitter]`, redrawn per packet. `Duration::ZERO`
    /// (the default via [`Self::new`]) disables jitter — every packet
    /// gets exactly `latency`. Combined with `reorder_buffer_size`,
    /// variable per-packet delay is what lets sends complete (and thus
    /// land on the wire) out of their original order without an
    /// explicit reorder buffer.
    pub jitter: Duration,
    /// Probabilistic drop, expressed as `(numerator, 1000)` — e.g.
    /// `5` = 0.5%. Clamped to `[0, 1000]` at construction.
    pub drop_per_thousand: u32,
    /// Probabilistic duplicate (emit twice), same encoding.
    /// Clamped to `[0, 1000]` at construction.
    pub duplicate_per_thousand: u32,
    /// Send-side reorder: hold up to `N` sends, then flush all of them
    /// (including the one that filled the buffer) in **reverse arrival
    /// order**. `0` (the default via [`Self::new`]) disables reordering.
    /// Mirrors `test_harness::policy::NetworkPolicy::reorder_buffer_size`.
    /// Only meaningful on the send side — see the module doc for why recv
    /// doesn't support it.
    pub reorder_buffer_size: u32,
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
            jitter: Duration::ZERO,
            drop_per_thousand: drop_per_thousand.min(MAX_PER_THOUSAND),
            duplicate_per_thousand: duplicate_per_thousand.min(MAX_PER_THOUSAND),
            reorder_buffer_size: 0,
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

    /// Add jitter: `[0, jitter]` extra random delay redrawn per packet on
    /// top of the base `latency`. Returns a new config.
    pub fn with_jitter(mut self, jitter: Duration) -> Self {
        self.jitter = jitter;
        self
    }

    /// Enable send-side reordering: hold up to `size` sends, flush
    /// reversed once full. `0` disables. Returns a new config.
    pub fn with_reorder_buffer(mut self, size: u32) -> Self {
        self.reorder_buffer_size = size;
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
    /// Send-side reorder buffer, keyed by destination address: each
    /// destination accumulates its own held packets independently and
    /// flushes (reversed) once *its own* bucket reaches
    /// `send_config.reorder_buffer_size`. Per-destination bucketing
    /// matters on a transport that multiplexes several peers through one
    /// socket (the real `BaseService` case) -- a single shared buffer
    /// would hold up one client's handshake reply behind an unrelated
    /// client's traffic (or vice versa), which isn't what network
    /// reordering does on a real wire (each flow reorders independently).
    reorder_buffer: Mutex<std::collections::HashMap<SocketAddr, Vec<Vec<u8>>>>,
    /// Deterministic burst-drop counter, decremented on every send
    /// attempt regardless of the probabilistic `drop_per_thousand` roll.
    /// See [`Self::drop_next_sends`].
    send_drop_next: AtomicU32,
    /// Deterministic, destination-filtered burst-drop: `(addr, remaining,
    /// min_len)`. Only sends to `addr` of at least `min_len` bytes are
    /// affected, decrementing `remaining`; sends to any other address, or
    /// smaller sends to `addr` (e.g. a tickSync keepalive interleaved with
    /// whatever real traffic the test is targeting), pass through
    /// untouched. See [`Self::drop_next_sends_to`].
    send_drop_next_to: Mutex<Option<(SocketAddr, u32, usize)>>,
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
            reorder_buffer: Mutex::new(std::collections::HashMap::new()),
            send_drop_next: AtomicU32::new(0),
            send_drop_next_to: Mutex::new(None),
        }
    }

    /// Arm a deterministic burst drop: the next `n` send attempts **to any
    /// destination** are dropped unconditionally (no RNG roll), independent
    /// of `drop_per_thousand`. Each dropped send decrements the counter by
    /// one. On a transport multiplexing several peers (like the real
    /// `BaseService` socket, one send call per outbound datagram to
    /// whichever client it's addressed to), this hits whatever the very
    /// next N sends happen to be -- fine when nothing else is expected to
    /// be in flight, imprecise otherwise. Prefer
    /// [`Self::drop_next_sends_to`] when the target needs to be a specific
    /// peer.
    pub fn drop_next_sends(&self, n: u32) {
        self.send_drop_next.fetch_add(n, Ordering::SeqCst);
    }

    /// Arm a deterministic, destination-filtered burst drop: the next `n`
    /// send attempts **whose destination is `addr` and whose length is at
    /// least `min_len` bytes** are dropped unconditionally; sends to any
    /// other address, or shorter sends to `addr`, are unaffected. Use this
    /// to guarantee a *specific* packet to a *specific* witness is lost
    /// (e.g. "the next thing ≥50 bytes the server sends to witness A is
    /// B's `CREATE_ENTITY`") without also catching unrelated same-address
    /// traffic that happens to be in flight at the same moment -- on a
    /// live `BaseService`, a connected client keeps receiving small
    /// periodic tickSync/keepalive packets (well under typical AoI-message
    /// sizes) whether or not the scenario the test cares about has fired
    /// yet, so an unfiltered "next N sends to this address" can spend its
    /// budget on a keepalive instead of the packet the test is actually
    /// targeting. `min_len = 0` disables the size filter (matches every
    /// size). Overwrites any previously-armed targeted drop (only one
    /// target at a time).
    pub fn drop_next_sends_to(&self, n: u32, addr: SocketAddr, min_len: usize) {
        *self
            .send_drop_next_to
            .lock()
            .expect("send_drop_next_to poisoned") = Some((addr, n, min_len));
    }

    /// Flush any packets currently held in the send-side reorder buffer
    /// (across **every** destination), each destination's held packets in
    /// reverse arrival order (the same order an auto-flush on a full
    /// per-destination bucket would use). Call this at test teardown so a
    /// partially-full buffer doesn't silently swallow held packets —
    /// mirrors `test_harness::LoopbackPeer::flush_reorder_buffer`.
    pub async fn flush_reorder_buffer(&self) -> io::Result<()> {
        let drained: std::collections::HashMap<SocketAddr, Vec<Vec<u8>>> = {
            let mut buf = self.reorder_buffer.lock().expect("reorder_buffer poisoned");
            std::mem::take(&mut *buf)
        };
        for (addr, mut held) in drained {
            held.reverse();
            for bytes in held {
                self.inner.send_to(&bytes, addr).await?;
            }
        }
        Ok(())
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

    /// `latency + [0, jitter]`, redrawing the jitter component from `rng`
    /// each call. Zero jitter returns exactly `latency` with no RNG draw.
    fn effective_latency(config: &LossyConfig, rng: &mut ChaCha20Rng) -> Duration {
        if config.jitter.is_zero() {
            return config.latency;
        }
        let jitter_nanos = config.jitter.as_nanos().min(u64::MAX as u128) as u64;
        let extra = if jitter_nanos == 0 {
            0
        } else {
            rng.random_range(0..=jitter_nanos)
        };
        config.latency + Duration::from_nanos(extra)
    }
}

#[async_trait]
impl Transport for LossyTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        // Deterministic drops take priority over the probabilistic roll --
        // a test that armed `drop_next_sends`/`drop_next_sends_to` wants a
        // guarantee, not another coin flip. Destination-filtered first:
        // it's the more specific request when both happen to be armed.
        let forced_drop_targeted = {
            let mut targeted = self
                .send_drop_next_to
                .lock()
                .expect("send_drop_next_to poisoned");
            match *targeted {
                Some((target_addr, remaining, min_len))
                    if target_addr == addr && remaining > 0 && bytes.len() >= min_len =>
                {
                    let next = remaining - 1;
                    *targeted = if next > 0 {
                        Some((target_addr, next, min_len))
                    } else {
                        None
                    };
                    true
                }
                _ => false,
            }
        };

        let forced_drop_global = !forced_drop_targeted
            && self
                .send_drop_next
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                    if n > 0 {
                        Some(n - 1)
                    } else {
                        None
                    }
                })
                .is_ok();

        let forced_drop = forced_drop_targeted || forced_drop_global;

        let (drop_this, duplicate, latency) = if forced_drop {
            (true, false, self.send_config.latency)
        } else {
            let mut rng = self.send_rng.lock().expect("send_rng poisoned");
            let drop_this = Self::should_drop(&self.send_config, &mut rng);
            let duplicate = if !drop_this {
                Self::should_duplicate(&self.send_config, &mut rng)
            } else {
                false
            };
            let latency = Self::effective_latency(&self.send_config, &mut rng);
            (drop_this, duplicate, latency)
        };

        if !latency.is_zero() {
            tokio::time::sleep(latency).await;
        }

        if drop_this {
            tracing::debug!(
                target: "mercury.lossy_transport",
                %addr,
                len = bytes.len(),
                forced_targeted = forced_drop_targeted,
                forced_global = forced_drop_global,
                "LossyTransport dropped an outbound send"
            );
            // Mimic a wire-side drop: return success on the byte
            // count (kernel's view) but never put bytes on the wire.
            return Ok(bytes.len());
        }

        if self.send_config.reorder_buffer_size > 0 {
            // Bucketed by destination: only `addr`'s own held packets
            // count toward `addr`'s flush threshold, and only `addr`'s
            // bucket is drained. See the field doc on `reorder_buffer` for
            // why this matters on a multiplexed (multi-client) transport.
            let to_flush: Option<Vec<Vec<u8>>> = {
                let mut buckets = self.reorder_buffer.lock().expect("reorder_buffer poisoned");
                let bucket = buckets.entry(addr).or_default();
                bucket.push(bytes.to_vec());
                if bucket.len() as u32 >= self.send_config.reorder_buffer_size {
                    Some(std::mem::take(bucket))
                } else {
                    None
                }
            };
            if let Some(mut held) = to_flush {
                // Reverse arrival order, matching NetworkPolicy's
                // multi-packet reorder contract.
                held.reverse();
                for held_bytes in &held {
                    self.inner.send_to(held_bytes, addr).await?;
                }
            }
            // The caller's own bytes were queued (and possibly already
            // flushed) above -- report success as if the OS accepted the
            // write, matching the drop-path's "kernel's view" convention.
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
            let (drop_this, latency) = {
                let mut rng = self.recv_rng.lock().expect("recv_rng poisoned");
                let drop_this = Self::should_drop(&self.recv_config, &mut rng);
                let latency = Self::effective_latency(&self.recv_config, &mut rng);
                (drop_this, latency)
            };
            if drop_this {
                continue;
            }
            if !latency.is_zero() {
                tokio::time::sleep(latency).await;
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
                jitter: Duration::ZERO,
                drop_per_thousand: 500, // 50%
                duplicate_per_thousand: 0,
                reorder_buffer_size: 0,
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

    #[test]
    fn with_jitter_and_with_reorder_buffer_are_builders() {
        let cfg = LossyConfig::new(Duration::ZERO, 0, 0, 0)
            .with_jitter(Duration::from_millis(10))
            .with_reorder_buffer(3);
        assert_eq!(cfg.jitter, Duration::from_millis(10));
        assert_eq!(cfg.reorder_buffer_size, 3);
        // Defaults from `new` alone are zero -- no jitter, no reorder.
        let plain = LossyConfig::new(Duration::ZERO, 0, 0, 0);
        assert_eq!(plain.jitter, Duration::ZERO);
        assert_eq!(plain.reorder_buffer_size, 0);
    }

    #[tokio::test]
    async fn drop_next_sends_forces_exact_count_then_resumes() {
        // Zero probabilistic loss so only the deterministic counter can
        // cause a drop.
        let receiver = make_inner(0);
        let recv_addr = receiver.local_addr().unwrap();
        let sender_inner = make_inner(0);
        let lossy = LossyTransport::new_symmetric(sender_inner, LossyConfig::new(Duration::ZERO, 0, 0, 0));

        lossy.drop_next_sends(3);

        for i in 0..3u32 {
            let n = lossy
                .send_to(format!("dropped-{i}").as_bytes(), recv_addr)
                .await
                .unwrap();
            assert!(n > 0, "forced-drop send still reports the kernel's-view byte count");
        }
        // The 4th send must land -- the counter is exhausted.
        lossy.send_to(b"survivor", recv_addr).await.unwrap();

        let mut buf = [0u8; 64];
        let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
            .await
            .expect("the 4th send must be delivered")
            .unwrap();
        assert_eq!(&buf[..len], b"survivor");

        // Nothing else arrives -- the 3 forced drops never hit the wire.
        let timed_out = tokio::time::timeout(Duration::from_millis(100), receiver.recv_from(&mut buf))
            .await
            .is_err();
        assert!(timed_out, "exactly 3 sends must have been dropped, not fewer");
    }

    #[tokio::test]
    async fn drop_next_sends_to_only_affects_the_targeted_destination() {
        // Two distinct receivers -- the targeted drop must hit only the
        // one it names, leaving traffic to the other receiver untouched
        // even when sends to both are interleaved.
        let receiver_a = make_inner(0);
        let addr_a = receiver_a.local_addr().unwrap();
        let receiver_b = make_inner(0);
        let addr_b = receiver_b.local_addr().unwrap();
        let sender_inner = make_inner(0);
        let lossy = LossyTransport::new_symmetric(sender_inner, LossyConfig::new(Duration::ZERO, 0, 0, 0));

        lossy.drop_next_sends_to(1, addr_a, 0);

        // Interleave: to B first (must land), then to A (must drop), then
        // to B again (must land) -- proves the filter doesn't just drop
        // "whatever's next" globally.
        lossy.send_to(b"to-b-1", addr_b).await.unwrap();
        lossy.send_to(b"to-a-dropped", addr_a).await.unwrap();
        lossy.send_to(b"to-b-2", addr_b).await.unwrap();

        let mut buf = [0u8; 64];
        for expected in [b"to-b-1".as_slice(), b"to-b-2".as_slice()] {
            let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver_b.recv_from(&mut buf))
                .await
                .expect("sends to B must be unaffected by the A-targeted drop")
                .unwrap();
            assert_eq!(&buf[..len], expected);
        }
        let timed_out = tokio::time::timeout(Duration::from_millis(100), receiver_a.recv_from(&mut buf))
            .await
            .is_err();
        assert!(timed_out, "the targeted send to A must have been dropped");
    }

    #[tokio::test]
    async fn drop_next_sends_to_min_len_skips_smaller_sends() {
        // A `min_len` filter must let a short "keepalive"-shaped send pass
        // through untouched and instead wait for a send at least that
        // long, still to the same address -- this is what lets a test
        // target "the next *substantial* packet to this witness" without
        // an interleaved small periodic send (tickSync, in the real
        // server) spending the armed drop's budget first.
        let receiver = make_inner(0);
        let recv_addr = receiver.local_addr().unwrap();
        let sender_inner = make_inner(0);
        let lossy = LossyTransport::new_symmetric(sender_inner, LossyConfig::new(Duration::ZERO, 0, 0, 0));

        lossy.drop_next_sends_to(1, recv_addr, 50);

        lossy.send_to(b"short", recv_addr).await.unwrap(); // 5 bytes, under min_len -- must land
        lossy
            .send_to(&vec![b'x'; 64], recv_addr)
            .await
            .unwrap(); // 64 bytes, at/over min_len -- must drop
        lossy.send_to(b"survivor", recv_addr).await.unwrap(); // budget exhausted -- must land

        let mut buf = [0u8; 128];
        let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
            .await
            .expect("the short send must pass the size filter and land")
            .unwrap();
        assert_eq!(&buf[..len], b"short");

        let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
            .await
            .expect("the survivor send must land once the budget is exhausted")
            .unwrap();
        assert_eq!(&buf[..len], b"survivor");

        // The 64-byte send never arrives -- it was the one dropped.
        let timed_out = tokio::time::timeout(Duration::from_millis(100), receiver.recv_from(&mut buf))
            .await
            .is_err();
        assert!(timed_out, "the 64-byte send must have been the one dropped");
    }

    #[tokio::test]
    async fn reorder_buffer_flushes_in_reverse_arrival_order() {
        let receiver = make_inner(0);
        let recv_addr = receiver.local_addr().unwrap();
        let sender_inner = make_inner(0);
        let cfg = LossyConfig::new(Duration::ZERO, 0, 0, 0).with_reorder_buffer(3);
        let lossy = LossyTransport::new_symmetric(sender_inner, cfg);

        // Nothing is on the wire yet -- all 3 are held until the buffer fills.
        lossy.send_to(b"first", recv_addr).await.unwrap();
        lossy.send_to(b"second", recv_addr).await.unwrap();
        let mut buf = [0u8; 64];
        assert!(
            tokio::time::timeout(Duration::from_millis(50), receiver.recv_from(&mut buf))
                .await
                .is_err(),
            "buffer isn't full yet -- nothing should have been sent"
        );

        // The 3rd send fills the buffer and flushes all 3, reversed:
        // arrival order was first, second, third -> wire order third,
        // second, first.
        lossy.send_to(b"third", recv_addr).await.unwrap();

        let mut order = Vec::new();
        for _ in 0..3 {
            let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
                .await
                .expect("held packets must flush once the buffer fills")
                .unwrap();
            order.push(String::from_utf8_lossy(&buf[..len]).to_string());
        }
        assert_eq!(order, vec!["third", "second", "first"]);
    }

    #[tokio::test]
    async fn flush_reorder_buffer_drains_a_partial_buffer() {
        let receiver = make_inner(0);
        let recv_addr = receiver.local_addr().unwrap();
        let sender_inner = make_inner(0);
        // Buffer size 5, but we only ever send 2 -- without an explicit
        // flush these would never reach the wire.
        let cfg = LossyConfig::new(Duration::ZERO, 0, 0, 0).with_reorder_buffer(5);
        let lossy = LossyTransport::new_symmetric(sender_inner, cfg);

        lossy.send_to(b"a", recv_addr).await.unwrap();
        lossy.send_to(b"b", recv_addr).await.unwrap();
        lossy.flush_reorder_buffer().await.unwrap();

        let mut buf = [0u8; 64];
        let mut order = Vec::new();
        for _ in 0..2 {
            let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
                .await
                .expect("explicit flush must deliver the partial buffer")
                .unwrap();
            order.push(String::from_utf8_lossy(&buf[..len]).to_string());
        }
        assert_eq!(order, vec!["b", "a"], "explicit flush also reverses arrival order");
    }

    #[tokio::test]
    async fn jitter_widens_delivery_latency_within_bounds() {
        // Deterministic seed -- pin that the observed latency never
        // exceeds latency + jitter, and that jitter actually moves the
        // needle (not always the minimum).
        let receiver = make_inner(0);
        let recv_addr = receiver.local_addr().unwrap();
        let sender_inner = make_inner(0);
        let base = Duration::from_millis(5);
        let jitter = Duration::from_millis(20);
        let cfg = LossyConfig::new(base, 0, 0, 7).with_jitter(jitter);
        let lossy = LossyTransport::new_symmetric(sender_inner, cfg);

        let mut latencies = Vec::new();
        for i in 0..8u32 {
            let start = std::time::Instant::now();
            lossy
                .send_to(format!("j-{i}").as_bytes(), recv_addr)
                .await
                .unwrap();
            latencies.push(start.elapsed());
        }

        for lat in &latencies {
            assert!(
                *lat >= base,
                "observed latency {lat:?} must be at least the base latency {base:?}"
            );
            assert!(
                // Generous scheduling slack -- this asserts the jitter
                // computation is bounded, not tight real-time delivery;
                // a loaded CI/dev box can add tens of ms of scheduler
                // noise on top of the sleep itself.
                *lat <= base + jitter + Duration::from_millis(200),
                "observed latency {lat:?} must not exceed base+jitter by more than scheduling slack"
            );
        }
        let distinct: std::collections::HashSet<_> = latencies.iter().map(|d| d.as_millis()).collect();
        assert!(
            distinct.len() > 1,
            "jitter must vary the observed latency across sends, got {latencies:?}"
        );
    }
}
