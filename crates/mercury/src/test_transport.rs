//! Recording [`Transport`] implementation for use in unit tests.
//!
//! Gated by `cfg(any(test, feature = "test-support"))`: available to
//! mercury's own tests, and to consumer crates that opt in via a
//! dev-dependency feature —
//! `cimmeria-mercury = { path = "../mercury", features = ["test-support"] }`.
//! Production builds never include it.
//!
//! [`TestTransport`] is the canonical UDP fake: a drop-in for
//! [`UdpTransport`](crate::transport::UdpTransport) that records every
//! outbound `send_to` so a test can assert byte-exact, addr-correct fan-out
//! ("when handler H runs with state S, exactly these `(addr, bytes)` pairs hit
//! the transport, in this order"). See `docs/architecture/transport-trait.md`.

use std::io;
use std::net::SocketAddr;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::encryption::MercuryEncryption;
use crate::packet::parse_incoming;
use crate::transport::Transport;

/// Records every outbound `send_to` for later inspection. Used as a drop-in
/// for [`UdpTransport`](crate::transport::UdpTransport) in unit tests.
///
/// The recorder is `Mutex`-protected so tests can hold a `&dyn Transport`
/// reference and still call [`drain`](Self::drain) from the test thread
/// between awaits.
///
/// # Thread safety
///
/// `TestTransport` is `Send + Sync`. The internal `Mutex<Vec<...>>` serializes
/// concurrent `send_to` calls; records land in lock-acquisition order (FIFO
/// per acquirer). For sequential single-task tests — the dominant pattern —
/// `drain`/`filter_to` returns records in send-order. For multi-task tests
/// that race sends from several `tokio::spawn`-ed tasks, the lock guarantees
/// every send is recorded but **inter-task ordering is non-deterministic** —
/// assert on the *set* of sends (and per-recipient cardinality), not the
/// sequence index.
pub struct TestTransport {
    sent: Mutex<Vec<(SocketAddr, Vec<u8>)>>,
    local: SocketAddr,
}

impl TestTransport {
    /// Construct a transport bound to a synthetic loopback addr. The addr is
    /// used purely as the value [`local_addr`](Transport::local_addr) returns
    /// — nothing actually binds to it.
    pub fn new() -> Self {
        Self::with_local("127.0.0.1:0".parse().unwrap())
    }

    /// Construct a transport whose [`local_addr`](Transport::local_addr)
    /// returns `addr`. Use this when a handler reads `local_addr()` to build a
    /// reply address and the test asserts on that path.
    pub fn with_local(addr: SocketAddr) -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            local: addr,
        }
    }

    /// Take all recorded sends since the last drain, in send-order.
    pub fn drain(&self) -> Vec<(SocketAddr, Vec<u8>)> {
        std::mem::take(&mut *self.sent.lock().unwrap())
    }

    /// Discard all recorded sends without returning them. Use when you want to
    /// reset state between phases of a test without consuming the records.
    pub fn clear(&self) {
        self.sent.lock().unwrap().clear();
    }

    /// All packets sent to `addr` in send-order. Allocates — for assertions
    /// that only need the count, prefer [`send_count_to`](Self::send_count_to).
    pub fn filter_to(&self, addr: SocketAddr) -> Vec<Vec<u8>> {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, b)| b.clone())
            .collect()
    }

    /// Number of sends recorded.
    pub fn len(&self) -> usize {
        self.sent.lock().unwrap().len()
    }

    /// True if no sends have been recorded.
    pub fn is_empty(&self) -> bool {
        self.sent.lock().unwrap().is_empty()
    }

    /// Sum of all sent payload sizes. Useful for asserting bandwidth bounds or
    /// "expected ~N bytes total fanned out across K witnesses".
    pub fn total_bytes_sent(&self) -> usize {
        self.sent.lock().unwrap().iter().map(|(_, b)| b.len()).sum()
    }

    /// Count of sends targeting `addr`, without allocating the payload `Vec`
    /// that [`filter_to`](Self::filter_to) would. Use for cardinality
    /// assertions in fan-out tests.
    pub fn send_count_to(&self, addr: SocketAddr) -> usize {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .count()
    }
}

