//! The lab chaos `net` profile (CH-06, `docs/analysis/lab-chaos/`): a
//! [`BidirectionalTransport`] over the base's real UDP socket that shapes
//! only the sessions the lab registry names, by client address.
//!
//! - **Send side** (server to client): drop, latency, jitter, duplicate and
//!   reorder, rolled and delivered by the same code as `LossyTransport`
//!   (`cimmeria_mercury::lossy_transport::filter`), so a seed means the same
//!   send-side pattern in both. Latency is a pipeline delay on a per-client
//!   delay line, capped at `MAX_DELAYED_PER_DEST` queued sends (past it a
//!   send is shed, with a rate-limited WARN); `send_to` never sleeps.
//! - **Receive side** (client to server): drop only. One task reads the
//!   socket for every session, so a sleep there would delay every client.
//!   The profile's latency is added once to the round trip, on the server's
//!   sends, not twice. The receive stream is drawn from the seed mixed with
//!   a constant, so it is not `LossyTransport`'s.
//!
//! Each shaped address gets its own [`FlowFilter`], seeded from the
//! registry entry's seed and tagged with the entry's generation, so a run
//! replays: every set, identical knobs included, gets a new generation and
//! a fresh filter. Once an entry is cleared, expires or is set again, the
//! older generation's held reorder sends and queued delayed sends are
//! discarded, never sent: the address may already belong to a new session.
//! Every send while some session has net chaos sweeps the shaped addresses,
//! so a dead one never keeps its state waiting for its own next datagram.
//!
//! Cost when nothing is set: [`LabChaos::has_net`] (one atomic load) and the
//! shim's own peer count (another) per datagram; the registry lock is taken
//! only while some session has net chaos.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use cimmeria_base_session::base::lab_chaos::LabChaos;
use cimmeria_base_session::base::lab_chaos::NetChaos;
use cimmeria_mercury::lossy_transport::filter::{FlowFilter, SendShaper};
use cimmeria_mercury::lossy_transport::LossyConfig;
use cimmeria_mercury::transport::{BidirectionalTransport, Transport};

/// The least time between two WARNs that a delay line shed sends.
const SHED_WARN_EVERY: Duration = Duration::from_secs(10);

/// Wraps the base's transport in the shim when the lab registry is installed
/// (a `lab-chaos` build started with `DEVELOPER_MODE`); otherwise returns it
/// unchanged. The flag comes from the same registry read that chooses the
/// transport, so the proof observer can suppress simulated send receipts.
pub(crate) fn wrap_if_installed(
    inner: Arc<dyn BidirectionalTransport>,
) -> (Arc<dyn BidirectionalTransport>, bool) {
    wrap_with_registry(inner, cimmeria_base_session::base::lab_chaos::current())
}

fn wrap_with_registry(
    inner: Arc<dyn BidirectionalTransport>,
    registry: Option<&'static LabChaos>,
) -> (Arc<dyn BidirectionalTransport>, bool) {
    match registry {
        Some(registry) => {
            tracing::warn!(
                target: "lab.chaos",
                event = "lab_chaos_net_shim",
                "base transport runs through the lab chaos net shim"
            );
            (Arc::new(ChaosTransport::new(inner, registry)), true)
        }
        None => (inner, false),
    }
}

/// One shaped address: the registry generation its filter was built for,
/// and the filter.
#[derive(Debug)]
struct Peer {
    generation: u64,
    filter: FlowFilter,
}

