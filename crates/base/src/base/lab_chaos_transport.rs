//! The lab chaos `net` profile (CH-06, `docs/analysis/lab-chaos/`): a
//! [`BidirectionalTransport`] over the base's real UDP socket that shapes
//! only the sessions the lab registry names, by client address.
//!
//! - **Send side** (server to client): drop, latency, jitter, duplicate and
//!   reorder, rolled and delivered by the same code as `LossyTransport`
//!   (`cimmeria_mercury::lossy_transport::filter`), so a seed means the same
//!   thing in both. Latency is a pipeline delay on a per-client delay line;
//!   `send_to` never sleeps.
//! - **Receive side** (client to server): drop only. One task reads the
//!   socket for every session, so a sleep there would delay every client.
//!   The profile's latency is added once to the round trip, on the server's
//!   sends, not twice.
//!
//! Each shaped address gets its own [`FlowFilter`], seeded from the
//! registry entry's seed (sends and receives on separate streams), so a run
//! replays. The filter is rebuilt when the entry's knobs change, and dropped
//! (its held sends released) once the entry is cleared or expires. A re-set
//! with identical knobs that the shim never saw cleared keeps the running
//! RNG streams.
//!
//! Cost when nothing is set: [`LabChaos::has_net`] (one atomic load) and the
//! shim's own peer count (another) per datagram; the registry lock is taken
//! only while some session has net chaos.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use cimmeria_base_session::base::lab_chaos::LabChaos;
use cimmeria_base_session::base::lab_chaos::NetChaos;
use cimmeria_mercury::lossy_transport::filter::{FlowFilter, SendShaper};
use cimmeria_mercury::lossy_transport::LossyConfig;
use cimmeria_mercury::transport::{BidirectionalTransport, Transport};

/// Wraps the base's transport in the shim when the lab registry is installed
/// (a `lab-chaos` build started with `DEVELOPER_MODE`); otherwise returns it
/// unchanged.
pub(crate) fn wrap_if_installed(
    inner: Arc<dyn BidirectionalTransport>,
) -> Arc<dyn BidirectionalTransport> {
    match cimmeria_base_session::base::lab_chaos::current() {
        Some(registry) => {
            tracing::warn!(
                target: "lab.chaos",
                event = "lab_chaos_net_shim",
                "base transport runs through the lab chaos net shim"
            );
            Arc::new(ChaosTransport::new(inner, registry))
        }
        None => inner,
    }
}

/// One shaped address: the knobs its filter was built from, and the filter.
#[derive(Debug)]
struct Peer {
    net: NetChaos,
    filter: FlowFilter,
}

impl Peer {
    fn new(net: NetChaos) -> Self {
        let filter = FlowFilter::new(lossy_config(&net));
        Self { net, filter }
    }
}

/// The registry's integer knobs as a `LossyConfig`. The registry has already
/// bounded every value, so nothing is clamped here.
fn lossy_config(net: &NetChaos) -> LossyConfig {
    LossyConfig::new(
        Duration::from_millis(u64::from(net.latency_ms)),
        net.loss_per_thousand,
        net.dup_per_thousand,
        net.seed,
    )
    .with_jitter(Duration::from_millis(u64::from(net.jitter_ms)))
    .with_reorder_buffer(net.reorder)
}

pub(crate) struct ChaosTransport {
    inner: Arc<dyn BidirectionalTransport>,
    registry: &'static LabChaos,
    shaper: SendShaper,
    peers: Mutex<HashMap<SocketAddr, Peer>>,
    /// `peers.len()`, written under the `peers` lock, so the fast path can
    /// see there is shaping state left to release without taking it.
    peer_count: AtomicUsize,
}

impl ChaosTransport {
    pub(crate) fn new(inner: Arc<dyn BidirectionalTransport>, registry: &'static LabChaos) -> Self {
        Self {
            shaper: SendShaper::new(inner.clone()),
            inner,
            registry,
            peers: Mutex::new(HashMap::new()),
            peer_count: AtomicUsize::new(0),
        }
    }

    /// A panic under the lock leaves at most one peer half-rolled, which only
    /// shifts that peer's RNG; refusing the lock would wedge every send.
    fn peers(&self) -> MutexGuard<'_, HashMap<SocketAddr, Peer>> {
        self.peers.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The registry's net chaos for `addr`, or `None` without touching the
    /// registry lock when no session has any.
    fn net_for(&self, addr: SocketAddr) -> Option<NetChaos> {
        if !self.registry.has_net() {
            return None;
        }
        self.registry.net_for(addr, Instant::now())
    }

