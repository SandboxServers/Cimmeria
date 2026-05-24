//! Concurrent-send tests that exercise the harness under genuine
//! parallelism (vs the cooperative `tokio::select!` shape that the
//! default `#[tokio::test]` current-thread runtime gives).
//!
//! Tests here use `#[tokio::test(flavor = "multi_thread",
//! worker_threads = 2)]` because they assert behavior that only
//! shows up when two `LoopbackPeer::send_bundle` calls can be
//! genuinely in flight at the same instant. The default
//! current-thread runtime serialises them via the executor.

use std::sync::Arc;
use std::time::Duration;

use crate::test_harness::LoopbackSession;

/// Two tasks on A spawn simultaneous reliable sends. The shared
/// `Channel` mutex serialises seq assignment + tx_window insertion;
/// the shared `socket` serialises wire emits. All bundles arrive
/// at B with distinct sequence numbers and no state corruption.
///
/// Multi-thread runtime opt-in because on the default
/// current-thread runtime, two `tokio::spawn`'d send tasks would
/// just alternate via the executor's scheduler — they'd never be
/// genuinely simultaneous, and a missing-lock regression wouldn't
/// reproduce. With `worker_threads = 2` both tasks can be on the
/// CPU at once, so a lock-discipline regression actually fires.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_sends_from_two_tasks_do_not_corrupt_state() {
    let session = Arc::new(LoopbackSession::connected(None).await.unwrap());

    // The handshake bumped send_count by 1 on each direction. Reset
    // so the test's assertion (send_count == sends_per_task * 2)
    // is meaningful against the test's own sends only.
    session.policy.lock().unwrap().reset_counters();

    let sends_per_task = 20u32;
    let session_for_left = session.clone();
    let session_for_right = session.clone();

    let left = tokio::spawn(async move {
        for i in 0..sends_per_task {
            session_for_left
                .a
                .send_bundle(format!("left-{i}").as_bytes(), true)
                .await
                .unwrap();
        }
    });
    let right = tokio::spawn(async move {
        for i in 0..sends_per_task {
            session_for_right
                .a
                .send_bundle(format!("right-{i}").as_bytes(), true)
                .await
                .unwrap();
        }
    });

    // Time-bounded wait — if a lock regression deadlocks one task,
    // we fail the test instead of wedging the suite.
    tokio::time::timeout(Duration::from_secs(5), async {
        left.await.unwrap();
        right.await.unwrap();
    })
    .await
    .expect("both send tasks must complete within timeout");

    let total = (sends_per_task * 2) as usize;
    let bundles = session
        .b
        .recv_n_bundles(total, Duration::from_secs(2))
        .await;
    assert_eq!(
        bundles.len(),
        total,
        "B must receive every bundle from both tasks (no losses, no corruption)"
    );

    // Every payload from both tasks should be represented. Order is
    // not guaranteed (the two tasks interleave at the OS-thread
    // level), but the *set* must be complete.
    let mut seen: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    for b in &bundles {
        seen.insert(b.to_vec());
    }
    for i in 0..sends_per_task {
        assert!(
            seen.contains(format!("left-{i}").as_bytes()),
            "missing left-{i} from delivered bundle set"
        );
        assert!(
            seen.contains(format!("right-{i}").as_bytes()),
            "missing right-{i} from delivered bundle set"
        );
    }

    // The session must still be functional after the burst — quiesce
    // proves the tx_window cleared and no acks are stuck.
    {
        let policy = session.policy.lock().unwrap();
        // Sanity: send_count must equal the actual number of A→B
        // sends (40 bundle calls). This catches a race where the
        // counter would have been bumped under a non-atomic guard.
        assert_eq!(
            policy.send_count.a_to_b, total as u32,
            "send_count must equal the sum of both tasks' send_bundle calls",
        );
    }

    session
        .b
        .send_bundle(b"final ack carrier", false)
        .await
        .unwrap();
    let _ = session.a.recv_n_bundles(1, Duration::from_secs(1)).await;
    assert!(
        session.quiesce(Duration::from_secs(1)).await,
        "session must reach quiescence after the carrier delivers the cumulative ack",
    );
}
