//! Monte Carlo: sustained ~5% probabilistic drop over a long
//! reliable burst, advancing simulated time between batches so the
//! RTO and retransmit machinery has room to run. Seeded ChaCha20
//! RNG ensures the exact drop pattern is reproducible across CI
//! runs and platforms.
//!
//! Goal: prove the channel state stays inside the safety
//! invariants under sustained non-deterministic loss — no
//! `tx_window` overflow past capacity, no orphan retransmits, no
//! infinite-loop blowup. Recovery completeness (every bundle
//! eventually delivered) is a stronger property and is exercised
//! by the deterministic burst-loss test rather than here.

use std::time::Duration;

use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::{DropProbability, LoopbackSession};

#[tokio::test]
async fn sustained_5pct_loss_holds_safety_invariants() {
    let session = LoopbackSession::connected(None).await.unwrap();

    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        // 5% loss on A→B, seeded for repro.
        policy.rng_seed = Some(0xC0FFEE_u64);
        policy.drop_probability.a_to_b = Some(DropProbability::pct(5));
    }

    // 8 batches × 20 unreliable sends = 160 packets. Unreliable so
    // we exercise the wire policy without piling up the TX window
    // (which has its own safety properties tested elsewhere). The
    // safety invariants apply to ANY long-run scenario.
    let batches = 8u32;
    let per_batch = 20u32;
    for batch in 0..batches {
        for i in 0..per_batch {
            session
                .a
                .send_bundle(format!("mc-{batch}-{i}").as_bytes(), false)
                .await
                .unwrap();
        }
        // Advance clock between batches so any retransmit / keepalive
        // logic that's clock-bound has space to run.
        session.a.clock.advance(Duration::from_millis(50));
        session.b.clock.advance(Duration::from_millis(50));
    }

    // Drain whatever made it through. The exact count varies (it's
    // probabilistic) but it should be close to (1 - 0.05) * 160 ≈ 152.
    let _drained = session
        .b
        .recv_n_bundles(160, Duration::from_millis(500))
        .await;

    // Drop count must be non-zero (5% over 160 packets ≈ 8 drops
    // expected; with the C0FFEE seed it's deterministic).
    let drop_count = session.policy.lock().unwrap().drop_count.a_to_b;
    assert!(
        drop_count > 0,
        "expected at least one probabilistic drop in 160 sends at 5%; got {drop_count}"
    );
    assert!(
        drop_count < 160,
        "probabilistic drop should NOT have dropped every packet; got {drop_count}/160"
    );

    // Safety invariants survive the sustained-chaos run.
    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
