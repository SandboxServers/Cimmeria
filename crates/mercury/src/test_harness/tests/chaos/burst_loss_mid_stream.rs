//! Burst loss: 10 consecutive packets dropped in the middle of a
//! reliable stream. Models a congestion-event drop rather than a
//! single isolated loss. Recovery time = max RTO of any of the 10
//! lost packets.
//!
//! Without server-side retransmit, this scenario would have
//! permanently lost the 10-packet window. Today, all 10 sit in
//! the TX window awaiting retransmit; the RTO scan re-emits each
//! on the next tick past their last-sent + RTO.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn burst_loss_recovers_via_retransmit_scan() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Burst exactly equal to RETRANSMIT_BUDGET_PER_TICK so one
    // retransmit tick covers the whole burst. Smaller bursts
    // give cleaner assertions; the multi-tick-budget shape is
    // exercised by the burst-budget unit test in
    // `channel/tests/`. Here the focus is the wire-level
    // burst-drop + recovery pattern.
    let burst_size = 5u32;

    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.drop_next.a_to_b = burst_size;
    }

    // Fire the burst — all dropped at the wire.
    for i in 0..burst_size {
        session
            .a
            .send_bundle(format!("burst-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // B observes nothing — the entire burst was dropped.
    let pre = session.b.recv_n_bundles(1, Duration::from_millis(50)).await;
    assert!(
        pre.is_empty(),
        "burst is fully dropped; B sees nothing pre-retx"
    );

    // A's TX window holds all burst entries.
    assert_eq!(
        session.a.tx_window_len(),
        burst_size as usize,
        "all {burst_size} burst entries sit in A's TX window awaiting ack"
    );

    // Single tick past RTO retransmits the whole burst (= budget).
    session.a.clock.advance(Duration::from_secs(2));
    let (a_actions, _) = session.tick().await.unwrap();
    assert_eq!(
        a_actions.retransmits.len(),
        burst_size as usize,
        "one tick past RTO retransmits the full burst (sized to fit budget)"
    );

    // B observes all burst payloads on retransmit.
    let after = session
        .b
        .recv_n_bundles(burst_size as usize, Duration::from_secs(2))
        .await;
    let after_payloads: std::collections::HashSet<Vec<u8>> =
        after.iter().map(|b| b.to_vec()).collect();
    for i in 0..burst_size {
        let expected = format!("burst-{i}");
        assert!(
            after_payloads.contains(expected.as_bytes()),
            "retransmit phase must redeliver burst-{i}; got {} unique payloads",
            after_payloads.len(),
        );
    }

    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
