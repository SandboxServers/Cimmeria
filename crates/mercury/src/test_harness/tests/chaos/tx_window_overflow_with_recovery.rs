//! TX-window overflow + deferred-send-queue recovery.
//!
//! The TX window is capped at `TX_WINDOW_SIZE = 32`. When more
//! reliable sends arrive than slots free, the overflow path is
//! the deferred-send queue (the TX-window-relief mechanism). This
//! scenario fires 50 reliable sends back-to-back and loses the 18
//! deferred ones in flight, so B's cumulative ack covers exactly the
//! TX-window-sized prefix. A drains the window, **promotes** the 18
//! deferred entries into the freed slots, and the retransmit scan
//! redelivers them. A second carrier acks the rest and the session
//! goes quiet.
//!
//! The harness puts every send on the wire immediately — deferral
//! only changes which queue tracks the entry, not whether bytes leave
//! the socket. Without the forced loss, B would owe all 50 acks, one
//! full cumulative ack would empty both queues directly, and the
//! promotion loop in `Channel::process_acks` would never run. The
//! loss is what makes this a promotion test rather than a drain test.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn tx_window_overflow_drains_via_deferred_queue() {
    let session = LoopbackSession::connected(None).await.unwrap();

    let total_sends = 50usize;
    let window = crate::consts::TX_WINDOW_SIZE;
    let deferred = total_sends - window;

    // Fill the TX window. These land at B.
    for i in 0..window {
        session
            .a
            .send_bundle(format!("burst-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // The overflow sends register in the deferred queue and are lost
    // in flight.
    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.drop_next.a_to_b = deferred as u32;
    }
    for i in window..total_sends {
        session
            .a
            .send_bundle(format!("burst-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // TX window saturated at 32; remaining 18 in deferred queue.
    {
        let channel = session.a.channel.lock().unwrap();
        assert_eq!(
            channel.tx_window.len(),
            window,
            "TX window must be saturated"
        );
        assert_eq!(
            channel.unsent_packets.len(),
            deferred,
            "sends past the TX-window cap must spill into the deferred queue"
        );
        all_safety_invariants(&channel);
    }

    // B receives the window-sized prefix. The recv pump records the
    // owed ack before it pushes to the inbox, so once the bundles are
    // here the acks are too — no separate wait on `pending_acks_len`.
    let received = session
        .b
        .recv_n_bundles(window, Duration::from_secs(5))
        .await;
    assert_eq!(
        received.len(),
        window,
        "the TX-window-sized prefix must land at B"
    );
    assert_eq!(
        session.b.pending_acks_len(),
        window,
        "B owes an ack for the delivered prefix and nothing else"
    );

    // B sends one packet so the piggyback cumulative ack rides back.
    // A's pump runs `process_acks` before it delivers the carrier to
    // the inbox, so receiving the carrier means the ack has applied.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let carrier = session.a.recv_n_bundles(1, Duration::from_secs(5)).await;
    assert_eq!(carrier.len(), 1, "prefix-ack carrier must reach A");

    // The prefix ack freed every TX-window slot; each freed slot
    // promotes one deferred entry so the retransmit scan can see it.
    {
        let channel = session.a.channel.lock().unwrap();
        assert_eq!(
            channel.unsent_packets.len(),
            0,
            "every deferred entry must leave the deferred queue on the prefix ack"
        );
        assert_eq!(
            channel.tx_window.len(),
            deferred,
            "every deferred entry must be promoted into the freed TX-window slots"
        );
        assert!(
            channel.tx_window.iter().all(|e| e.retransmit_count == 0),
            "promoted entries have not been retransmitted yet"
        );
        all_safety_invariants(&channel);
    }

    // Recovery: one clock jump past the RTO ceiling expires every
    // promoted entry. The scan resends RETRANSMIT_BUDGET_PER_TICK per
    // tick, and already-resent entries carry a fresh `last_sent`, so
    // repeated ticks walk the window front to back.
    session.a.clock.advance(Duration::from_secs(10));
    let mut retransmitted = 0;
    let max_ticks = deferred.div_ceil(crate::consts::RETRANSMIT_BUDGET_PER_TICK);
    for _ in 0..max_ticks {
        retransmitted += session.a.tick().await.unwrap().retransmits.len();
    }
    assert_eq!(
        retransmitted, deferred,
        "the retransmit scan must resend every promoted entry exactly once"
    );

    // B observes every deferred payload on retransmit.
    let post = session
        .b
        .recv_n_bundles(deferred, Duration::from_secs(5))
        .await;
    let post_payloads: std::collections::HashSet<Vec<u8>> =
        post.iter().map(|b| b.to_vec()).collect();
    for i in window..total_sends {
        let expected = format!("burst-{i}");
        assert!(
            post_payloads.contains(expected.as_bytes()),
            "retransmit phase must deliver burst-{i}; got {} unique payloads",
            post_payloads.len(),
        );
    }

    // Second carrier acks the promoted entries.
    session
        .b
        .send_bundle(b"ack carrier 2", false)
        .await
        .unwrap();
    let carrier = session.a.recv_n_bundles(1, Duration::from_secs(5)).await;
    assert_eq!(carrier.len(), 1, "final-ack carrier must reach A");

    let quiet = session.quiesce(Duration::from_millis(500)).await;
    assert!(quiet, "session must quiesce after the final cumulative ack");

    // The channel state must satisfy the safety invariants — no
    // overflow, no orphan retransmits.
    {
        let channel = session.a.channel.lock().unwrap();
        all_safety_invariants(&channel);
        assert_eq!(
            channel.tx_window.len(),
            0,
            "TX window must be fully drained after the final cumulative ack"
        );
        assert_eq!(
            channel.unsent_packets.len(),
            0,
            "deferred queue must stay empty after the final cumulative ack"
        );
    }
}
