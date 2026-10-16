//! Regression fixtures for Base's immutable send tickets and proof coverage.

use super::*;
use crate::auth::PendingLogin;
use crate::base::service::BaseService;
use crate::test_support::test_default_connected_client_state;
use cimmeria_common::ServerConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::UdpSocket;
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
fn session_wire_proof_accessor_samples_player_at_send() {
    let a = SessionOriginIssuer::new().unwrap();
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
        .wire_send_origin()
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
        .wire_send_origin()
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
        .wire_send_origin()
        .unwrap();
    let origin_b = connected
        .lock()
        .unwrap()
        .get(&b)
        .unwrap()
        .wire_send_origin()
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
        .wire_send_origin()
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
        .wire_send_origin()
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
        .wire_send_origin()
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
async fn session_wire_proof_snapshot_cannot_publish_a_after_b_relog() {
    let connected = connected();
    let wire = Arc::new(Wire::new());
    let issuer = SessionOriginIssuer::new().unwrap();
    let address = peer(30012);
    install(&connected, address, issuer.issue(address, Instant::now()));
    let observer = Arc::new(ProofTransport::new(wire, &connected, false));
    let a = connected.lock().unwrap()[&address]
        .wire_send_origin()
        .unwrap();
    observer
        .send_to_with_origin(b"a", address, &a)
        .await
        .unwrap();

    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let reading = {
        let observer = observer.clone();
        std::thread::spawn(move || {
            observer.snapshot_after_lock(|| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        })
    };
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(
        connected.try_lock().is_err(),
        "snapshot released the roster lock before filtering"
    );
    let b_seed = issuer.issue(address, Instant::now());
    let relog = {
        let connected = connected.clone();
        std::thread::spawn(move || install(&connected, address, b_seed))
    };
    release_tx.send(()).unwrap();
    let first = reading.join().unwrap();
    assert_eq!(first.rows.len(), 1);
    assert_eq!(first.rows[0].session_generation, a.session_generation);
    relog.join().unwrap();
    assert!(observer.snapshot().rows.is_empty());
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
        .wire_send_origin()
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
        .wire_send_origin()
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
async fn session_wire_proof_expired_gap_and_row_allow_fresh_window() {
    let connected = connected();
    let address = peer(30011);
    let issuer = SessionOriginIssuer::new().unwrap();
    install(&connected, address, issuer.issue(address, Instant::now()));
    let origin = connected
        .lock()
        .unwrap()
        .get(&address)
        .unwrap()
        .wire_send_origin()
        .unwrap();
    let observer = ProofTransport::new(Arc::new(Wire::new()), &connected, false);
    observer.send_to(b"old-untagged", address).await.unwrap();
    observer
        .send_to_with_origin(b"old-row", address, &origin)
        .await
        .unwrap();
    assert!(!observer.snapshot().coverage_complete);
    {
        let mut ring = observer.ring.lock().unwrap();
        let past = Instant::now() - MAX_AGE - Duration::from_millis(1);
        ring.last_gap_at = Some(past);
        ring.rows.front_mut().unwrap().added = past;
    }
    observer
        .send_to_with_origin(b"fresh", address, &origin)
        .await
        .unwrap();
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.rows[0].fingerprint, wire_fingerprint(b"fresh"));
    assert_eq!(snapshot.gaps.untagged, 1);
    assert_eq!(snapshot.gaps.ring_expired, 1);
    assert_eq!(snapshot.dropped, 1);
    assert!(snapshot.coverage_complete);
}

fn issue_login(pending: &Arc<Mutex<HashMap<String, PendingLogin>>>, account: u32) -> String {
    let ticket = format!("TICKET{account:014}");
    pending.lock().unwrap().insert(
        ticket.clone(),
        PendingLogin {
            account_id: account,
            account_name: format!("account{account}"),
            access_level: 0,
            test_account: false,
            ticket: ticket.clone(),
            session_key: "42".repeat(32),
            client_ip: "127.0.0.1".parse().unwrap(),
            created: Instant::now(),
        },
    );
    ticket
}

fn login_datagram(ticket: &str, account: u32) -> Vec<u8> {
    let mut raw = vec![0x41, 0x00];
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
    raw.extend_from_slice(&account.to_le_bytes());
    raw.push(ticket.len() as u8);
    raw.extend_from_slice(ticket.as_bytes());
    raw.extend_from_slice(&1u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw
}

async fn await_sessions(service: &BaseService, count: usize) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if service.connected_clients.lock().unwrap().len() == count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("real Base receive loop did not register sessions");
}

#[tokio::test]
async fn session_wire_proof_service_restart_and_overlapping_phase3_logins() {
    let config = ServerConfig {
        base_port: 0,
        ..ServerConfig::loopback()
    };
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let mut first_service = BaseService::new(&config);
    first_service.set_pending_logins(pending.clone());
    first_service.start().await.unwrap();
    let first_server = first_service
        .proof_transport
        .as_ref()
        .unwrap()
        .local_addr()
        .unwrap();
    let a_client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let b_client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let a_peer = a_client.local_addr().unwrap();
    let b_peer = b_client.local_addr().unwrap();
    let a_ticket = issue_login(&pending, 7001);
    let b_ticket = issue_login(&pending, 7002);
    let a_login = login_datagram(&a_ticket, 7001);
    let b_login = login_datagram(&b_ticket, 7002);
    let (a_sent, b_sent) = tokio::join!(
        a_client.send_to(&a_login, first_server),
        b_client.send_to(&b_login, first_server),
    );
    a_sent.unwrap();
    b_sent.unwrap();
    await_sessions(&first_service, 2).await;
    let (a, b) = {
        let sessions = first_service.connected_clients.lock().unwrap();
        (
            sessions[&a_peer].wire_send_origin().unwrap(),
            sessions[&b_peer].wire_send_origin().unwrap(),
        )
    };
    assert_eq!(a.server_boot_id, b.server_boot_id);
    assert_ne!(a.session_generation, b.session_generation);
    assert_eq!(a.player_entity_id, None);
    let proof = first_service.proof_transport.as_ref().unwrap();
    proof
        .send_to_with_origin(b"active", a_peer, &a)
        .await
        .unwrap();
    let snapshot = proof.snapshot();
    assert_eq!(snapshot.gaps.pre_phase3_exempt, 4);
    assert!(snapshot
        .rows
        .iter()
        .any(|row| row.session_generation == a.session_generation));

    for session in first_service.connected_clients.lock().unwrap().values() {
        session.cancelled.store(true, Ordering::Relaxed);
    }
    first_service.stop().await;

    let restart_pending = Arc::new(Mutex::new(HashMap::new()));
    let mut restarted = BaseService::new(&config);
    restarted.set_pending_logins(restart_pending.clone());
    restarted.start().await.unwrap();
    let second_server = restarted
        .proof_transport
        .as_ref()
        .unwrap()
        .local_addr()
        .unwrap();
    let restart_ticket = issue_login(&restart_pending, 7003);
    a_client
        .send_to(&login_datagram(&restart_ticket, 7003), second_server)
        .await
        .unwrap();
    await_sessions(&restarted, 1).await;
    let restart_origin = restarted.connected_clients.lock().unwrap()[&a_peer]
        .wire_send_origin()
        .unwrap();
    assert_ne!(a.server_boot_id, restart_origin.server_boot_id);
    assert_ne!(a.session_generation, restart_origin.session_generation);
    restarted.connected_clients.lock().unwrap()[&a_peer]
        .cancelled
        .store(true, Ordering::Relaxed);
}
