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

    // No wire loss in this scenario, and the harness puts every send
    // on the wire immediately (deferral only changes which queue
    // tracks the entry), so the whole burst lands at B. Receiving all
    // of it up front also guarantees B owes every ack before the
    // carrier is built — a carrier built after a partial receive
    // returns a prefix ack (TESTING.md type 10). The promotion path is
    // pinned by `tx_window_overflow_with_recovery`.
    let received = session
        .b
        .recv_n_bundles(burst_size as usize, Duration::from_secs(2))
        .await;
    assert_eq!(
        received.len(),
        burst_size as usize,
        "B must receive the whole burst exactly once"
    );

    // Cumulative ack carrier from B drains A's TX window. A's pump
    // applies the ack before it delivers the carrier.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let carrier = session.a.recv_n_bundles(1, Duration::from_secs(5)).await;
    assert_eq!(carrier.len(), 1, "ack carrier must reach A");

    // Safety invariants persist after the ack-drain phase, and the
    // full cumulative ack leaves nothing tracked.
    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
    assert_eq!(
        channel.tx_window.len() + channel.unsent_packets.len(),
        0,
        "full cumulative ack must drain the TX window and the deferred queue"
    );
}
