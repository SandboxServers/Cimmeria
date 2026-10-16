//! Proof ring and send observer regressions.

use super::*;

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
async fn session_wire_proof_untagged_send_stays_incomplete_past_gap_age() {
    let wire = Arc::new(Wire::new());
    wire.pause.store(true, Ordering::SeqCst);
    let connected = connected();
    let issuer = SessionOriginIssuer::new().unwrap();
    let address = peer(30013);
    install(&connected, address, issuer.issue(address, Instant::now()));
    let observer = Arc::new(ProofTransport::new(wire.clone(), &connected, false));
    let sending = tokio::spawn({
        let observer = observer.clone();
        async move { observer.send_to(b"paused", address).await }
    });
    wire.started.notified().await;
    {
        let mut ring = observer.ring.lock().unwrap();
        ring.last_gap_at = Some(Instant::now() - MAX_AGE - Duration::from_millis(1));
    }
    let pending = observer.snapshot();
    assert_eq!(pending.gaps.pending_untagged, 1);
    assert!(!pending.coverage_complete);
    wire.release.notify_one();
    sending.await.unwrap().unwrap();
    let completed = observer.snapshot();
    assert_eq!(completed.gaps.pending_untagged, 0);
    assert_eq!(completed.gaps.untagged, 1);
    assert!(!completed.coverage_complete);
}

#[tokio::test]
async fn session_wire_proof_mismatched_send_refreshes_gap_after_long_await() {
    let wire = Arc::new(Wire::new());
    wire.pause.store(true, Ordering::SeqCst);
    let connected = connected();
    let issuer = SessionOriginIssuer::new().unwrap();
    let address = peer(30014);
    let other = peer(30015);
    install(&connected, address, issuer.issue(address, Instant::now()));
    let observer = Arc::new(ProofTransport::new(wire.clone(), &connected, false));
    let origin = connected.lock().unwrap()[&address]
        .wire_send_origin()
        .unwrap();
    let sending = tokio::spawn({
        let observer = observer.clone();
        async move {
            observer
                .send_to_with_origin(b"paused", other, &origin)
                .await
        }
    });
    wire.started.notified().await;
    {
        let mut ring = observer.ring.lock().unwrap();
        ring.last_gap_at = Some(Instant::now() - MAX_AGE - Duration::from_millis(1));
    }
    let pending = observer.snapshot();
    assert_eq!(pending.gaps.pending_tagged, 1);
    assert!(!pending.coverage_complete);
    wire.release.notify_one();
    sending.await.unwrap().unwrap();
    let completed = observer.snapshot();
    assert_eq!(completed.gaps.pending_tagged, 0);
    assert_eq!(completed.gaps.origin_peer_mismatch, 1);
    assert!(completed.rows.is_empty());
    assert!(!completed.coverage_complete);
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
