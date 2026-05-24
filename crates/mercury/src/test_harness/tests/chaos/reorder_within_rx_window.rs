//! Reorder within the RX window: packets in a small batch arrive
//! scrambled. Mercury's RX window (size = 64) is designed to
//! absorb this for fragmented payloads via the per-channel
//! `FragmentAssembler`.
//!
//! Pins the existing behavior of the assembler so a regression
//! that breaks out-of-order reassembly within the window would
//! fail this test.

use std::time::Duration;

use crate::packet::FRAGMENT_BODY_SIZE;
use crate::test_harness::invariants::all_safety_invariants;
use crate::test_harness::LoopbackSession;

#[tokio::test]
async fn reorder_within_rx_window_reassembles_correctly() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // 5-fragment bundle with `reorder_buffer_size = 5`. The harness
    // holds all 5 outbound fragments, then flushes them reversed.
    // The peer's FragmentAssembler must reassemble despite the
    // arrival permutation.
    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.reorder_buffer_size.a_to_b = 5;
    }

    let body: Vec<u8> = (0..(FRAGMENT_BODY_SIZE * 5))
        .map(|i| ((i * 13) & 0xFF) as u8)
        .collect();
    session.a.send_bundle(&body, true).await.unwrap();

    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(
        bundles.len(),
        1,
        "B must reassemble exactly one bundle despite reversed fragment arrival"
    );
    assert_eq!(
        bundles[0].as_ref(),
        body.as_slice(),
        "reassembled body must be byte-identical to the original despite reorder",
    );

    let channel = session.a.channel.lock().unwrap();
    all_safety_invariants(&channel);
}
