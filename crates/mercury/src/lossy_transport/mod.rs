//! A [`BidirectionalTransport`] wrapper that applies seeded
//! filters to the underlying transport:
//!
//! - **Send side**: drop / latency+jitter (latency delays delivery; it
//!   never blocks the caller) / duplicate / reorder (full
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

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Instant;

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
    /// gets exactly `latency`. Delivery to one destination keeps its
    /// order (each destination has a FIFO delay line); use
    /// `reorder_buffer_size` to reorder.
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
    /// Send-side only: the first `N` sends to each destination skip the
    /// probabilistic drop roll (the roll is still drawn, so the RNG
    /// sequence does not shift). A Mercury connection opens with two
    /// phase-3 handshake datagrams. The server resends a lost one on RTO
    /// (#842), but `cimmeria-wireclient` reads the two positionally and
    /// fails on whichever arrives first, so a wire-client scenario about
    /// loss on an established channel sets this to 2 (#1371).
    /// Deterministic `drop_next_sends*` drops still apply. `0` (the
    /// default via [`Self::new`]) exempts nothing.
    pub lossless_first_sends: u32,
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
            lossless_first_sends: 0,
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

    /// Exempt the first `n` sends to each destination from the
    /// probabilistic drop. See [`Self::lossless_first_sends`]. Returns a
    /// new config.
    pub fn with_lossless_first_sends(mut self, n: u32) -> Self {
        self.lossless_first_sends = n;
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
    recv_rng: Arc<Mutex<ChaCha20Rng>>,
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
    /// One FIFO delay line per destination. A delayed send is queued here
    /// with its due time and `send_to` returns at once; the line's task
    /// puts it on the wire when it is due. Latency is a pipeline delay,
    /// not a cap on how fast the caller can send (#1371).
    send_lines: Mutex<HashMap<SocketAddr, mpsc::UnboundedSender<Delayed>>>,
    /// Receive side of the delayed-recv pump, started on the first
    /// `recv_from` when the recv config has latency or jitter.
    recv_line: tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<RecvItem>>>,
    /// The pump task, aborted when the transport is dropped.
    recv_pump: Mutex<Option<JoinHandle<()>>>,
    /// Sends so far per destination, for `send_config.lossless_first_sends`.
    sends_per_dest: Mutex<HashMap<SocketAddr, u32>>,
}

/// One datagram waiting on a destination's delay line.
#[derive(Debug)]
struct Delayed {
    bytes: Vec<u8>,
    due: Instant,
}

/// What the recv pump hands `recv_from`: a datagram, its source and the
/// time it is due, or an error to return.
type RecvItem = io::Result<(Vec<u8>, SocketAddr, Instant)>;

/// Start the delay line for `addr`: a task that sends each queued datagram
/// once it is due, in queue order. A datagram due before the one ahead of it
/// goes right after it, so a destination never sees its packets reordered.
fn spawn_send_line(
    inner: Arc<dyn BidirectionalTransport>,
    addr: SocketAddr,
) -> mpsc::UnboundedSender<Delayed> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Delayed>();
    tokio::spawn(async move {
        while let Some(delayed) = rx.recv().await {
            tokio::time::sleep_until(delayed.due).await;
            if let Err(e) = inner.send_to(&delayed.bytes, addr).await {
                tracing::debug!(
                    target: "mercury.lossy_transport",
                    %addr,
                    error = %e,
                    "LossyTransport delayed send failed"
                );
            }
        }
    });
    tx
}