impl Default for TestTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// The bundle body of each send in `packets`, decrypted with `enc`: a
/// single datagram's body as sent, or the bodies of a fragmented bundle
/// joined in order. A send too big for the client's buffer goes out as
/// fragments, so a test that decodes "one packet = one bundle" reads the
/// recorded sends through this.
///
/// Panics on a packet that does not decrypt or parse, on a fragment whose
/// range differs from the bundle it continues, and on a bundle whose last
/// fragment was not sent.
pub fn reassembled_bodies(packets: &[Vec<u8>], enc: &MercuryEncryption) -> Vec<Vec<u8>> {
    let mut bodies = Vec::new();
    let mut open: Option<((u32, u32), Vec<u8>)> = None;
    for (i, wire) in packets.iter().enumerate() {
        let plaintext = enc.decrypt(wire).expect("decrypt test packet");
        let packet = parse_incoming(&plaintext).expect("parse test packet");
        let (Some(begin), Some(end)) = (packet.frag_begin, packet.frag_end) else {
            assert!(open.is_none(), "packet {i} interrupts a fragmented bundle");
            bodies.push(packet.body.to_vec());
            continue;
        };
        let (range, mut body) = open.take().unwrap_or(((begin, end), Vec::new()));
        assert_eq!(range, (begin, end), "packet {i} is in another bundle");
        body.extend_from_slice(&packet.body);
        if packet.seq_id == Some(end) {
            bodies.push(body);
        } else {
            open = Some((range, body));
        }
    }
    assert!(open.is_none(), "the last fragmented bundle is incomplete");
    bodies
}

