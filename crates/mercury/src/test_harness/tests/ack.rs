//! Category 6 — ack aggregation.
//!
//! When B receives N reliable packets from A in a short window, B is
//! supposed to coalesce them into the next outbound packet's ack
//! footer rather than emit one ack per inbound. The unit tests in
//! `packet/build.rs` validate the footer build; these tests validate
//! the coalescing decision under realistic burst arrival patterns.

use std::time::Duration;

use crate::test_harness::LoopbackSession;

/// 5 reliable A→B packets accumulate in B's `pending_acks`. B's next
/// outbound packet carries all 5 ack ids piggybacked in one footer.
#[tokio::test]
async fn n_packets_in_window_produce_one_aggregated_ack_packet() {
    let session = LoopbackSession::connected(None).await.unwrap();

    for i in 0..5u32 {
        session
            .a
            .send_bundle(format!("burst {i}").as_bytes(), true)
            .await
            .unwrap();
    }

    // B receives the burst.
    let bundles = session.b.recv_n_bundles(5, Duration::from_secs(1)).await;
    assert_eq!(bundles.len(), 5);
    assert_eq!(
        session.b.pending_acks_len(),
        5,
        "B must owe A 5 acks before piggybacking",
    );

    // B sends one carrier packet. The piggyback footer should drain
    // all 5 pending acks.
    session.b.send_bundle(b"ack carrier", false).await.unwrap();
    assert_eq!(
        session.b.pending_acks_len(),
        0,
        "B's pending_acks must drain in one piggyback emit",
    );

    // A's receive of the carrier runs `process_ack_footer` over all 5
    // acks — each retires its packet, draining the tx_window in one pass.
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    let quiet = session.quiesce(Duration::from_millis(500)).await;
    assert!(quiet, "burst + single ack carrier must reach quiescence");
    assert_eq!(session.a.tx_window_len(), 0);
}

/// Two separate ack rounds — first round drains, second round
/// accumulates fresh. Proves pending_acks isn't leaking between rounds.
#[tokio::test]
async fn pending_acks_reset_between_rounds() {
    let session = LoopbackSession::connected(None).await.unwrap();

    session.a.send_bundle(b"r1-1", true).await.unwrap();
    session.a.send_bundle(b"r1-2", true).await.unwrap();
    let _ = session.b.recv_n_bundles(2, Duration::from_secs(1)).await;
    assert_eq!(session.b.pending_acks_len(), 2);

    session
        .b
        .send_bundle(b"round 1 carrier", false)
        .await
        .unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(session.b.pending_acks_len(), 0);

    // Round 2 — fresh pending_acks accumulation.
    session.a.send_bundle(b"r2-1", true).await.unwrap();
    let _ = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(
        session.b.pending_acks_len(),
        1,
        "round 2 acks must accumulate fresh, not on top of round 1",
    );
}

/// A backlog longer than one packet's ACK budget is delivered over
/// several carriers instead of overflowing one datagram (the 2026-09-29
/// 1488-byte stall). Each carrier takes the oldest ACKs up to the v1
/// budget; the rest stay queued until a later carrier, and the session
/// still quiesces.
#[tokio::test]
async fn ack_backlog_beyond_the_budget_drains_over_several_carriers() {
    use crate::encryption::{EncryptionVersion, MercuryEncryption};

    let enc = MercuryEncryption::from_session_key([0x3C; 32]);
    let session = LoopbackSession::connected(Some(enc)).await.unwrap();
    let budget = crate::packet::data_ack_budget(EncryptionVersion::V1);
    let burst = budget * 3 + 5;

    for i in 0..burst {
        session
            .a
            .send_bundle(format!("burst {i}").as_bytes(), true)
            .await
            .unwrap();
    }
    let bundles = session
        .b
        .recv_n_bundles(burst, Duration::from_secs(2))
        .await;
    assert_eq!(bundles.len(), burst);
    assert_eq!(session.b.pending_acks_len(), burst);

    let mut carriers = 0;
    while session.b.pending_acks_len() > 0 {
        let before = session.b.pending_acks_len();
        session.b.send_bundle(b"ack carrier", false).await.unwrap();
        carriers += 1;
        assert_eq!(
            before - session.b.pending_acks_len(),
            before.min(budget),
            "each carrier takes at most one budget of ACKs",
        );
        assert!(
            carriers <= 4,
            "backlog must drain in ceil(burst / budget) carriers"
        );
    }
    assert_eq!(carriers, 4);

    let _ = session
        .a
        .recv_n_bundles(carriers, Duration::from_secs(1))
        .await;
    assert!(
        session.quiesce(Duration::from_millis(500)).await,
        "every reliable packet is acked once the backlog drains",
    );
    assert_eq!(session.a.tx_window_len(), 0);
}