/// The error `recv_from` returns after [`MAX_RECV_DROP_LOOPS`] drops in a row.
fn recv_drop_cap_error(drop_per_thousand: u32) -> io::Error {
    io::Error::other(format!(
        "LossyTransport::recv_from dropped {MAX_RECV_DROP_LOOPS} consecutive packets — \
         recv-side drop_per_thousand may be misconfigured at or near 100%; \
         current config: {drop_per_thousand}/{MAX_PER_THOUSAND}",
    ))
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
        let recv_rng = Arc::new(Mutex::new(ChaCha20Rng::seed_from_u64(recv.rng_seed)));
        Self {
            inner,
            send_config: send,
            recv_config: recv,
            send_rng,
            recv_rng,
            reorder_buffer: Mutex::new(std::collections::HashMap::new()),
            send_drop_next: AtomicU32::new(0),
            send_drop_next_to: Mutex::new(None),
            send_lines: Mutex::new(HashMap::new()),
            recv_line: tokio::sync::Mutex::new(None),
            recv_pump: Mutex::new(None),
            sends_per_dest: Mutex::new(HashMap::new()),
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

    /// Put `bytes` on the wire now (`due` is `None`), or queue them on
    /// `addr`'s delay line until `due`. A queued send reports the full
    /// length, the kernel's view, like the drop path.
    async fn emit(
        &self,
        bytes: &[u8],
        addr: SocketAddr,
        due: Option<Instant>,
    ) -> io::Result<usize> {
        let Some(due) = due else {
            return self.inner.send_to(bytes, addr).await;
        };
        let mut lines = self.send_lines.lock().expect("send_lines poisoned");
        let line = lines
            .entry(addr)
            .or_insert_with(|| spawn_send_line(self.inner.clone(), addr));
        line.send(Delayed {
            bytes: bytes.to_vec(),
            due,
        })
        .map_err(|_| io::Error::other("LossyTransport delay line closed"))?;
        Ok(bytes.len())
    }

    /// Start the delayed-recv pump: it reads the inner transport as fast as
    /// datagrams arrive, applies the recv drop roll and stamps each survivor
    /// with its due time, so a slow reader never adds the latency once per
    /// packet. The pump owns the recv RNG from here on.
    fn spawn_recv_pump(&self) -> mpsc::UnboundedReceiver<RecvItem> {
        let (tx, rx) = mpsc::unbounded_channel::<RecvItem>();
        let inner = self.inner.clone();
        let config = self.recv_config.clone();
        let rng = self.recv_rng.clone();
        let pump = tokio::spawn(async move {
            let mut buf = vec![0u8; 65_536];
            let mut dropped_in_a_row = 0u32;
            loop {
                let item = match inner.recv_from(&mut buf).await {
                    Ok((n, addr)) => {
                        let (drop_this, latency) = {
                            let mut rng = rng.lock().expect("recv_rng poisoned");
                            (
                                Self::should_drop(&config, &mut rng),
                                Self::effective_latency(&config, &mut rng),
                            )
                        };
                        if drop_this {
                            dropped_in_a_row += 1;
                            if dropped_in_a_row < MAX_RECV_DROP_LOOPS {
                                continue;
                            }
                            dropped_in_a_row = 0;
                            Err(recv_drop_cap_error(config.drop_per_thousand))
                        } else {
                            dropped_in_a_row = 0;
                            Ok((buf[..n].to_vec(), addr, Instant::now() + latency))
                        }
                    }
                    Err(e) => Err(e),
                };
                if tx.send(item).is_err() {
                    return;
                }
            }
        });
        *self.recv_pump.lock().expect("recv_pump poisoned") = Some(pump);
        rx
    }
}

impl Drop for LossyTransport {
    fn drop(&mut self) {
        if let Some(pump) = self.recv_pump.get_mut().ok().and_then(Option::take) {
            pump.abort();
        }
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

        // Forced drops count toward the grace too: a send is a send.
        let in_grace = self.send_config.lossless_first_sends > 0 && {
            let mut sends = self.sends_per_dest.lock().expect("sends_per_dest poisoned");
            let n = sends.entry(addr).or_insert(0);
            let in_grace = *n < self.send_config.lossless_first_sends;
            *n = n.saturating_add(1);
            in_grace
        };

        let (drop_this, duplicate, latency) = if forced_drop {
            (true, false, self.send_config.latency)
        } else {
            let mut rng = self.send_rng.lock().expect("send_rng poisoned");
            // The grace only overrides the outcome: every roll is drawn
            // exactly as without it, so the RNG sequence after the grace
            // window is unchanged (a spared send draws no duplicate roll).
            let rolled_drop = Self::should_drop(&self.send_config, &mut rng);
            let duplicate = if !rolled_drop {
                Self::should_duplicate(&self.send_config, &mut rng)
            } else {
                false
            };
            let latency = Self::effective_latency(&self.send_config, &mut rng);
            (rolled_drop && !in_grace, duplicate, latency)
        };

        // Never sleep here: the caller may be a loop that serves every
        // client (the base's cell-message dispatcher is one task), and a
        // sleep per send turns 80 ms of latency into a cap of about 12
        // sends a second for the whole server (#1371). The delay line
        // delivers the datagram when it is due instead. Any configured
        // delay goes through the line, even a zero draw, so a packet never
        // overtakes one still queued for the same destination.
        let delayed = !self.send_config.latency.is_zero() || !self.send_config.jitter.is_zero();
        let due = delayed.then(|| Instant::now() + latency);

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
                    self.emit(held_bytes, addr, due).await?;
                }
            }
            // The caller's own bytes were queued (and possibly already
            // flushed) above -- report success as if the OS accepted the
            // write, matching the drop-path's "kernel's view" convention.
            return Ok(bytes.len());
        }

        let n = self.emit(bytes, addr, due).await?;
        if duplicate {
            // Best-effort dup; ignore the secondary I/O result so a
            // duplicate failure doesn't propagate as the primary
            // send's outcome.
            let _ = self.emit(bytes, addr, due).await;
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
        // With latency or jitter, a pump reads ahead and `recv_from` only
        // waits for each datagram's due time. Sleeping here after every
        // read would cap the server's whole receive loop at one datagram
        // per latency (#1371).
        if !self.recv_config.latency.is_zero() || !self.recv_config.jitter.is_zero() {
            let mut line = self.recv_line.lock().await;
            if line.is_none() {
                *line = Some(self.spawn_recv_pump());
            }
            let Some(rx) = line.as_mut() else {
                return Err(io::Error::other("LossyTransport recv pump missing"));
            };
            return match rx.recv().await {
                Some(Ok((bytes, addr, due))) => {
                    tokio::time::sleep_until(due).await;
                    let n = bytes.len().min(buf.len());
                    buf[..n].copy_from_slice(&bytes[..n]);
                    Ok((n, addr))
                }
                Some(Err(e)) => Err(e),
                None => Err(io::Error::other("LossyTransport recv pump stopped")),
            };
        }
        for _ in 0..MAX_RECV_DROP_LOOPS {
            let (n, addr) = self.inner.recv_from(buf).await?;
            let drop_this = {
                let mut rng = self.recv_rng.lock().expect("recv_rng poisoned");
                Self::should_drop(&self.recv_config, &mut rng)
            };
            if drop_this {
                continue;
            }
            return Ok((n, addr));
        }
        Err(recv_drop_cap_error(self.recv_config.drop_per_thousand))
    }
}

#[cfg(test)]
mod tests;
