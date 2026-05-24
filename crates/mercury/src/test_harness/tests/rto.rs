//! Category 7 — adaptive RTO convergence on loopback.
//!
//! The RTO calculator is unit-tested in `channel/rto.rs` with synthetic
//! RTT samples. These tests drive it under real round-trips on the
//! harness's loopback (sub-millisecond RTT) and verify the floor
//! engages — guarding the regression shape called out in
//! `channel/rto.rs:286-292, 407-411` where removing the floor would
//! let the RTO collapse to zero.

use std::time::Duration;

use crate::test_harness::LoopbackSession;

/// Run N reliable round-trips on zero-latency loopback and verify the
/// RTO does not collapse below the configured floor.
#[tokio::test]
async fn rto_converges_to_floor_on_zero_latency_loopback() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Drive ~10 reliable round-trips — enough samples for the smoothing
    // to fully bias toward the observed RTT (which on real loopback is
    // sub-millisecond).
    for i in 0..10u32 {
        session
            .a
            .send_bundle(format!("rtt {i}").as_bytes(), true)
            .await
            .unwrap();
        let _ = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;
        session
            .b
            .send_bundle(format!("rtt-back {i}").as_bytes(), false)
            .await
            .unwrap();
        let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    }

    let rto = session.a.channel.lock().unwrap().rto().current();
    let min = session.a.channel.lock().unwrap().rto().config().min;
    assert!(
        rto >= min,
        "RTO must never drop below the configured floor (got {rto:?}, floor {min:?})",
    );
    assert!(
        rto > Duration::ZERO,
        "RTO must be positive — the floor exists to prevent collapse to zero",
    );
}

/// Smoke that the RTO state actually moves when fresh samples come in.
/// Together with `rto_converges_to_floor_on_zero_latency_loopback`
/// these two pin the "RTO updates but never collapses" invariant.
#[tokio::test]
async fn rto_updates_after_real_round_trips() {
    let session = LoopbackSession::connected(None).await.unwrap();

    let srtt_before = session.a.channel.lock().unwrap().rto().srtt();
    assert!(
        srtt_before.is_none(),
        "no samples yet → srtt should be unset",
    );

    session.a.send_bundle(b"one", true).await.unwrap();
    let _ = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    let _ = session.quiesce(Duration::from_millis(500)).await;

    let srtt_after = session.a.channel.lock().unwrap().rto().srtt();
    assert!(
        srtt_after.is_some(),
        "one clean round-trip must seed an SRTT sample (Karn's algorithm)",
    );
}
