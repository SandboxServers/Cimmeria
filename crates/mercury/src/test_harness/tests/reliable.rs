//! Category 1 — reliable delivery under simulated loss.
//!
//! Every leg of "send → drop → RTO → retransmit → ack → tx-window clear"
//! has unit tests in `crates/mercury/src/channel/tests/`. These tests
//! exercise the **pipeline itself**: two real channels on real loopback
//! sockets driving the end-to-end behavior under a policy-controlled
//! drop pattern. Time is advanced via [`TestClock`](super::super::TestClock)
//! rather than wall-clock sleeps so the suite stays fast and
//! deterministic.

use std::time::Duration;

use crate::consts;
use crate::test_harness::LoopbackSession;

/// Drop A's first reliable packet → advance A's clock past RTO → tick
/// fires the retransmit → B receives it and acks → A's tx_window
/// clears.
#[tokio::test]
async fn reliable_packet_retransmits_after_drop() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Drop A's next outbound packet on its way to B.
    session.policy.lock().unwrap().drop_next.a_to_b = 1;

    session
        .a
        .send_bundle(b"will be dropped", true)
        .await
        .unwrap();
    assert_eq!(session.a.tx_window_len(), 1);

    // Advance A's clock past the default 1.5s initial RTO (initial_srtt
    // 500ms × 3) so the next tick selects the entry for retransmit.
    session.a.clock.advance(Duration::from_secs(2));
    let (a_actions, _) = session.tick().await.unwrap();
    assert!(
        !a_actions.retransmits.is_empty(),
        "advancing past RTO must produce a retransmit on this tick"
    );

    // B should have received the retransmit.
    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(bundles.len(), 1, "B must receive the retransmit");
    assert_eq!(bundles[0].as_ref(), b"will be dropped");

    // B carries the ack back on its next send.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    let quiet = session.quiesce(Duration::from_millis(500)).await;
    assert!(quiet, "session must quiesce after ack clears tx_window");
}

/// Acks for a burst of three reliable sends drain the entire tx_window
/// in a single cumulative-ack pass once B carries the ack back.
#[tokio::test]
async fn cumulative_ack_drains_all_predecessors() {
    let session = LoopbackSession::connected(None).await.unwrap();

    session.a.send_bundle(b"one", true).await.unwrap();
    session.a.send_bundle(b"two", true).await.unwrap();
    session.a.send_bundle(b"three", true).await.unwrap();
    assert_eq!(session.a.tx_window_len(), 3);

    let bundles = session.b.recv_n_bundles(3, Duration::from_secs(1)).await;
    assert_eq!(bundles.len(), 3);
    assert_eq!(session.b.pending_acks_len(), 3);

    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    let quiet = session.quiesce(Duration::from_millis(500)).await;
    assert!(quiet, "cumulative ack must drain all predecessors");
    assert_eq!(session.a.tx_window_len(), 0);
}

/// Drop-everything policy in A→B direction + repeated clock advances:
/// the retransmit counter climbs past `MAX_RETRIES` and
/// `is_timed_out` flips true.
#[tokio::test]
async fn channel_times_out_after_max_retries_under_total_loss() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Drop more than MAX_RETRIES packets so every attempt dies.
    session.policy.lock().unwrap().drop_next.a_to_b = 1_000;

    session.a.send_bundle(b"black hole", true).await.unwrap();

    // Pump check_timeouts repeatedly with clock advances bigger than
    // the running RTO. The per-tick budget is `RETRANSMIT_BUDGET_PER_TICK`
    // so a single-entry TX window only needs as many ticks as there are
    // retry rounds.
    for _ in 0..(consts::MAX_RETRIES + 4) {
        // Past every plausible (backoff-doubled) RTO.
        session.a.clock.advance(Duration::from_secs(10));
        let _ = session.a.tick().await.unwrap();
    }

    assert!(
        session.a.channel.lock().unwrap().is_timed_out(),
        "channel must time out after exhausting MAX_RETRIES under total loss"
    );
}

/// Dropping A→B does not affect B→A. Symmetric proof that the policy
/// is per-direction, not shared.
#[tokio::test]
async fn drop_policy_is_per_direction() {
    let session = LoopbackSession::connected(None).await.unwrap();

    session.policy.lock().unwrap().drop_next.a_to_b = 1;

    session.a.send_bundle(b"dropped", true).await.unwrap();
    session.b.send_bundle(b"delivered", true).await.unwrap();

    let a_recv = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(a_recv.len(), 1);
    assert_eq!(a_recv[0].as_ref(), b"delivered");

    let b_recv = session.b.recv_n_bundles(1, Duration::from_millis(50)).await;
    assert!(b_recv.is_empty(), "A→B drop must not deliver to B");
}
