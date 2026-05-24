//! The canonical chaos scenario: a single mid-stream packet drop on
//! a long-running reliable session, validating that the server-side
//! RTO + retransmit pipeline recovers the channel.
//!
//! Models the real player session from
//! `debug/lomiada-broke-in-hallway02/`: ~6 minutes of clean play,
//! then a transatlantic UDP drop of server-sent packet `#1148`,
//! followed by 210 buffered packets above the gap. Pre-retransmit
//! work, this would have hung until the 60s inactivity timer
//! reaped the channel. After the work in #308 + #354's
//! deferred-send queue, the channel recovers in N ticks.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn lomiada_single_packet_gap_recovers_via_retransmit() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // Drop the 3rd reliable send mid-stream. With the per-tick
    // retransmit budget of `RETRANSMIT_BUDGET_PER_TICK = 5`, the
    // dropped entry must land inside the first-5 slots of the TX
    // window for a single-tick retransmit. A more comprehensive
    // burst-of-drops scenario lives in `burst_loss_mid_stream`.
    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.drop_at_send_count.a_to_b = Some(3);
    }

    // Send 10 reliable packets. The 3rd never arrives.
    for i in 0..10u32 {
        session
            .a
            .send_bundle(format!("packet-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // B receives 9 of 10 in arrival order (the 3rd is missing).
    let bundles_before_retx = session.b.recv_n_bundles(9, Duration::from_secs(1)).await;
    assert_eq!(
        bundles_before_retx.len(),
        9,
        "B must receive 9 of 10 sends (the 3rd was dropped pre-retx)"
    );
    let payloads_before: std::collections::HashSet<Vec<u8>> =
        bundles_before_retx.iter().map(|b| b.to_vec()).collect();
    let missing = b"packet-2".to_vec(); // 0-indexed: the 3rd send is index 2
    assert!(
        !payloads_before.contains(&missing),
        "packet-2 must be absent pre-retransmit (proof the drop fired)"
    );

    // A's TX window still holds all 10 (no acks back yet).
    assert_eq!(
        session.a.tx_window_len(),
        10,
        "all 10 reliable sends sit in A's TX window awaiting ack"
    );

    // Advance A's clock past RTO so the retransmit scan fires.
    // First 5 TX-window entries retransmit on this tick (per
    // `RETRANSMIT_BUDGET_PER_TICK`) — the dropped seq-2 is in
    // that batch.
    session.a.clock.advance(Duration::from_secs(2));
    let (a_actions, _) = session.tick().await.unwrap();
    assert!(
        !a_actions.retransmits.is_empty(),
        "RTO past + tick must produce at least one retransmit"
    );

    // B observes the retransmits (the harness pushes each wire
    // arrival to the inbox; dedup happens at the channel layer in
    // production but the harness is wire-faithful).
    let after_retx = session
        .b
        .recv_n_bundles(a_actions.retransmits.len(), Duration::from_secs(1))
        .await;

    // The originally-missing payload must be among what B received
    // across both phases.
    let mut all_received: std::collections::HashSet<Vec<u8>> = payloads_before;
    for b in after_retx.iter() {
        all_received.insert(b.to_vec());
    }
    assert!(
        all_received.contains(&missing),
        "the originally-dropped packet must arrive via retransmit; \
         distinct payloads seen by B: {}",
        all_received.len(),
    );

    // Safety: channel state stays inside the spec'd bounds.
    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
