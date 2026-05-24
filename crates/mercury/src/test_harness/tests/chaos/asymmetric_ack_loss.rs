//! Asymmetric loss: server→client packets flow fine, client→server
//! ACKs all dropped. This is exactly lomiada's failure shape — the
//! server perceives a silent peer because no ACKs land, but the
//! client is alive and receiving normally.
//!
//! Expected behavior: A keeps retransmitting on RTO; A's TX-window
//! entries accumulate retransmit_count; eventually
//! `is_timed_out()` flips to true past `INACTIVITY_TIMEOUT_MS`.
//! Throughout, no state corruption — the channel either survives
//! (acks flow back) or is reaped (acks never flow back), no
//! third-state hang.

use std::time::Duration;

use crate::consts;
use crate::test_harness::invariants::{no_orphan_retransmits, tx_window_within_capacity};
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn asymmetric_ack_loss_times_out_eventually() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Block all B→A traffic (= all ACKs from client to server).
    // u32::MAX is the harness's "drop forever" sentinel.
    session.policy.lock().unwrap().drop_next.b_to_a = u32::MAX;

    // A sends a few reliable packets — B receives them but
    // cannot ack back (drops are silent on B→A).
    for i in 0..5u32 {
        session
            .a
            .send_bundle(format!("ack-loss-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // B's recv pump observes the originals and queues acks for
    // piggyback. B sends an "ack carrier" — but the carrier is
    // dropped by the B→A policy.
    let received = session.b.recv_n_bundles(5, Duration::from_secs(1)).await;
    assert_eq!(received.len(), 5, "B receives all originals (A→B is open)");
    session
        .b
        .send_bundle(b"would carry acks", false)
        .await
        .unwrap();

    // No ack reached A; TX window stays full.
    assert_eq!(
        session.a.tx_window_len(),
        5,
        "TX window holds all 5 sends — no ack arrived"
    );

    // Advance A's clock past INACTIVITY_TIMEOUT_MS to surface the
    // silent-peer detection. The channel-level is_timed_out check
    // reads last_received against the clock; since B's "ack carrier"
    // was dropped, last_received is still the handshake time.
    session
        .a
        .clock
        .advance(Duration::from_millis(consts::INACTIVITY_TIMEOUT_MS + 1_000));

    {
        let channel = session.a.channel.lock().unwrap();
        assert!(
            channel.is_timed_out(),
            "channel must be detectably timed out after INACTIVITY_TIMEOUT_MS \
             of silent-peer + ack-less retransmits"
        );
        // Even when timed out, the safety invariants still hold —
        // no corruption, no orphan-retransmit blowup.
        tx_window_within_capacity(&channel);
        no_orphan_retransmits(&channel);
    }
}
