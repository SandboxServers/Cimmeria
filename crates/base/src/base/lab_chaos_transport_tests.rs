//! Network-chaos tests (TESTING.md type 10) for the lab chaos net shim, over
//! a recording fake wire so every outcome is deterministic. Each test builds
//! its own registry, never the process-wide one.

use super::*;
use cimmeria_base_session::base::lab_chaos::DEFAULT_TTL_S;
use tokio::sync::mpsc;

/// A recording wire: sends are kept in order, receives come from a queue the
/// test fills.
struct FakeWire {
    sent: Mutex<Vec<(SocketAddr, Vec<u8>)>>,
    inbound: tokio::sync::Mutex<mpsc::UnboundedReceiver<(Vec<u8>, SocketAddr)>>,
}

#[async_trait]
impl Transport for FakeWire {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.sent.lock().expect("sent").push((addr, bytes.to_vec()));
        Ok(bytes.len())
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(addr(32_832))
    }
}

#[async_trait]
impl BidirectionalTransport for FakeWire {
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (bytes, src) = self
            .inbound
            .lock()
            .await
            .recv()
            .await
            .ok_or_else(|| io::Error::other("fake wire closed"))?;
        buf[..bytes.len()].copy_from_slice(&bytes);
        Ok((bytes.len(), src))
    }
}

struct Rig {
    wire: Arc<FakeWire>,
    inject: mpsc::UnboundedSender<(Vec<u8>, SocketAddr)>,
    registry: &'static LabChaos,
    shim: ChaosTransport,
}

impl Rig {
    fn new() -> Self {
        let (inject, rx) = mpsc::unbounded_channel();
        let wire = Arc::new(FakeWire {
            sent: Mutex::new(Vec::new()),
            inbound: tokio::sync::Mutex::new(rx),
        });
        // Leaked so the shim gets the `'static` registry the server's
        // installed one is; a few bytes per test.
        let registry: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
        let shim = ChaosTransport::new(wire.clone(), registry);
        Self {
            wire,
            inject,
            registry,
            shim,
        }
    }

    fn set(&self, peer: SocketAddr, net: NetChaos, ttl_s: u32) {
        self.registry
            .set(
                peer,
                1007,
                "net".into(),
                Some(net),
                None,
                ttl_s,
                Instant::now(),
            )
            .expect("a valid set");
    }

    async fn send_seqs(&self, peer: SocketAddr, seqs: std::ops::Range<u32>) {
        for seq in seqs {
            self.shim
                .send_to(&seq.to_le_bytes(), peer)
                .await
                .expect("send");
        }
    }

    /// The sequence numbers the wire carried to `peer`, in wire order.
    fn delivered(&self, peer: SocketAddr) -> Vec<u32> {
        self.wire
            .sent
            .lock()
            .expect("sent")
            .iter()
            .filter(|(to, _)| *to == peer)
            .map(|(_, b)| u32::from_le_bytes(b[..4].try_into().expect("4 bytes")))
            .collect()
    }

    fn inject(&self, from: SocketAddr, seq: u32) {
        self.inject
            .send((seq.to_le_bytes().to_vec(), from))
            .expect("inject");
    }

    /// The next datagram the shim hands the receive loop, as (source, seq),
    /// or `None` if none arrives within `within`.
    async fn recv(&self, within: Duration) -> Option<(SocketAddr, u32)> {
        let mut buf = [0u8; 64];
        let (n, src) = tokio::time::timeout(within, self.shim.recv_from(&mut buf))
            .await
            .ok()?
            .expect("recv never errors on drops");
        Some((
            src,
            u32::from_le_bytes(buf[..n].try_into().expect("4 bytes")),
        ))
    }
}

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn lossy(loss_per_thousand: u32, seed: u64) -> NetChaos {
    NetChaos {
        latency_ms: 0,
        jitter_ms: 0,
        loss_per_thousand,
        dup_per_thousand: 0,
        reorder: 0,
        seed,
    }
}

const A: u16 = 41_001;
const B: u16 = 41_002;

/// Chaos on A at 100% loss, both directions; B, on the same socket, loses
/// nothing. Fails if the shim stops keying on the peer address. A's 100
/// inbound drops in a row also must not surface as a receive error (the
/// receive loop serves every session).
#[tokio::test]
async fn net_chaos_is_scoped_to_one_peer() {
    let rig = Rig::new();
    rig.set(addr(A), lossy(1000, 1), DEFAULT_TTL_S);

    rig.send_seqs(addr(A), 0..20).await;
    rig.send_seqs(addr(B), 0..20).await;
    assert_eq!(rig.delivered(addr(A)), Vec::<u32>::new());
    assert_eq!(rig.delivered(addr(B)), (0..20).collect::<Vec<_>>());

    for seq in 0..100 {
        rig.inject(addr(A), seq);
    }
    rig.inject(addr(B), 7);
    assert_eq!(
        rig.recv(Duration::from_secs(5)).await,
        Some((addr(B), 7)),
        "A's datagrams are dropped, B's arrives"
    );
}