impl Peer {
    fn new(net: &NetChaos, generation: u64) -> Self {
        let filter = FlowFilter::new(lossy_config(net));
        Self { generation, filter }
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

/// The base's transport under the lab chaos `net` profile: shapes the
/// datagrams to and from each address the registry names and passes every
/// other one through untouched (see the module doc).
pub(crate) struct ChaosTransport {
    inner: Arc<dyn BidirectionalTransport>,
    registry: &'static LabChaos,
    shaper: SendShaper,
    peers: Mutex<HashMap<SocketAddr, Peer>>,
    /// `peers.len()`, written under the `peers` lock, so the fast path can
    /// see there is shaping state left to discard without taking it.
    peer_count: AtomicUsize,
    /// The shaper's shed total as of the last WARN, and when that went out.
    shed_reported: AtomicU64,
    shed_warned_at: Mutex<Option<Instant>>,
}

impl ChaosTransport {
    pub(crate) fn new(inner: Arc<dyn BidirectionalTransport>, registry: &'static LabChaos) -> Self {
        Self {
            shaper: SendShaper::new(inner.clone()),
            inner,
            registry,
            peers: Mutex::new(HashMap::new()),
            peer_count: AtomicUsize::new(0),
            shed_reported: AtomicU64::new(0),
            shed_warned_at: Mutex::new(None),
        }
    }

    /// A panic under the lock leaves at most one peer half-rolled, which only
    /// shifts that peer's RNG; refusing the lock would wedge every send.
    fn peers(&self) -> MutexGuard<'_, HashMap<SocketAddr, Peer>> {
        self.peers.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The registry's net chaos for `addr` and its generation, or `None`
    /// without touching the registry lock when no session has any.
    fn entry_for(&self, addr: SocketAddr) -> Option<(NetChaos, u64)> {
        if !self.registry.has_net() {
            return None;
        }
        self.registry.net_entry_for(addr, Instant::now())
    }

    fn is_current(&self, addr: SocketAddr, generation: u64) -> bool {
        self.entry_for(addr).is_some_and(|(_, g)| g == generation)
    }

    /// Runs `roll` on `addr`'s filter for the entry of `generation`, building
    /// it first, or rebuilding it for a newer generation (the older one's
    /// held and queued sends are discarded). Returns the roll and a copy of
    /// the filter's config, or `None` for a stale send: a newer generation
    /// already shapes `addr`, or the entry is gone.
    fn roll<R>(
        &self,
        addr: SocketAddr,
        net: &NetChaos,
        generation: u64,
        roll: impl FnOnce(&mut FlowFilter) -> R,
    ) -> Option<(R, LossyConfig)> {
        let mut peers = self.peers();
        let peer = match peers.entry(addr) {
            Entry::Occupied(entry) => {
                let peer = entry.into_mut();
                if peer.generation > generation {
                    return None;
                }
                if peer.generation < generation {
                    self.shaper.discard(addr, peer.generation);
                    *peer = Peer::new(net, generation);
                }
                peer
            }
            Entry::Vacant(entry) => {
                // Checked under the lock `sweep` and `forget` take, so a send
                // that read the entry just before a teardown cannot rebuild
                // the peer after the teardown's sweep removed it.
                if !self.is_current(addr, generation) {
                    return None;
                }
                tracing::debug!(
                    target: "lab.chaos",
                    event = "lab_chaos_net_shaping",
                    seed = net.seed,
                    generation,
                    "lab chaos net shaping started on a session"
                );
                entry.insert(Peer::new(net, generation))
            }
        };
        let out = roll(&mut peer.filter);
        let config = peer.filter.config().clone();
        self.peer_count.store(peers.len(), Ordering::Release);
        Some((out, config))
    }

    /// Drops every peer whose entry is gone or was set again, discarding its
    /// held and queued sends. Peers are the few sessions under net chaos, so
    /// one walk per datagram is cheap, and it bounds how long a dead address
    /// keeps its state while other sessions have chaos.
    fn sweep(&self) {
        if self.peer_count.load(Ordering::Acquire) == 0 {
            return;
        }
        let now = Instant::now();
        let mut peers = self.peers();
        peers.retain(|addr, peer| {
            let live = self
                .registry
                .net_entry_for(*addr, now)
                .is_some_and(|(_, g)| g == peer.generation);
            if !live {
                self.discard(*addr, peer.generation);
            }
            live
        });
        self.peer_count.store(peers.len(), Ordering::Release);
    }

    /// Stops shaping `addr` up to `generation`, after a send found its entry
    /// gone while it was being delivered.
    fn forget(&self, addr: SocketAddr, generation: u64) {
        let mut peers = self.peers();
        if peers
            .get(&addr)
            .is_some_and(|peer| peer.generation <= generation)
        {
            peers.remove(&addr);
        }
        self.discard(addr, generation);
        self.peer_count.store(peers.len(), Ordering::Release);
    }

    /// Stops shaping every address (no session has net chaos any more).
    fn forget_all(&self) {
        let mut peers = self.peers();
        for (addr, peer) in peers.drain() {
            self.discard(addr, peer.generation);
        }
        self.peer_count.store(0, Ordering::Release);
    }

    fn discard(&self, addr: SocketAddr, generation: u64) {
        tracing::debug!(
            target: "lab.chaos",
            event = "lab_chaos_net_shaping",
            generation,
            "lab chaos net shaping ended on a session; held sends discarded"
        );
        self.shaper.discard(addr, generation);
    }

    /// WARNs, at most once per [`SHED_WARN_EVERY`], when a full delay line
    /// shed sends since the last WARN.
    fn warn_if_shed(&self) {
        let total = self.shaper.shed_total();
        let reported = self.shed_reported.load(Ordering::Relaxed);
        if total == reported {
            return;
        }
        let mut warned_at = self
            .shed_warned_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if warned_at.is_some_and(|at| at.elapsed() < SHED_WARN_EVERY) {
            return;
        }
        tracing::warn!(
            target: "lab.chaos",
            event = "lab_chaos_net_shed",
            shed = total - reported,
            shed_total = total,
            "lab chaos net delay line full: sends shed"
        );
        self.shed_reported.store(total, Ordering::Relaxed);
        *warned_at = Some(Instant::now());
    }
}

#[async_trait]
impl Transport for ChaosTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        if !self.registry.has_net() {
            if self.peer_count.load(Ordering::Acquire) > 0 {
                self.forget_all();
            }
            return self.inner.send_to(bytes, addr).await;
        }
        self.sweep();
        let Some((net, generation)) = self.entry_for(addr) else {
            return self.inner.send_to(bytes, addr).await;
        };
        let Some((roll, config)) = self.roll(addr, &net, generation, FlowFilter::roll_send) else {
            return self.inner.send_to(bytes, addr).await;
        };
        if roll.drop {
            // No row per drop: the seed replays the drop pattern exactly.
            // A wire-side drop: the kernel would have accepted the write.
            return Ok(bytes.len());
        }
        let sent = self
            .shaper
            .deliver_as(
                bytes,
                addr,
                &config,
                roll.duplicate,
                roll.latency,
                generation,
            )
            .await;
        // The entry can go while this send is delivered, after a teardown's
        // sweep already ran: whatever it just held or queued is the old
        // session's, so discard it here.
        if !self.is_current(addr, generation) {
            self.forget(addr, generation);
        }
        self.warn_if_shed();
        sent
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
            let Some((net, generation)) = self.entry_for(src) else {
                return Ok((n, src));
            };
            if net.loss_per_thousand == 0 {
                return Ok((n, src));
            }
            let dropped = self
                .roll(src, &net, generation, FlowFilter::roll_recv_drop)
                .is_some_and(|(dropped, _)| dropped);
            if !dropped {
                return Ok((n, src));
            }
            // No row per drop: the seed replays the drop pattern exactly.
        }
    }
}

#[cfg(test)]
#[path = "lab_chaos_transport_tests.rs"]
mod tests;
