//! The seeded filter logic shared by [`super::LossyTransport`] (one config
//! for every peer, fixed at construction) and the lab-chaos net shim in
//! `cimmeria-base` (one config per peer, switched at runtime). Both roll a
//! send's fate with the same functions in the same draw order, and both put
//! the survivors on the wire through a [`SendShaper`], so a seed means the
//! same thing on the send side of either. (The receive streams differ:
//! [`FlowFilter`] draws them from the seed mixed with a constant.)

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

use super::LossyConfig;
use crate::transport::BidirectionalTransport;

/// Mixed into a [`FlowFilter`]'s seed for its receive stream, so the two
/// directions draw independent sequences and the send pattern of a seed
/// does not depend on how much the peer sent.
const RECV_STREAM: u64 = 0x9E37_79B9_7F4A_7C15;

/// The most datagrams one destination's delay line holds. A send past it is
/// shed (a wire-side drop, counted by [`SendShaper::shed_total`]): at 5 s of
/// latency a busy destination would otherwise queue without bound.
pub const MAX_DELAYED_PER_DEST: usize = 4096;

/// One send's fate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendRoll {
    /// The send never reaches the wire.
    pub drop: bool,
    /// The send goes out twice. Never set on a dropped send.
    pub duplicate: bool,
    /// `latency + [0, jitter]`; meaningful only when the config delays.
    pub latency: Duration,
}

/// `true` with probability `drop_per_thousand / 1000`. A zero rate draws
/// nothing, so a lossless config leaves the RNG untouched.
pub fn roll_drop(config: &LossyConfig, rng: &mut ChaCha20Rng) -> bool {
    if config.drop_per_thousand == 0 {
        return false;
    }
    rng.random_range(0..1000u32) < config.drop_per_thousand
}

fn roll_duplicate(config: &LossyConfig, rng: &mut ChaCha20Rng) -> bool {
    if config.duplicate_per_thousand == 0 {
        return false;
    }
    rng.random_range(0..1000u32) < config.duplicate_per_thousand
}

