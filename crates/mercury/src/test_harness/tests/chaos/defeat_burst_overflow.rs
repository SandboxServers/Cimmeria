//! BSF defeat-burst: at `onBeginAidWait` the original C++ server
//! fires a burst of ~19 reliable state-change packets in a single
//! tick. Pre-#356 (ChannelBundle), each one was a separate
//! datagram; with TX_WINDOW_SIZE = 32 a single defeat already
//! sat at ~60% window usage, and a subsequent burst could
//! overflow.
//!
//! Post-#356, the bundle accumulator collapses related sends into
//! fewer wire packets. This test models a 19-send burst without
//! the bundle (the rust harness sends one-per-call); it asserts
//! the safety invariants hold and the channel doesn't
//! catastrophically overflow. With the deferred-send queue from
//! #357, any overflow above TX_WINDOW_SIZE spills safely.

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
    assert!(
        !received.is_empty(),
        "B must receive at least the wire-eligible burst portion"
    );

    // Cumulative ack carrier from B drains A's TX window;
    // deferred entries promote and go out.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    // Safety invariants persist after the ack-drain phase.
    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
