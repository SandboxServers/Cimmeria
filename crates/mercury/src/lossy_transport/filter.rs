//! The seeded filter logic shared by [`super::LossyTransport`] (one config
//! for every peer, fixed at construction) and the lab-chaos net shim in
//! `cimmeria-base` (one config per peer, switched at runtime). Both roll a
//! send's fate with the same functions in the same draw order, and both put
//! the survivors on the wire through a [`SendShaper`], so a seed means the
//! same thing in either.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::LossyConfig;
use crate::transport::BidirectionalTransport;

/// Mixed into a [`FlowFilter`]'s seed for its receive stream, so the two
/// directions draw independent sequences and the send pattern of a seed
/// does not depend on how much the peer sent.
const RECV_STREAM: u64 = 0x9E37_79B9_7F4A_7C15;

/// One send's fate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendRoll {
    pub drop: bool,
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
    pub fn new(config: LossyConfig) -> Self {
        let send_rng = ChaCha20Rng::seed_from_u64(config.rng_seed);
        let recv_rng = ChaCha20Rng::seed_from_u64(config.rng_seed ^ RECV_STREAM);
        Self {
            config,
            send_rng,
            recv_rng,
        }
    }

    pub fn config(&self) -> &LossyConfig {
        &self.config
    }

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

/// Start the delay line for `addr`: a task that sends each queued datagram
/// once it is due, in queue order. A datagram due before the one ahead of it
/// goes right after it, so a destination never sees its packets reordered.
/// The task ends once its sender is dropped and the queue is drained.
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

/// Puts the sends that survived their roll on the wire: through a
/// per-destination delay line when the config delays, and through a
/// per-destination reorder bucket when it reorders. Never sleeps in the
/// caller: the base's cell-message dispatcher is one task serving every
/// client, and a sleep per send turns 80 ms of latency into a cap of about
/// 12 sends a second for the whole server (#1371).
pub struct SendShaper {
    inner: Arc<dyn BidirectionalTransport>,
    /// Held sends per destination. Each destination flushes (reversed) once
    /// its own bucket is full: one shared buffer would hold one client's
    /// handshake reply behind another client's traffic, which is not what a
    /// real wire does (each flow reorders independently).
    reorder_buffer: Mutex<HashMap<SocketAddr, Vec<Vec<u8>>>>,
    /// One FIFO delay line per destination. Latency is a pipeline delay,
    /// not a cap on how fast the caller can send (#1371).
    send_lines: Mutex<HashMap<SocketAddr, mpsc::UnboundedSender<Delayed>>>,
}

impl SendShaper {
    pub fn new(inner: Arc<dyn BidirectionalTransport>) -> Self {
        Self {
            inner,
            reorder_buffer: Mutex::new(HashMap::new()),
            send_lines: Mutex::new(HashMap::new()),
        }
    }

    /// Delivers a send whose roll did not drop it. Reports the full length,
    /// the kernel's view, whether the bytes went out now, were queued on the
    /// delay line or were held for reordering. A held send is not
    /// duplicated.
    pub async fn deliver(
        &self,
        bytes: &[u8],
        addr: SocketAddr,
        config: &LossyConfig,
        duplicate: bool,
        latency: Duration,
    ) -> io::Result<usize> {
        // Any configured delay goes through the line, even a zero draw, so
        // a packet never overtakes one still queued for the same destination.
        let delayed = !config.latency.is_zero() || !config.jitter.is_zero();
        let due = delayed.then(|| Instant::now() + latency);

        if config.reorder_buffer_size > 0 {
            let to_flush: Option<Vec<Vec<u8>>> = {
                let mut buckets = self.reorder_buffer.lock().expect("reorder_buffer poisoned");
                let bucket = buckets.entry(addr).or_default();
                bucket.push(bytes.to_vec());
                if bucket.len() as u32 >= config.reorder_buffer_size {
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
            return Ok(bytes.len());
        }

        let n = self.emit(bytes, addr, due).await?;
        if duplicate {
            // Best-effort: a failed duplicate is not the primary send's
            // outcome.
            let _ = self.emit(bytes, addr, due).await;
        }
        Ok(n)
    }

    /// Flushes every destination's held sends, each in reverse arrival
    /// order (the order a full bucket would use).
    pub async fn flush_reorder_buffer(&self) -> io::Result<()> {
        let drained: HashMap<SocketAddr, Vec<Vec<u8>>> = {
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

    /// Ends shaping for `addr`: its held sends go out now (reversed), and its
    /// delay line is closed once what is already queued has been sent.
    pub async fn release(&self, addr: SocketAddr) -> io::Result<()> {
        let held = self
            .reorder_buffer
            .lock()
            .expect("reorder_buffer poisoned")
            .remove(&addr);
        self.send_lines
            .lock()
            .expect("send_lines poisoned")
            .remove(&addr);
        if let Some(mut held) = held {
            held.reverse();
            for bytes in held {
                self.inner.send_to(&bytes, addr).await?;
            }
        }
        Ok(())
    }

    /// Put `bytes` on the wire now (`due` is `None`), or queue them on
    /// `addr`'s delay line until `due`.
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
}
