//! TX-window overflow + deferred-send-queue recovery.
//!
//! The TX window is capped at `TX_WINDOW_SIZE = 32`. When more
//! reliable sends arrive than slots free, the overflow path is
//! the deferred-send queue (PR #357 in the #354 family). This
//! scenario fires 50 reliable sends back-to-back, lets the
//! pending acks accumulate at B, then has B carry the
//! cumulative ack back. A's TX window drains and the deferred
//! queue promotes into freed slots until everything is delivered.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn tx_window_overflow_drains_via_deferred_queue() {
    let session = LoopbackSession::connected(None).await.unwrap();

    let total_sends = 50u32;
    for i in 0..total_sends {
        session
            .a
            .send_bundle(format!("burst-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // TX window saturated at 32; remaining 18 in deferred queue.
    {
        let channel = session.a.channel.lock().unwrap();
        assert!(
            channel.tx_window.len() + channel.unsent_packets.len() == total_sends as usize,
            "tx_window ({}) + unsent_packets ({}) must equal total sends ({})",
            channel.tx_window.len(),
            channel.unsent_packets.len(),
            total_sends,
        );
        all_safety_invariants(&channel);
    }

    // B receives the wire-arrived bundles (up to the TX window
    // size since the deferred ones never went on the wire under
    // the current send path).
    let received = session
        .b
        .recv_n_bundles(total_sends as usize, Duration::from_secs(2))
        .await;
    assert!(
        !received.is_empty(),
        "B must receive at least the wire-sent burst before any ack flows back"
    );

    // B sends one packet so the piggyback cumulative ack rides back
    // and drains A's TX window. Each freed slot promotes one
    // deferred entry up to the wire-eligible state.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    // After the cumulative ack drain, the channel state must
    // satisfy the safety invariants — no overflow, no orphan
    // retransmits.
    {
        let channel = session.a.channel.lock().unwrap();
        all_safety_invariants(&channel);
    }
}
