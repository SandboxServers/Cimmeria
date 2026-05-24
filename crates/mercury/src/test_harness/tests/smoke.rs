//! Smoke tests for the loopback harness wiring.
//!
//! These prove the harness can stand up two peers, exchange a packet,
//! and tear down cleanly. They do NOT exercise any of the 7 gap
//! categories the harness is built to test — those live in the sibling
//! `reliable` / `fragment` / `keepalive` / `encryption` / `handshake` /
//! `ack` / `rto` modules.

use std::time::Duration;

use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn unconnected_session_exchanges_one_packet() {
    let session = LoopbackSession::unconnected(None).await.unwrap();

    let payload = b"hello mercury";
    session.a.send_bundle(payload, false).await.unwrap();

    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;

    assert_eq!(bundles.len(), 1, "B must receive exactly one bundle");
    assert_eq!(
        bundles[0].as_ref(),
        payload,
        "delivered body must match the sent payload byte-for-byte"
    );
}

#[tokio::test]
async fn connected_session_handshake_completes() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Both inboxes should be drained after the handshake settles.
    assert_eq!(session.a.inbox_len(), 0);
    assert_eq!(session.b.inbox_len(), 0);

    // Channels should be in Connected.
    assert_eq!(
        session.a.channel.lock().unwrap().state,
        crate::channel::ChannelState::Connected
    );
    assert_eq!(
        session.b.channel.lock().unwrap().state,
        crate::channel::ChannelState::Connected
    );
}

#[tokio::test]
async fn reliable_send_then_recv_clears_tx_window_after_quiesce() {
    let session = LoopbackSession::connected(None).await.unwrap();

    let payload = b"reliable hello";
    session.a.send_bundle(payload, true).await.unwrap();
    assert_eq!(
        session.a.tx_window_len(),
        1,
        "reliable send must populate the TX window"
    );

    // B receives the bundle and queues an ack.
    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].as_ref(), payload);
    assert_eq!(
        session.b.pending_acks_len(),
        1,
        "B must owe A an ack for the reliable bundle"
    );

    // B sends anything back so the ack rides along.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    // The ack should have cleared A's TX window.
    let quiet = session.quiesce(Duration::from_millis(500)).await;
    assert!(quiet, "session must reach quiescence after the ack lands");
    assert_eq!(session.a.tx_window_len(), 0);
    assert_eq!(session.b.pending_acks_len(), 0);
}
