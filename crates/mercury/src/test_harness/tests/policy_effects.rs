//! Tests pinning the observable effects of each [`NetworkPolicy`]
//! field. Sibling to the per-category test files; lives here
//! because the assertions are about policy mechanics rather than
//! channel-protocol behavior.
//!
//! The drop policies (`drop_next`, `drop_at_send_count`) are
//! exercised by the reliable / fragment tests where loss is part of
//! the scenario. Latency is exercised by the fragment-with-latency
//! test. This file covers the remaining two: `duplicate_every` and
//! `reorder_pairs`.

use std::time::Duration;

use crate::test_harness::LoopbackSession;

/// `duplicate_every = Some(2)` on A→B with 4 unreliable sends
/// produces 6 wire emits (sends 2 and 4 are duplicated). The
/// harness recv pump pushes every arrival to the inbox (it does NOT
/// dedup non-fragmented packets via the RX window — a known
/// simplification documented in the harness module doc), so the
/// inbox order matches the wire order exactly: send-1, send-2,
/// send-2-dup, send-3, send-4, send-4-dup.
#[tokio::test]
async fn duplicate_every_doubles_every_nth_send() {
    let session = LoopbackSession::connected(None).await.unwrap();

    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.duplicate_every.a_to_b = Some(2);
    }

    for i in 0..4u32 {
        session
            .a
            .send_bundle(format!("dup-{i}").as_bytes(), false)
            .await
            .unwrap();
    }

    let bundles = session.b.recv_n_bundles(6, Duration::from_secs(1)).await;
    assert_eq!(
        bundles.len(),
        6,
        "Some(2) doubles sends 2 and 4 → 4 originals + 2 duplicates = 6 wire arrivals"
    );

    // Inbox arrival order = wire order = OS UDP loopback order
    // (preserved per-source-per-destination).
    let expected: [&[u8]; 6] = [b"dup-0", b"dup-1", b"dup-1", b"dup-2", b"dup-3", b"dup-3"];
    for (i, (actual, expected)) in bundles.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            actual.as_ref(),
            *expected,
            "inbox[{i}] = {actual:?}, expected {expected:?} \
             (wire order = send order + per-Nth duplicates inserted in place)",
        );
    }

    // The duplicate counter should have cycled exactly twice (sends
    // 2 and 4 triggered duplication, resetting to 0 each time).
    let dup_count = session.policy.lock().unwrap().duplicate_count.a_to_b;
    assert_eq!(
        dup_count, 0,
        "duplicate_count must end at 0 after two full Some(2) cycles"
    );
}

/// `duplicate_every = Some(3)` with 5 unreliable sends triggers
/// exactly ONE duplication (on the 3rd send) and leaves the
/// counter at 2 (incremented twice after the reset, by sends 4 and
/// 5). Proves the counter is per-direction and survives between
/// sends.
#[tokio::test]
async fn duplicate_every_n_3_partial_cycle_leaves_counter_partway() {
    let session = LoopbackSession::connected(None).await.unwrap();

    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.duplicate_every.a_to_b = Some(3);
    }

    for i in 0..5u32 {
        session
            .a
            .send_bundle(format!("n3-{i}").as_bytes(), false)
            .await
            .unwrap();
    }

    let bundles = session.b.recv_n_bundles(6, Duration::from_secs(1)).await;
    assert_eq!(
        bundles.len(),
        6,
        "Some(3) with 5 sends → 5 originals + 1 duplicate of the 3rd = 6 wire arrivals",
    );

    let expected: [&[u8]; 6] = [b"n3-0", b"n3-1", b"n3-2", b"n3-2", b"n3-3", b"n3-4"];
    for (i, (actual, expected)) in bundles.iter().zip(expected.iter()).enumerate() {
        assert_eq!(actual.as_ref(), *expected, "inbox[{i}] mismatch");
    }

    let dup_count = session.policy.lock().unwrap().duplicate_count.a_to_b;
    assert_eq!(
        dup_count, 2,
        "after 5 sends with Some(3): cycle reset at send 3, counter incremented to 2 after sends 4+5",
    );
}

/// `reorder_pairs = true` on A→B with 4 sends produces wire order
/// `(2, 1, 4, 3)` — the policy holds the 1st packet of each pair
/// in the reorder buffer and ships them swapped when the 2nd
/// arrives. The harness recv pump pushes arrivals to the inbox in
/// wire order (no RX-window reordering for non-fragmented
/// packets — known harness simplification), so the inbox order
/// matches the wire reorder. The test pins the wire effect of the
/// policy; reliable RX-window re-sequencing is a separate Channel
/// concern not exercised by the harness's non-fragmented path.
#[tokio::test]
async fn reorder_pairs_swaps_adjacent_packets_on_wire() {
    let session = LoopbackSession::connected(None).await.unwrap();

    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.reorder_pairs.a_to_b = true;
    }

    // Send 4 unreliable bundles. The reorder buffer holds the 1st,
    // then ships (2, 1) when the 2nd arrives. Same for (4, 3).
    let payloads: [&[u8]; 4] = [b"one", b"two", b"three", b"four"];
    for p in payloads.iter() {
        session.a.send_bundle(p, false).await.unwrap();
    }

    let bundles = session.b.recv_n_bundles(4, Duration::from_secs(1)).await;
    assert_eq!(bundles.len(), 4, "B must deliver all 4 bundles");

    // Wire-order = (2nd, 1st, 4th, 3rd). The inbox preserves this
    // because the recv pump pushes in arrival order without
    // re-sequencing. Test pins the policy's wire-level effect.
    let expected_wire_order: [&[u8]; 4] = [b"two", b"one", b"four", b"three"];
    for (i, (actual, expected)) in bundles.iter().zip(expected_wire_order.iter()).enumerate() {
        assert_eq!(
            actual.as_ref(),
            *expected,
            "wire-order[{i}] = {actual:?}, expected {expected:?} — reorder_pairs swaps adjacent pairs",
        );
    }
}

/// Odd-count send with `reorder_pairs = true` — the unpaired final
/// packet is held in the reorder buffer indefinitely. Verifies that
/// "wire reorder needs pairs to complete" is the documented
/// limitation, not a bug.
#[tokio::test]
async fn reorder_pairs_holds_unpaired_final_send_indefinitely() {
    let session = LoopbackSession::connected(None).await.unwrap();

    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.reorder_pairs.a_to_b = true;
    }

    // Send 3 packets. The 1st pairs with the 2nd (wire order: 2, 1).
    // The 3rd is held — never shipped because no 4th arrived.
    session.a.send_bundle(b"a", false).await.unwrap();
    session.a.send_bundle(b"b", false).await.unwrap();
    session.a.send_bundle(b"c", false).await.unwrap();

    // B should receive 2 bundles (the pair). The 3rd is stuck in
    // A's reorder buffer.
    let bundles = session
        .b
        .recv_n_bundles(3, Duration::from_millis(300))
        .await;
    assert_eq!(
        bundles.len(),
        2,
        "only the completed pair (2, 1) is shipped; the unpaired 3rd is held"
    );

    let held = session.policy.lock().unwrap().reorder_held.a_to_b.is_some();
    assert!(held, "the 3rd packet must still be in the reorder buffer");
}