    /// Runs `roll` on `addr`'s filter, building it first, or rebuilding it
    /// when the knobs changed. Returns the roll, a copy of the filter's
    /// config and whether an older filter was replaced (its held sends must
    /// then be released).
    fn roll<R>(
        &self,
        addr: SocketAddr,
        net: NetChaos,
        roll: impl FnOnce(&mut FlowFilter) -> R,
    ) -> (R, LossyConfig, bool) {
        let mut peers = self.peers();
        let mut replaced = false;
        let peer = match peers.entry(addr) {
            Entry::Occupied(entry) => {
                let peer = entry.into_mut();
                if peer.net != net {
                    replaced = true;
                    *peer = Peer::new(net);
                }
                peer
            }
            Entry::Vacant(entry) => {
                tracing::debug!(
                    target: "lab.chaos",
                    event = "lab_chaos_net_shaping",
                    seed = net.seed,
                    "lab chaos net shaping started on a session"
                );
                entry.insert(Peer::new(net))
            }
        };
        let out = roll(&mut peer.filter);
        let config = peer.filter.config().clone();
        self.peer_count.store(peers.len(), Ordering::Release);
        (out, config, replaced)
    }

    /// Stops shaping `addr` (its chaos was cleared or expired): its held
    /// sends go out, and its delay line closes once drained.
    async fn forget(&self, addr: SocketAddr) {
        if self.peer_count.load(Ordering::Acquire) == 0 {
            return;
        }
        let removed = {
            let mut peers = self.peers();
            let removed = peers.remove(&addr).is_some();
            self.peer_count.store(peers.len(), Ordering::Release);
            removed
        };
        if removed {
            self.release(addr).await;
        }
    }

    /// Stops shaping every address (no session has net chaos any more).
    async fn forget_all(&self) {
        let addrs: Vec<SocketAddr> = {
            let mut peers = self.peers();
            let addrs = peers.drain().map(|(addr, _)| addr).collect();
            self.peer_count.store(0, Ordering::Release);
            addrs
        };
        for addr in addrs {
            self.release(addr).await;
        }
    }

    async fn release(&self, addr: SocketAddr) {
        tracing::debug!(
            target: "lab.chaos",
            event = "lab_chaos_net_shaping",
            "lab chaos net shaping ended on a session"
        );
        // Best effort: the held sends are chaos traffic, and a socket error
        // here would show again on the caller's own send.
        if let Err(e) = self.shaper.release(addr).await {
            tracing::debug!(
                target: "lab.chaos",
                error = %e,
                "lab chaos net shim could not release held sends"
            );
        }
    }
}

#[async_trait]
impl Transport for ChaosTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        let Some(net) = self.net_for(addr) else {
            if self.registry.has_net() {
                self.forget(addr).await;
            } else if self.peer_count.load(Ordering::Acquire) > 0 {
                self.forget_all().await;
            }
            return self.inner.send_to(bytes, addr).await;
        };
        let (roll, config, replaced) = self.roll(addr, net, FlowFilter::roll_send);
        if replaced {
            self.release(addr).await;
        }
        if roll.drop {
            tracing::trace!(
                target: "lab.chaos",
                len = bytes.len(),
                "lab chaos dropped an outbound datagram"
            );
            // A wire-side drop: the kernel would have accepted the write.
            return Ok(bytes.len());
        }
        self.shaper
            .deliver(bytes, addr, &config, roll.duplicate, roll.latency)
            .await
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

#[async_trait]
impl BidirectionalTransport for ChaosTransport {
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        // No cap on consecutive drops (unlike `LossyTransport`): every turn
        // awaits a real datagram, so 100% loss on one session never spins,
        // and an error here would reach the receive loop for every session.
        loop {
            let (n, src) = self.inner.recv_from(buf).await?;
            let Some(net) = self.net_for(src) else {
                return Ok((n, src));
            };
            if net.loss_per_thousand == 0 {
                return Ok((n, src));
            }
            let (dropped, _, replaced) = self.roll(src, net, FlowFilter::roll_recv_drop);
            if replaced {
                self.release(src).await;
            }
            if !dropped {
                return Ok((n, src));
            }
            tracing::trace!(
                target: "lab.chaos",
                len = n,
                "lab chaos dropped an inbound datagram"
            );
        }
    }
}

#[cfg(test)]
#[path = "lab_chaos_transport_tests.rs"]
mod tests;
