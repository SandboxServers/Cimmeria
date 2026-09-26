//! The canonical chaos scenario: a single mid-stream packet drop on
//! a long-running reliable session, validating that the server-side
//! RTO + retransmit pipeline recovers the channel.
//!
//! Models the real player session from
//! `debug/lomiada-broke-in-hallway02/`: ~6 minutes of clean play,
//! then a transatlantic UDP drop of server-sent packet `#1148`,
//! followed by 210 buffered packets above the gap. Pre-retransmit
//! work, this would have hung until the 60s inactivity timer
//! reaped the channel. With server-side retransmit + the
//! deferred-send queue now in place, the channel recovers in N
//! ticks.

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

    // Nine of the ten reach B, but only the two ahead of the gap are
    // delivered. packet-3..packet-9 wait in B's receive window behind the
    // missing packet-2, as the SGW client's `queueAckForPacket` holds them
    // (NA38). The generous wait lets all nine datagrams land first.
    let bundles_before_retx = session
        .b
        .recv_n_bundles(10, Duration::from_millis(300))
        .await;
    let before: Vec<&[u8]> = bundles_before_retx.iter().map(|b| b.as_ref()).collect();
    assert_eq!(
        before,
        vec![b"packet-0".as_slice(), b"packet-1".as_slice()],
        "only the packets ahead of the gap may be delivered before the retransmit"
    );
    assert_eq!(
        session.b.channel.lock().unwrap().rx_window.len(),
        8,
        "the gap slot plus the seven packets buffered behind it"
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

    // The retransmitted packet-2 fills the gap and releases everything
    // behind it, in sequence order. The retransmitted copies of packets B
    // already had are dropped as duplicates, so each payload is
    // delivered exactly once.
    let after_retx = session.b.recv_n_bundles(8, Duration::from_secs(1)).await;
    let after: Vec<Vec<u8>> = after_retx.iter().map(|b| b.to_vec()).collect();
    let expected: Vec<Vec<u8>> = (2..10u32)
        .map(|i| format!("packet-{i}").into_bytes())
        .collect();
    assert_eq!(
        after, expected,
        "the retransmit must release packet-2..packet-9 in sequence order"
    );
    let extra = session
        .b
        .recv_n_bundles(1, Duration::from_millis(100))
        .await;
    assert!(
        extra.is_empty(),
        "retransmitted duplicates must not be delivered twice; got {extra:?}"
    );

    // Safety: channel state stays inside the spec'd bounds.
    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
