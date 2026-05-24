//! BSF defeat-burst: at `onBeginAidWait` the original C++ server
//! fires a burst of ~19 reliable state-change packets in a single
//! tick. Without the bundle accumulator, each state-change is a
//! separate datagram; with TX_WINDOW_SIZE = 32 a single defeat
//! already sat at ~60% window usage, and a subsequent burst could
//! overflow.
//!
//! With the bundle accumulator now in place, related sends collapse
//! into fewer wire packets. This test models a 19-send burst
//! without bundling (the rust harness sends one-per-call); it
//! asserts the safety invariants hold and the channel doesn't
//! catastrophically overflow. With the deferred-send queue, any
//! overflow above TX_WINDOW_SIZE spills safely.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn defeat_burst_does_not_lose_packets_under_tx_window_pressure() {
    let session = LoopbackSession::connected(None).await.unwrap();

    let burst_size = 19u32;

    // Fire a 19-send burst (the unbundled shape).
    for i in 0..burst_size {
        session
            .a
            .send_bundle(format!("bsf-state-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // TX window holds the burst; no overflow past spec'd cap.
    {
        let channel = session.a.channel.lock().unwrap();
        assert!(
            channel.tx_window.len() + channel.unsent_packets.len() == burst_size as usize,
            "TX-window ({}) + deferred-queue ({}) must total burst size ({})",
            channel.tx_window.len(),
            channel.unsent_packets.len(),
            burst_size,
        );
        all_safety_invariants(&channel);
    }

    // B receives whatever made it on the wire (= up to TX_WINDOW_SIZE).
    let received = session
        .b
        .recv_n_bundles(burst_size as usize, Duration::from_secs(2))
        .await;
    // No wire loss in this scenario — every TX-window-eligible entry
    // (up to TX_WINDOW_SIZE) lands at B exactly once. Deferred entries
    // sit in A's unsent_packets and have NOT been put on the wire yet,
    // so B's expected count is min(burst_size, TX_WINDOW_SIZE).
    let expected_pre_ack = std::cmp::min(burst_size as usize, crate::consts::TX_WINDOW_SIZE);
    assert_eq!(
        received.len(),
        expected_pre_ack,
        "B must receive exactly {expected_pre_ack} bundles pre-ack (TX-window cap; \
         deferred entries don't go on the wire until acks free slots)"
    );

    // Cumulative ack carrier from B drains A's TX window;
    // deferred entries promote and go out.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    // After the ack drains: TX-window is empty AND every deferred
    // entry has been promoted + emitted, so B sees the remaining
    // packets too. For burst_size <= TX_WINDOW_SIZE this is a no-op.
    let remaining_after_ack = (burst_size as usize).saturating_sub(expected_pre_ack);
    if remaining_after_ack > 0 {
        let post = session
            .b
            .recv_n_bundles(remaining_after_ack, Duration::from_secs(2))
            .await;
        assert_eq!(
            post.len(),
            remaining_after_ack,
            "post-ack: deferred entries must promote and deliver"
        );
    }

    // Safety invariants persist after the ack-drain phase.
    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
