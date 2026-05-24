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
/// keepalive datagram → B's recv pump observes it and bumps B's
/// `last_received`.
#[tokio::test]
async fn keepalive_fires_after_idle_interval_and_peer_recognizes_it() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Snapshot B's `last_received` BEFORE the keepalive flies.
    let b_last_received_before = session.b.channel.lock().unwrap().last_received;

    // Advance A's clock past KEEPALIVE_INTERVAL_MS so A's tick
    // schedules the keepalive.
    session
        .a
        .clock
        .advance(Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 50));

    // Also advance B's clock by a small amount so the
    // `touch_received` call B's recv pump makes when the keepalive
    // arrives uses a strictly-later `Instant` than the handshake
    // touch. Without this, B's `Clock::now()` returns the same
    // value both times and the `before != after` assertion below
    // could pass even on regressions where the keepalive packet was
    // silently dropped (because both touches would read the same
    // baseline Instant).
    session.b.clock.advance(Duration::from_millis(100));

    let (a_actions, _) = session.tick().await.unwrap();
    assert_eq!(
        a_actions.keepalives.len(),
        1,
        "tick past keepalive interval must emit exactly one keepalive"
    );

    // Drain the inbox so we know B's recv pump processed the arrival
    // (empty body → empty bundle pushed, then drained here).
    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(
        bundles.len(),
        1,
        "B's recv pump must observe the keepalive packet on the wire"
    );
    assert!(
        bundles[0].is_empty(),
        "keepalive body is empty (the wake-up is the signal, not the payload)"
    );

    // After the keepalive landed, B's recv pump called
    // `touch_received` which reads B's (now-advanced) clock. The
    // resulting `last_received` Instant must be strictly after the
    // pre-advance snapshot — proving the keepalive was both
    // observed AND processed (not silently dropped during decrypt /
    // parse / dispatch).
    let b_last_received_after = session.b.channel.lock().unwrap().last_received;
    assert!(
        b_last_received_after > b_last_received_before,
        "B's last_received must advance after keepalive arrival \
         (before={b_last_received_before:?}, after={b_last_received_after:?}) — \
         a silent drop in the recv pipeline would leave last_received unchanged",
    );
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
