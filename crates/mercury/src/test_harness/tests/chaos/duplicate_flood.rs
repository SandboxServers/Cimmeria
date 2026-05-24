//! Duplicate flood: the same packet arrives at the peer 5 times
//! back-to-back. Production duplication can come from over-eager
//! retransmits, NAT path duplication, or buggy proxies. This
//! scenario uses the one-shot `duplicate_next_count = 4` primitive
//! to emit 5 copies (1 original + 4 dups) of a single send.
//!
//! For the harness with naive non-fragmented delivery, the peer's
//! recv pump pushes every copy to the inbox. The Channel's RX
//! window (when wired) would dedup; here we assert wire-level
//! observability — the harness DID emit 5 copies and the policy
//! counters reflect that.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn duplicate_flood_emits_n_plus_one_copies_on_wire() {
    let session = LoopbackSession::connected(None).await.unwrap();

    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        // One-shot: next send emits 4 extra copies (= 5 total).
        policy.duplicate_next_count.a_to_b = 4;
    }

    session.a.send_bundle(b"flooded-once", false).await.unwrap();

    // Receive all 5 wire arrivals.
    let bundles = session.b.recv_n_bundles(5, Duration::from_secs(1)).await;
    assert_eq!(
        bundles.len(),
        5,
        "duplicate_next_count=4 must emit exactly 5 wire copies of the single send"
    );
    for (i, b) in bundles.iter().enumerate() {
        assert_eq!(
            b.as_ref(),
            b"flooded-once",
            "copy {i}: every duplicate must be byte-identical to the original"
        );
    }

    // The one-shot counter must have reset to 0 after firing.
    let next = session.policy.lock().unwrap().duplicate_next_count.a_to_b;
    assert_eq!(
        next, 0,
        "one-shot duplicate counter must clear after firing"
    );

    // Send a follow-up; no more duplication should fire.
    session.a.send_bundle(b"single", false).await.unwrap();
    let after = session.b.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert_eq!(after.len(), 1, "follow-up send must emit exactly one copy");

    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
