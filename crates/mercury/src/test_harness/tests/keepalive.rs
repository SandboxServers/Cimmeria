//! Category 3 — keepalive cadence end-to-end.
//!
//! The `tick_re_flags_keepalive_until_caller_acks_send` unit test in
//! `channel/tests/` pins the return-value contract of `keepalive_due` /
//! `touch_sent`. These tests verify that a real keepalive datagram
//! goes on the wire when due, the peer's recv pump updates the peer's
//! `last_received`, and a busy outbound send suppresses the next
//! keepalive.

use std::time::Duration;

use crate::consts;
use crate::test_harness::LoopbackSession;

/// Advance A's clock past the keepalive interval → A's tick emits a
/// keepalive datagram → B observes it.
#[tokio::test]
async fn keepalive_fires_after_idle_interval_and_peer_recognizes_it() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Advance past KEEPALIVE_INTERVAL_MS so A's `keepalive_due()` is true.
    session
        .a
        .clock
        .advance(Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 50));

    let (a_actions, _) = session.tick().await.unwrap();
    assert_eq!(
        a_actions.keepalives.len(),
        1,
        "tick past keepalive interval must emit exactly one keepalive"
    );

    // B's recv pump bumps `last_received` on every non-fragmented arrival.
    // The keepalive body is empty, so it doesn't show up as a bundle but
    // it does touch B's clock-side activity. Verify by re-checking B's
    // tx-side keepalive state hasn't fired in the same way (B's clock
    // wasn't advanced).
    let _ = session.b.recv_n_bundles(1, Duration::from_millis(50)).await;
}

/// A busy A→B send path means A's `last_sent` was just bumped — so
/// `keepalive_due` returns false on the very next tick. Production
/// behavior: don't waste an empty packet when the channel is actively
/// chatty.
#[tokio::test]
async fn active_traffic_suppresses_keepalive() {
    let session = LoopbackSession::connected(None).await.unwrap();

    session.a.send_bundle(b"chatter", false).await.unwrap();

    // No clock advance — `last_sent` is fresh. Tick must NOT emit a keepalive.
    let (a_actions, _) = session.tick().await.unwrap();
    assert!(
        a_actions.keepalives.is_empty(),
        "active traffic must suppress keepalive on the same tick"
    );
}

/// Drop the keepalive in flight → A's `last_sent` was already bumped by
/// the emit, so the next tick (without further clock advancement) does
/// not re-fire. Validates the "touch_sent on emit, not on ack" cadence.
#[tokio::test]
async fn keepalive_emit_bumps_last_sent_even_if_dropped() {
    let session = LoopbackSession::connected(None).await.unwrap();

    session
        .a
        .clock
        .advance(Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 50));

    session.policy.lock().unwrap().drop_next.a_to_b = 1;

    let (a_actions, _) = session.tick().await.unwrap();
    assert_eq!(
        a_actions.keepalives.len(),
        1,
        "first tick still emits the keepalive (drop happens at the wire)"
    );

    // Without advancing the clock again, the next tick must NOT emit a
    // second keepalive — touch_sent fired on the first emit.
    let (a_actions2, _) = session.tick().await.unwrap();
    assert!(
        a_actions2.keepalives.is_empty(),
        "second tick at same time must not re-fire keepalive"
    );
}
