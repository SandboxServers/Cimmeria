//! Unit tests for the argon2id gate in [`gated_check`]: at most two verifies run
//! at once, and the async runtime keeps running while one does. No DB access.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::task::JoinSet;

use super::{gated_check, PasswordCheck, ARGON2_GATE, IN_FLIGHT, PEAK};

/// Four checks at once on a multi-thread runtime never run more than two
/// verifies together. Without the gate, `PEAK` reaches three or four.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn gated_check_runs_at_most_two_at_once() {
    let mut set = JoinSet::new();
    for _ in 0..4 {
        set.spawn(gated_check(None, "pw".to_owned()));
    }
    while let Some(answer) = set.join_next().await {
        assert_eq!(answer.expect("join"), PasswordCheck::Invalid);
    }
    let peak = PEAK.load(Ordering::SeqCst);
    assert!((1..=2).contains(&peak), "peak was {peak}");
}

/// A ticker task keeps running while one check is in progress, so the verify
/// does not block the runtime's only thread. With `check_row` called inline the
/// ticker never runs and the count stays at zero.
#[tokio::test(flavor = "current_thread")]
async fn gated_check_leaves_the_async_runtime_free() {
    let ticks = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let ticker = {
        let ticks = Arc::clone(&ticks);
        let stop = Arc::clone(&stop);
        tokio::spawn(async move {
            while !stop.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        })
    };
    let answer = gated_check(None, "pw".to_owned()).await;
    stop.store(true, Ordering::SeqCst);
    ticker.await.expect("ticker");
    assert_eq!(answer, PasswordCheck::Invalid);
    assert!(ticks.load(Ordering::SeqCst) > 0, "runtime was blocked");
}

/// Dropping the caller while a verify runs does not free its permit: the permit
/// moves into the blocking task, so free plus running stays within the cap. With
/// the permit held by the async future, the abort frees it and free plus running
/// reaches three.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gated_check_dropped_caller_keeps_the_permit_until_the_verify_ends() {
    let handle = tokio::spawn(gated_check(None, "pw".to_owned()));
    while IN_FLIGHT.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    handle.abort();
    let _ = handle.await;
    let free = ARGON2_GATE.available_permits();
    let running = IN_FLIGHT.load(Ordering::SeqCst);
    assert!(free + running <= 2, "free {free} running {running}");
}