#[async_trait]
impl Transport for TestTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.sent.lock().unwrap().push((addr, bytes.to_vec()));
        Ok(bytes.len())
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn addr(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    fn enc() -> MercuryEncryption {
        MercuryEncryption::from_session_key([0u8; 32])
    }

    /// One encrypted single-datagram bundle at `seq`.
    fn single(body: &[u8], seq: u32) -> Vec<u8> {
        use crate::packet::{build_outgoing, FLAG_HAS_SEQUENCE, FLAG_RELIABLE};
        let flags = FLAG_RELIABLE | FLAG_HAS_SEQUENCE;
        enc()
            .encrypt(&build_outgoing(flags, body, Some(seq), &[], None))
            .unwrap()
    }

    /// One encrypted fragment at `seq` of the bundle `begin..=end`.
    fn fragment(body: &[u8], seq: u32, begin: u32, end: u32) -> Vec<u8> {
        use crate::packet::{build_outgoing_fragmented, FLAG_RELIABLE};
        enc()
            .encrypt(&build_outgoing_fragmented(
                FLAG_RELIABLE,
                body,
                seq,
                begin,
                end,
                &[],
            ))
            .unwrap()
    }

    /// A send that went out as fragments reads back as the one body it
    /// carried, between the single-datagram sends around it.
    #[test]
    fn reassembled_bodies_joins_fragments_and_keeps_singles() {
        let wire = vec![
            single(b"one", 4),
            fragment(b"two-", 5, 5, 7),
            fragment(b"three-", 6, 5, 7),
            fragment(b"four", 7, 5, 7),
            single(b"five", 8),
        ];
        assert_eq!(
            reassembled_bodies(&wire, &enc()),
            vec![
                b"one".to_vec(),
                b"two-three-four".to_vec(),
                b"five".to_vec()
            ]
        );
    }

    #[test]
    #[should_panic(expected = "the last fragmented bundle is incomplete")]
    fn reassembled_bodies_refuses_a_bundle_without_its_last_fragment() {
        reassembled_bodies(&[fragment(b"two-", 5, 5, 6)], &enc());
    }

    #[test]
    #[should_panic(expected = "interrupts a fragmented bundle")]
    fn reassembled_bodies_refuses_a_single_inside_a_bundle() {
        let wire = vec![fragment(b"two-", 5, 5, 6), single(b"one", 7)];
        reassembled_bodies(&wire, &enc());
    }

    #[tokio::test]
    async fn records_each_send_in_order() {
        let t = TestTransport::new();
        let a = addr("127.0.0.1:1000");
        let b = addr("127.0.0.1:1001");
        t.send_to(b"first", a).await.unwrap();
        t.send_to(b"second", b).await.unwrap();
        t.send_to(b"third", a).await.unwrap();

        let sent = t.drain();
        assert_eq!(
            sent,
            vec![
                (a, b"first".to_vec()),
                (b, b"second".to_vec()),
                (a, b"third".to_vec()),
            ]
        );
    }

    #[tokio::test]
    async fn send_to_returns_payload_len() {
        let t = TestTransport::new();
        let n = t.send_to(b"abcd", addr("127.0.0.1:1000")).await.unwrap();
        assert_eq!(n, 4);
    }

    #[tokio::test]
    async fn drain_returns_then_resets() {
        let t = TestTransport::new();
        t.send_to(b"x", addr("127.0.0.1:1000")).await.unwrap();
        assert_eq!(t.drain().len(), 1);
        // Second drain sees nothing — drain consumed the records.
        assert!(t.drain().is_empty());
        assert!(t.is_empty());
    }

    #[tokio::test]
    async fn clear_resets_without_returning() {
        let t = TestTransport::new();
        t.send_to(b"x", addr("127.0.0.1:1000")).await.unwrap();
        t.send_to(b"y", addr("127.0.0.1:1000")).await.unwrap();
        assert_eq!(t.len(), 2);
        t.clear();
        assert_eq!(t.len(), 0);
        assert!(t.is_empty());
    }

    #[tokio::test]
    async fn filter_to_scopes_to_addr_in_order() {
        let t = TestTransport::new();
        let a = addr("127.0.0.1:1000");
        let b = addr("127.0.0.1:1001");
        t.send_to(b"a1", a).await.unwrap();
        t.send_to(b"b1", b).await.unwrap();
        t.send_to(b"a2", a).await.unwrap();

        assert_eq!(t.filter_to(a), vec![b"a1".to_vec(), b"a2".to_vec()]);
        assert_eq!(t.filter_to(b), vec![b"b1".to_vec()]);
        assert!(t.filter_to(addr("127.0.0.1:9999")).is_empty());
    }

    #[tokio::test]
    async fn send_count_to_matches_filter_to_len() {
        let t = TestTransport::new();
        let a = addr("127.0.0.1:1000");
        let b = addr("127.0.0.1:1001");
        t.send_to(b"a1", a).await.unwrap();
        t.send_to(b"a2", a).await.unwrap();
        t.send_to(b"b1", b).await.unwrap();

        assert_eq!(t.send_count_to(a), t.filter_to(a).len());
        assert_eq!(t.send_count_to(a), 2);
        assert_eq!(t.send_count_to(b), 1);
        assert_eq!(t.send_count_to(addr("127.0.0.1:9999")), 0);
    }

    #[tokio::test]
    async fn total_bytes_sent_sums_payload_sizes() {
        let t = TestTransport::new();
        t.send_to(b"abc", addr("127.0.0.1:1000")).await.unwrap(); // 3
        t.send_to(b"de", addr("127.0.0.1:1001")).await.unwrap(); // 2
        t.send_to(b"", addr("127.0.0.1:1002")).await.unwrap(); // 0
        assert_eq!(t.total_bytes_sent(), 5);
    }

    #[tokio::test]
    async fn len_and_is_empty_contract() {
        let t = TestTransport::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
        t.send_to(b"x", addr("127.0.0.1:1000")).await.unwrap();
        assert!(!t.is_empty());
        assert_eq!(t.len(), 1);
    }

    #[tokio::test]
    async fn with_local_sets_local_addr() {
        let synthetic = addr("10.1.2.3:7777");
        let t = TestTransport::with_local(synthetic);
        assert_eq!(t.local_addr().unwrap(), synthetic);
        // new() defaults to an unbound loopback addr.
        assert_eq!(
            TestTransport::new().local_addr().unwrap(),
            addr("127.0.0.1:0")
        );
    }

    #[tokio::test]
    async fn usable_as_dyn_transport_behind_arc() {
        // Pin the intended usage shape: handlers take `&Arc<dyn Transport>`.
        let t: Arc<dyn Transport> = Arc::new(TestTransport::new());
        t.send_to(b"hello", addr("127.0.0.1:1000")).await.unwrap();
        // Downcast-free inspection isn't possible through `dyn Transport`, so
        // tests keep a typed handle; this just proves the trait-object path
        // compiles and records.
        assert_eq!(t.local_addr().unwrap(), addr("127.0.0.1:0"));
    }
}