/// `latency + [0, jitter]`, redrawing the jitter component from `rng`
/// each call. Zero jitter returns exactly `latency` with no RNG draw.
pub fn roll_latency(config: &LossyConfig, rng: &mut ChaCha20Rng) -> Duration {
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

/// Rolls drop, then duplicate (only for a send that survived the drop roll),
/// then latency. The order is part of the replay contract: changing it
/// changes what every recorded seed does.
pub fn roll_send(config: &LossyConfig, rng: &mut ChaCha20Rng) -> SendRoll {
    let drop = roll_drop(config, rng);
    let duplicate = !drop && roll_duplicate(config, rng);
    let latency = roll_latency(config, rng);
    SendRoll {
        drop,
        duplicate,
        latency,
    }
}

/// One flow's config and the two seeded RNG streams it rolls with: sends
/// from `rng_seed`, receives from `rng_seed` mixed with a fixed constant.
#[derive(Debug)]
pub struct FlowFilter {
    config: LossyConfig,
    send_rng: ChaCha20Rng,
    recv_rng: ChaCha20Rng,
}

impl FlowFilter {
    /// Seeds both streams from `config.rng_seed`, so two filters built from
    /// one config roll the same sequence.
    pub fn new(config: LossyConfig) -> Self {
        let send_rng = ChaCha20Rng::seed_from_u64(config.rng_seed);
        let recv_rng = ChaCha20Rng::seed_from_u64(config.rng_seed ^ RECV_STREAM);
        Self {
            config,
            send_rng,
            recv_rng,
        }
    }

    /// The config the filter was built from, which [`SendShaper::deliver`]
    /// needs for the latency and reorder settings.
    pub fn config(&self) -> &LossyConfig {
        &self.config
    }

    /// The next send's fate, from the send stream ([`roll_send`]).
    pub fn roll_send(&mut self) -> SendRoll {
        roll_send(&self.config, &mut self.send_rng)
    }

    /// The receive side honours drop only (see [`super::LossyTransport`]'s
    /// module doc for why).
    pub fn roll_recv_drop(&mut self) -> bool {
        roll_drop(&self.config, &mut self.recv_rng)
    }
}

/// One datagram waiting on a destination's delay line.
#[derive(Debug)]
struct Delayed {
    bytes: Vec<u8>,
    due: Instant,
}

/// A destination's delay line. Dropping it cancels the line: the task sends
/// nothing more, and what is still queued is discarded.
struct SendLine {
    tx: mpsc::Sender<Delayed>,
    _cancel: oneshot::Sender<()>,
}

/// Start the delay line for `addr`: a task that sends each queued datagram
/// once it is due, in queue order. A datagram due before the one ahead of it
/// goes right after it, so a destination never sees its packets reordered.
fn spawn_send_line(inner: Arc<dyn BidirectionalTransport>, addr: SocketAddr) -> SendLine {
    let (tx, mut rx) = mpsc::channel::<Delayed>(MAX_DELAYED_PER_DEST);
    let (cancel, mut cancelled) = oneshot::channel::<()>();
    tokio::spawn(async move {
        while let Some(delayed) = rx.recv().await {
            // The cancel side is never sent on: it resolves (closed) once the
            // line is dropped. `biased` checks it first, so a cancelled line
            // drains its queue without sending any of it.
            tokio::select! {
                biased;
                _ = &mut cancelled => return,
                () = tokio::time::sleep_until(delayed.due) => {}
            }
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
    SendLine {
        tx,
        _cancel: cancel,
    }
}

/// A destination's shaping state: its reorder bucket and delay line, and
/// the generation they belong to.
struct Flow {
    generation: u64,
    held: Vec<Vec<u8>>,
    line: Option<SendLine>,
}

impl Flow {
    fn new(generation: u64) -> Self {
        Self {
            generation,
            held: Vec::new(),
            line: None,
        }
    }
}

/// `addr`'s flow for `generation`, built if missing. An older flow is
/// discarded first; a newer one means the send is stale (`None`).
fn flow_for(
    flows: &mut HashMap<SocketAddr, Flow>,
    addr: SocketAddr,
    generation: u64,
) -> Option<&mut Flow> {
    let flow = flows.entry(addr).or_insert_with(|| Flow::new(generation));
    if flow.generation > generation {
        return None;
    }
    if flow.generation < generation {
        *flow = Flow::new(generation);
    }
    Some(flow)
}

/// Puts the sends that survived their roll on the wire: through a
/// per-destination delay line when the config delays, and through a
/// per-destination reorder bucket when it reorders. Never sleeps in the
/// caller: the base's cell-message dispatcher is one task serving every
/// client, and a sleep per send turns 80 ms of latency into a cap of about
/// 12 sends a second for the whole server (#1371).
pub struct SendShaper {
    inner: Arc<dyn BidirectionalTransport>,
    /// Per destination: its held sends and its delay line. Each destination
    /// flushes (reversed) once its own bucket is full: one shared buffer
    /// would hold one client's handshake reply behind another client's
    /// traffic, which is not what a real wire does. Latency is a pipeline
    /// delay, not a cap on how fast the caller can send (#1371).
    flows: Mutex<HashMap<SocketAddr, Flow>>,
    /// Sends shed because a delay line was full.
    shed: AtomicU64,
}

impl SendShaper {
    /// A shaper with no held sends, writing to `inner`.
    pub fn new(inner: Arc<dyn BidirectionalTransport>) -> Self {
        Self {
            inner,
            flows: Mutex::new(HashMap::new()),
            shed: AtomicU64::new(0),
        }
    }

    /// Delivers a send whose roll did not drop it. Reports the full length,
    /// the kernel's view, whether the bytes went out now, were queued on the
    /// delay line, were shed by a full one or were held for reordering. A
    /// held send is not duplicated.
    pub async fn deliver(
        &self,
        bytes: &[u8],
        addr: SocketAddr,
        config: &LossyConfig,
        duplicate: bool,
        latency: Duration,
    ) -> io::Result<usize> {
        self.deliver_as(bytes, addr, config, duplicate, latency, 0)
            .await
    }

    /// [`Self::deliver`] for a destination whose shaping is replaced over
    /// time. `generation` tags the destination's held sends and delay line:
    /// a higher one discards what an older generation left there, so a new
    /// run starts clean, and a lower one is stale, so its send is discarded
    /// (and reported sent).
    pub async fn deliver_as(
        &self,
        bytes: &[u8],
        addr: SocketAddr,
        config: &LossyConfig,
        duplicate: bool,
        latency: Duration,
        generation: u64,
    ) -> io::Result<usize> {
        if !self.admit(addr, generation) {
            return Ok(bytes.len());
        }
        // Any configured delay goes through the line, even a zero draw, so
        // a packet never overtakes one still queued for the same destination.
        let delayed = !config.latency.is_zero() || !config.jitter.is_zero();
        let due = delayed.then(|| Instant::now() + latency);

        if config.reorder_buffer_size > 0 {
            let to_flush = {
                let mut flows = self.flows();
                let Some(flow) = flow_for(&mut flows, addr, generation) else {
                    return Ok(bytes.len());
                };
                flow.held.push(bytes.to_vec());
                (flow.held.len() as u32 >= config.reorder_buffer_size)
                    .then(|| std::mem::take(&mut flow.held))
            };
            if let Some(mut held) = to_flush {
                // Reverse arrival order, matching NetworkPolicy's
                // multi-packet reorder contract.
                held.reverse();
                for held_bytes in &held {
                    self.emit(held_bytes, addr, due, generation).await?;
                }
            }
            return Ok(bytes.len());
        }

        let n = self.emit(bytes, addr, due, generation).await?;
        if duplicate {
            // Best-effort: a failed duplicate is not the primary send's
            // outcome.
            let _ = self.emit(bytes, addr, due, generation).await;
        }
        Ok(n)
    }

    /// Flushes every destination's held sends, each in reverse arrival
    /// order (the order a full bucket would use).
    pub async fn flush_reorder_buffer(&self) -> io::Result<()> {
        let drained: Vec<(SocketAddr, Vec<Vec<u8>>)> = self
            .flows()
            .iter_mut()
            .filter(|(_, flow)| !flow.held.is_empty())
            .map(|(addr, flow)| (*addr, std::mem::take(&mut flow.held)))
            .collect();
        for (addr, mut held) in drained {
            held.reverse();
            for bytes in held {
                self.inner.send_to(&bytes, addr).await?;
            }
        }
        Ok(())
    }

    /// Ends shaping for `addr` up to `generation`: its held sends and
    /// whatever its delay line still queues are discarded, never sent. A
    /// newer generation's state is left alone.
    pub fn discard(&self, addr: SocketAddr, generation: u64) {
        let mut flows = self.flows();
        if flows
            .get(&addr)
            .is_some_and(|flow| flow.generation <= generation)
        {
            flows.remove(&addr);
        }
    }

    /// Whether `addr` has held sends or a delay line.
    pub fn tracks(&self, addr: SocketAddr) -> bool {
        self.flows().contains_key(&addr)
    }

    /// How many sends a full delay line has shed since the shaper was built.
    pub fn shed_total(&self) -> u64 {
        self.shed.load(Ordering::Relaxed)
    }

    fn flows(&self) -> MutexGuard<'_, HashMap<SocketAddr, Flow>> {
        self.flows.lock().expect("shaper flows poisoned")
    }

    /// False for a stale `generation`; an older flow at `addr` is discarded.
    /// Builds nothing, so an unshaped send leaves no state behind.
    fn admit(&self, addr: SocketAddr, generation: u64) -> bool {
        let mut flows = self.flows();
        match flows.get(&addr).map(|flow| flow.generation) {
            Some(current) if current > generation => false,
            Some(current) if current < generation => {
                flows.remove(&addr);
                true
            }
            _ => true,
        }
    }

    /// Put `bytes` on the wire now (`due` is `None`), or queue them on
    /// `addr`'s delay line until `due`.
    async fn emit(
        &self,
        bytes: &[u8],
        addr: SocketAddr,
        due: Option<Instant>,
        generation: u64,
    ) -> io::Result<usize> {
        let Some(due) = due else {
            return self.inner.send_to(bytes, addr).await;
        };
        let mut flows = self.flows();
        let Some(flow) = flow_for(&mut flows, addr, generation) else {
            return Ok(bytes.len());
        };
        let line = flow
            .line
            .get_or_insert_with(|| spawn_send_line(self.inner.clone(), addr));
        let delayed = Delayed {
            bytes: bytes.to_vec(),
            due,
        };
        match line.tx.try_send(delayed) {
            Ok(()) => Ok(bytes.len()),
            Err(TrySendError::Full(_)) => {
                self.shed.fetch_add(1, Ordering::Relaxed);
                Ok(bytes.len())
            }
            Err(TrySendError::Closed(_)) => {
                Err(io::Error::other("LossyTransport delay line closed"))
            }
        }
    }
}
