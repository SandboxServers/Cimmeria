//! The Lomiada gap again, with the receiver acking the way the real SGW
//! client does: promptly, and for every packet it buffers behind the gap.
//!
//! `lomiada_single_packet_gap` never lets B send before A's retransmit
//! fires, so A sees no ACK at all until the resend. The real client sends
//! about six packets a second even when idle, and every one carries the
//! ACKs it owes. In `debug/lomiada-broke-in-hallway02/` it acked #1149 about
//! 100 ms after #1148 went missing, then #1150..#1155 in one footer, and so
//! on up to #1358. #1148 was never acked.
//!
//! When the ACK path was a cumulative drain, the ACK for #1149 retired #1148
//! and it was never resent: the client's reliable stream stayed wedged for
//! the rest of the session, which is how a player stops seeing anyone new
//! arrive. This scenario fails on that code: A's TX window is empty after
//! the carrier, the tick resends nothing, and B never gets packet-2.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn gap_acked_past_by_a_prompt_client_is_still_resent() {
    let session = LoopbackSession::connected(None).await.unwrap();
    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.drop_at_send_count.a_to_b = Some(3);
    }

    for i in 0..10u32 {
        session
            .a
            .send_bundle(format!("packet-{i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // B delivers what is ahead of the gap and buffers the rest.
    let before = session
        .b
        .recv_n_bundles(10, Duration::from_millis(300))
        .await;
    assert_eq!(
        before.len(),
        2,
        "only packet-0 and packet-1 are deliverable"
    );
    assert_eq!(
        session.b.pending_acks_len(),
        9,
        "B owes an ack for every packet that arrived, buffered ones included"
    );

    // B acks right away, long before A's RTO, as the real client does.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let carrier = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(carrier.len(), 1);

    {
        let channel = session.a.channel.lock().unwrap();
        assert_eq!(
            channel.tx_window.len(),
            1,
            "the ACKs of the nine packets B got must not retire the one it did not"
        );
        assert_eq!(channel.tx_holes, 1, "A knows B is missing a packet");
        all_safety_invariants(&channel);
    }

    // A's retransmit scan resends the lost packet once its RTO passes.
    session.a.clock.advance(Duration::from_secs(2));
    let (a_actions, _) = session.tick().await.unwrap();
    assert_eq!(a_actions.retransmits.len(), 1, "exactly the lost packet");

    let after = session.b.recv_n_bundles(8, Duration::from_secs(1)).await;
    let after: Vec<Vec<u8>> = after.iter().map(|b| b.to_vec()).collect();
    let expected: Vec<Vec<u8>> = (2..10u32)
        .map(|i| format!("packet-{i}").into_bytes())
        .collect();
    assert_eq!(
        after, expected,
        "the resend must release packet-2..packet-9 in order"
    );

    // B acks the resend; the hole closes and nothing is left outstanding.
    session
        .b
        .send_bundle(b"ack carrier 2", false)
        .await
        .unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    let channel = session.a.channel.lock().unwrap();
    assert!(channel.tx_window.is_empty());
    assert_eq!(channel.open_tx_hole(), None);
}

/// The watchdog's view of the same stall: if the resend never lands, A
/// reports the hole once it has been open past the threshold.
#[tokio::test]
async fn unrecovered_gap_is_reported_by_the_tx_hole_watchdog() {
    let session = LoopbackSession::connected(None).await.unwrap();
    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.drop_at_send_count.a_to_b = Some(1);
    }
    for i in 0..3u32 {
        session
            .a
            .send_bundle(format!("packet-{i}").as_bytes(), true)
            .await
            .unwrap();
    }
    let _ = session
        .b
        .recv_n_bundles(3, Duration::from_millis(300))
        .await;
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;

    // Every resend is dropped too.
    session.policy.lock().unwrap().drop_next.a_to_b = 100;
    session
        .a
        .clock
        .advance(Duration::from_millis(crate::consts::TX_HOLE_WARN_MS + 100));
    let actions = session.a.tick().await.unwrap();
    let stall = actions.tx_hole.expect("hole open past the threshold");
    assert_eq!(stall.highest_acked, stall.seq + 2);
    assert!(stall.first_warning);
}
