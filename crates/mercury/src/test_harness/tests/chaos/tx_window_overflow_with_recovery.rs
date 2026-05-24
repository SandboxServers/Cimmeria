//! TX-window overflow + deferred-send-queue recovery.
//!
//! The TX window is capped at `TX_WINDOW_SIZE = 32`. When more
//! reliable sends arrive than slots free, the overflow path is
//! the deferred-send queue (the TX-window-relief mechanism). This
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

    // B receives the wire-arrived bundles. Only the TX-window-sized
    // portion goes on the wire pre-ack; deferred entries wait for
    // freed slots.
    let expected_pre_ack = std::cmp::min(total_sends as usize, crate::consts::TX_WINDOW_SIZE);
    let received = session
        .b
        .recv_n_bundles(expected_pre_ack, Duration::from_secs(2))
        .await;
    assert_eq!(
        received.len(),
        expected_pre_ack,
        "exactly the TX-window-sized portion ({expected_pre_ack}) must land at B pre-ack"
    );

    // B sends one packet so the piggyback cumulative ack rides back
    // and drains A's TX window. Each freed slot promotes one
    // deferred entry up to the wire-eligible state.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    // Post-ack: A's TX window has drained, deferred entries have
    // promoted + been emitted. B should now have received the
    // remaining (total_sends - expected_pre_ack) bundles.
    let remaining = (total_sends as usize) - expected_pre_ack;
    if remaining > 0 {
        let post = session
            .b
            .recv_n_bundles(remaining, Duration::from_secs(2))
            .await;
        assert_eq!(
            post.len(),
            remaining,
            "post-ack: every deferred entry must have promoted and delivered"
        );
    }

    // After the cumulative ack drain, the channel state must
    // satisfy the safety invariants — no overflow, no orphan
    // retransmits.
    {
        let channel = session.a.channel.lock().unwrap();
        all_safety_invariants(&channel);
        // Stronger end-state assertion: TX window is empty.
        assert_eq!(
            channel.tx_window.len(),
            0,
            "TX window must be fully drained after cumulative ack"
        );
        assert_eq!(
            channel.unsent_packets.len(),
            0,
            "deferred queue must be fully promoted after cumulative ack"
        );
    }
}