/// The same seed drops the same sequence numbers; another seed does not.
/// Fails if the filter's RNG is not seeded from the registry entry.
#[tokio::test]
async fn net_chaos_replays_with_the_same_seed() {
    async fn survivors(seed: u64) -> Vec<u32> {
        let rig = Rig::new();
        rig.set(addr(A), lossy(300, seed), DEFAULT_TTL_S);
        rig.send_seqs(addr(A), 0..200).await;
        rig.delivered(addr(A))
    }
    let first = survivors(7).await;
    assert!(
        !first.is_empty() && first.len() < 200,
        "30% loss drops some, not all: {} of 200 arrived",
        first.len()
    );
    assert_eq!(survivors(7).await, first, "seed 7 replays");
    assert_ne!(survivors(8).await, first, "seed 8 is another run");
}

/// Once the entry's TTL passes, the peer is sent to and heard from again,
/// and the shim drops its per-peer state. Fails if the shim caches the
/// chaos past the registry's expiry.
#[tokio::test]
async fn expired_net_chaos_stops_dropping() {
    let rig = Rig::new();
    rig.set(addr(A), lossy(1000, 1), 1);
    rig.send_seqs(addr(A), 0..10).await;
    assert_eq!(rig.delivered(addr(A)), Vec::<u32>::new(), "live: all lost");

    // The registry's clock is `std::time::Instant`, so this is a real wait.
    tokio::time::sleep(Duration::from_millis(1_100)).await;
    rig.send_seqs(addr(A), 10..20).await;
    assert_eq!(rig.delivered(addr(A)), (10..20).collect::<Vec<_>>());
    assert!(
        !rig.registry.has_net(),
        "the read removed the expired entry"
    );
    assert_eq!(rig.shim.peer_count.load(Ordering::Acquire), 0);
    rig.inject(addr(A), 99);
    assert_eq!(rig.recv(Duration::from_secs(5)).await, Some((addr(A), 99)));
}

/// A is under 50% receive loss with 2 s of latency; B's datagram, queued
/// behind 20 of A's, still comes out at once, and so do A's survivors. Fails
/// if the shim sleeps in `recv_from` (for A's latency, or per drop), which
/// would stall the one task that reads every session.
#[tokio::test]
async fn receive_drop_never_delays_other_peers() {
    let rig = Rig::new();
    let mut slow = lossy(500, 3);
    slow.latency_ms = 2_000;
    rig.set(addr(A), slow, DEFAULT_TTL_S);
    for seq in 0..20 {
        rig.inject(addr(A), seq);
    }
    rig.inject(addr(B), 7);

    let started = std::time::Instant::now();
    let mut from_a = Vec::new();
    loop {
        let (src, seq) = rig
            .recv(Duration::from_millis(500))
            .await
            .expect("B's datagram arrives without waiting on A's latency");
        if src == addr(B) {
            assert_eq!(seq, 7);
            break;
        }
        from_a.push(seq);
    }
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "receive took {:?}",
        started.elapsed()
    );
    assert!(
        !from_a.is_empty() && from_a.len() < 20,
        "A is dropped, not delayed: {} of 20 got through",
        from_a.len()
    );
}

/// With nothing set, every datagram passes through untouched and in order,
/// and the shim builds no per-peer state.
#[tokio::test]
async fn no_chaos_passes_everything_through() {
    let rig = Rig::new();
    rig.send_seqs(addr(A), 0..50).await;
    assert_eq!(rig.delivered(addr(A)), (0..50).collect::<Vec<_>>());
    rig.inject(addr(A), 5);
    assert_eq!(rig.recv(Duration::from_secs(5)).await, Some((addr(A), 5)));
    assert_eq!(rig.shim.peer_count.load(Ordering::Acquire), 0);
}

/// Sends held for reordering are released when the chaos is cleared, not
/// stranded. Fails if the shim forgets a peer without releasing its bucket.
#[tokio::test]
async fn clearing_chaos_releases_held_reorder_sends() {
    let rig = Rig::new();
    let mut reorder = lossy(0, 1);
    reorder.reorder = 5;
    rig.set(addr(A), reorder, DEFAULT_TTL_S);
    rig.send_seqs(addr(A), 0..3).await;
    assert_eq!(rig.delivered(addr(A)), Vec::<u32>::new(), "held");

    assert_eq!(rig.registry.clear(Some(addr(A))), 1);
    rig.send_seqs(addr(A), 3..4).await;
    assert_eq!(rig.delivered(addr(A)), [2, 1, 0, 3]);
    assert_eq!(rig.shim.peer_count.load(Ordering::Acquire), 0);
}

/// A full reorder bucket goes out reversed, the `LossyTransport` contract.
/// Fails if the shim stops delivering through the shared shaper.
#[tokio::test]
async fn reorder_matches_the_lossy_transport_contract() {
    let rig = Rig::new();
    let mut reorder = lossy(0, 1);
    reorder.reorder = 3;
    rig.set(addr(A), reorder, DEFAULT_TTL_S);
    rig.send_seqs(addr(A), 0..6).await;
    assert_eq!(rig.delivered(addr(A)), [2, 1, 0, 5, 4, 3]);
}
