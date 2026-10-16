//! Regression fixtures for Base's immutable send tickets and proof coverage.

use super::*;
use crate::auth::PendingLogin;
use crate::base::login::handle_login_with_issuer;
use crate::test_support::test_default_connected_client_state;
use cimmeria_base_session::base::plugin::BasePlugins;
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::EncryptionVersion;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

struct Wire {
    started: Notify,
    release: Notify,
    pause: AtomicBool,
    sent: Mutex<Vec<SocketAddr>>,
}

impl Wire {
    fn new() -> Self {
        Self {
            started: Notify::new(),
            release: Notify::new(),
            pause: AtomicBool::new(false),
            sent: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Transport for Wire {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        if self.pause.load(Ordering::SeqCst) && bytes == b"paused" {
            self.started.notify_one();
            self.release.notified().await;
        }
        self.sent.lock().unwrap().push(addr);
        match bytes {
            b"failed" => Err(io::Error::other("fake send failure")),
            b"partial" => Ok(bytes.len() - 1),
            _ => Ok(bytes.len()),
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

#[async_trait]
impl BidirectionalTransport for Wire {
    async fn recv_from(&self, _buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        Err(io::Error::other("unused fake recv"))
    }
}

fn peer(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn connected() -> Arc<Connected> {
    Arc::new(Mutex::new(HashMap::new()))
}

fn install(
    connected: &Arc<Connected>,
    peer: SocketAddr,
    seed: cimmeria_base_session::base::SessionOriginSeed,
) {
    let mut session = test_default_connected_client_state();
    session.extensions.insert(seed);
    connected.lock().unwrap().insert(peer, session);
}

#[test]
fn session_wire_proof_generation_mints_new_boot_and_samples_player_at_send() {
    let a = SessionOriginIssuer::new().unwrap();
    let b = SessionOriginIssuer::new().unwrap();
    assert_ne!(a.boot_id, b.boot_id);
    let address = peer(30001);
    let seed_a = a.issue(address, Instant::now());
    let seed_b = a.issue(address, Instant::now());
    assert_ne!(seed_a.session_generation, seed_b.session_generation);
    assert_eq!(seed_a.server_boot_id, seed_b.server_boot_id);
    let connected = connected();
    install(&connected, address, seed_a);
    let before = connected
        .lock()
        .unwrap()
        .get(&address)
        .unwrap()
        .send_origin()
        .unwrap();
    assert_eq!(before.player_entity_id, None);
    connected
        .lock()
        .unwrap()
        .get_mut(&address)
        .unwrap()
        .player_entity_id = Some(77);
    let after = connected
        .lock()
        .unwrap()
        .get(&address)
        .unwrap()
        .send_origin()
        .unwrap();
    assert_eq!(after.player_entity_id, Some(77));
    assert_eq!(before.session_generation, after.session_generation);
    assert_eq!(before.player_entity_id, None);
}

#[tokio::test]
async fn session_wire_proof_only_full_matching_socket_sends_are_rows() {
    let wire = Arc::new(Wire::new());
    let connected = connected();
    let issuer = SessionOriginIssuer::new().unwrap();
    let a = peer(30002);
    let b = peer(30003);
    install(&connected, a, issuer.issue(a, Instant::now()));
    install(&connected, b, issuer.issue(b, Instant::now()));
    let observer = ProofTransport::new(wire, &connected, false);
    let origin_a = connected
        .lock()
        .unwrap()
        .get(&a)
        .unwrap()
        .send_origin()
        .unwrap();
    let origin_b = connected
        .lock()
        .unwrap()
        .get(&b)
        .unwrap()
        .send_origin()
        .unwrap();

    observer.send_to_pre_phase3(b"reply", b).await.unwrap();
    observer
        .send_to_with_origin(b"ok", a, &origin_a)
        .await
        .unwrap();
    connected
        .lock()
        .unwrap()
        .get_mut(&a)
        .unwrap()
        .player_entity_id = Some(77);
    let selected = connected
        .lock()
        .unwrap()
        .get(&a)
        .unwrap()
        .send_origin()
        .unwrap();
    observer
        .send_to_with_origin(b"selected", a, &selected)
        .await
        .unwrap();
    observer
        .send_to_with_origin(b"partial", a, &origin_a)
        .await
        .unwrap();
    assert!(observer
        .send_to_with_origin(b"failed", a, &origin_a)
        .await
        .is_err());
    observer.send_to(b"legacy", a).await.unwrap();
    // Swapping recipient tickets must never turn either address into a row.
    observer
        .send_to_with_origin(b"wrong-a", a, &origin_b)
        .await
        .unwrap();
    observer
        .send_to_with_origin(b"wrong-b", b, &origin_a)
        .await
        .unwrap();

    let snapshot = observer.snapshot();
    assert_eq!(snapshot.rows.len(), 2);
    assert_eq!(snapshot.rows[0].peer, a);
    assert_eq!(snapshot.rows[0].fingerprint, wire_fingerprint(b"ok"));
    assert_eq!(snapshot.rows[0].player_entity_id, None);
    assert_eq!(snapshot.rows[1].player_entity_id, Some(77));
    assert_eq!(snapshot.gaps.pre_phase3_exempt, 1);
    assert_eq!(snapshot.gaps.untagged, 1);
    assert_eq!(snapshot.gaps.origin_peer_mismatch, 2);
    assert_eq!(snapshot.gaps.partial_send, 1);
    assert_eq!(snapshot.gaps.failed_send, 1);
    assert!(!snapshot.coverage_complete);
}

#[tokio::test]
async fn session_wire_proof_paused_old_send_does_not_relabel_new_generation() {
    let wire = Arc::new(Wire::new());
    wire.pause.store(true, Ordering::SeqCst);
    let connected = connected();
    let issuer = SessionOriginIssuer::new().unwrap();
    let address = peer(30004);
    install(&connected, address, issuer.issue(address, Instant::now()));
    let observer = Arc::new(ProofTransport::new(wire.clone(), &connected, false));
    let old = connected
        .lock()
        .unwrap()
        .get(&address)
        .unwrap()
        .send_origin()
        .unwrap();
    let observer_for_send = observer.clone();
    let sending = tokio::spawn(async move {
        observer_for_send
            .send_to_with_origin(b"paused", address, &old)
            .await
            .unwrap();
    });
    wire.started.notified().await;
    let pending = observer.snapshot();
    assert_eq!(pending.gaps.pending_tagged, 1);
    assert!(!pending.coverage_complete);
    let fresh = issuer.issue(address, Instant::now());
    let fresh_generation = fresh.session_generation.clone();
    install(&connected, address, fresh);
    wire.release.notify_one();
    sending.await.unwrap();
    let raw = observer.ring.lock().unwrap().rows[0].row.clone();
    assert_ne!(raw.session_generation, fresh_generation);
    assert!(observer.snapshot().rows.is_empty());
}

#[tokio::test]
async fn session_wire_proof_cancelled_await_is_a_permanent_gap() {
    let wire = Arc::new(Wire::new());
    wire.pause.store(true, Ordering::SeqCst);
    let connected = connected();
    let issuer = SessionOriginIssuer::new().unwrap();
    let address = peer(30010);
    install(&connected, address, issuer.issue(address, Instant::now()));
    let observer = Arc::new(ProofTransport::new(wire.clone(), &connected, false));
    let origin = connected
        .lock()
        .unwrap()
        .get(&address)
        .unwrap()
        .send_origin()
        .unwrap();
    let sending = tokio::spawn({
        let observer = observer.clone();
        async move {
            observer
                .send_to_with_origin(b"paused", address, &origin)
                .await
        }
    });
    wire.started.notified().await;
    sending.abort();
    assert!(sending.await.is_err());
    let snapshot = observer.snapshot();
    assert!(snapshot.rows.is_empty());
    assert_eq!(snapshot.gaps.pending_tagged, 0);
    assert_eq!(snapshot.gaps.cancelled_send, 1);
    assert!(!snapshot.coverage_complete);
}

#[tokio::test]
async fn session_wire_proof_chaos_success_is_ineligible() {
    let connected = connected();
    let address = peer(30005);
    let issuer = SessionOriginIssuer::new().unwrap();
    install(&connected, address, issuer.issue(address, Instant::now()));
    let origin = connected
        .lock()
        .unwrap()
        .get(&address)
        .unwrap()
        .send_origin()
        .unwrap();
    let observer = ProofTransport::new(Arc::new(Wire::new()), &connected, true);
    observer
        .send_to_with_origin(b"simulated-ok", address, &origin)
        .await
        .unwrap();
    let snapshot = observer.snapshot();
    assert!(snapshot.rows.is_empty());
    assert_eq!(snapshot.gaps.chaos_simulated, 1);
    assert!(!snapshot.coverage_complete);
}

#[tokio::test]
async fn session_wire_proof_ring_overflow_has_exact_sequence_and_drops() {
    let connected = connected();
    let address = peer(30006);
    let issuer = SessionOriginIssuer::new().unwrap();
    install(&connected, address, issuer.issue(address, Instant::now()));
    let origin = connected
        .lock()
        .unwrap()
        .get(&address)
        .unwrap()
        .send_origin()
        .unwrap();
    let observer = ProofTransport::new(Arc::new(Wire::new()), &connected, false);
    for number in 0..260u16 {
        observer
            .send_to_with_origin(&number.to_le_bytes(), address, &origin)
            .await
            .unwrap();
    }
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.rows.len(), 256);
    assert_eq!(snapshot.next_sequence, 260);
    assert_eq!(snapshot.rows.first().unwrap().sequence, 5);
    assert_eq!(snapshot.rows.last().unwrap().sequence, 260);
    assert_eq!(snapshot.dropped, 4);
    assert_eq!(snapshot.gaps.ring_overflow, 4);
}

#[test]
fn session_wire_proof_ring_expires_at_thirty_seconds() {
    let issuer = SessionOriginIssuer::new().unwrap();
    let address = peer(30009);
    let seed = issuer.issue(address, Instant::now());
    let origin = SendOrigin {
        server_boot_id: seed.server_boot_id,
        session_generation: seed.session_generation,
        peer: address,
        connected_at: seed.connected_at,
        player_entity_id: None,
    };
    let mut ring = Ring::default();
    let now = Instant::now();
    ring.record(b"old", &origin, now - MAX_AGE);
    ring.record(b"fresh", &origin, now);
    ring.expire(now);
    assert_eq!(ring.rows.len(), 1);
    assert_eq!(ring.rows[0].row.sequence, 2);
    assert_eq!(ring.dropped, 1);
    assert_eq!(ring.gaps.ring_expired, 1);
}

#[tokio::test]
async fn session_wire_proof_two_concurrent_logins_only_exempt_four_handshakes() {
    let connected = connected();
    let wire = Arc::new(Wire::new());
    let observer = Arc::new(ProofTransport::new(wire, &connected, false));
    let transport: Arc<dyn Transport> = observer.clone();
    let issuer = SessionOriginIssuer::new().unwrap();
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let entities = Arc::new(Mutex::new(EntityManager::new()));
    let addresses = Arc::new(Mutex::new(HashMap::new()));
    let first = peer(30007);
    let second = peer(30008);
    for (address, account) in [(first, 7001), (second, 7002)] {
        let ticket = format!("TICKET0000000000{account}");
        pending.lock().unwrap().insert(
            ticket.clone(),
            PendingLogin {
                account_id: account,
                account_name: format!("account{account}"),
                access_level: 0,
                test_account: false,
                ticket: ticket.clone(),
                session_key: "42".repeat(32),
                client_ip: address.ip(),
                created: Instant::now(),
            },
        );
        handle_login_with_issuer(
            &transport,
            address,
            1,
            &ticket,
            &pending,
            &connected,
            &entities,
            &None,
            &addresses,
            &None,
            EncryptionVersion::V1,
            &BasePlugins::empty(),
            &issuer,
        )
        .await
        .unwrap();
        if address == first {
            let origin = connected
                .lock()
                .unwrap()
                .get(&first)
                .unwrap()
                .send_origin()
                .unwrap();
            observer
                .send_to_with_origin(b"active", first, &origin)
                .await
                .unwrap();
        }
    }
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.gaps.pre_phase3_exempt, 4);
    assert_eq!(snapshot.gaps.untagged, 0);
    assert_eq!(snapshot.rows.len(), 1);
    assert!(snapshot.coverage_complete);
    for session in connected.lock().unwrap().values() {
        session.cancelled.store(true, Ordering::Relaxed);
    }
}
