//! The duel tick on an injected clock: expiry (D-SS18) and cooldown pruning.
//! The countdown end is the engage, tested in `engage.rs`.

use crate::cell::duel::DuelResources;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use super::*;
use crate::cell::duel::limits::{CHALLENGE_TIMEOUT, PAIR_COOLDOWN};
use crate::cell::duel::tick::run_at;
use crate::cell::duel::ChallengeRefusal;
use crate::test_support::LogCapture;

const A: (u32, i32) = (A_EID, A_PID);
const B: (u32, i32) = (B_EID, B_PID);

/// Unanswered for 30 s: the sweep removes the challenge, tells both 878,
/// and starts the pair cooldown. One millisecond earlier it does nothing.
#[tokio::test]
async fn unanswered_challenge_expires_and_tells_both() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, B, t0).await;
    drain(&mut rx);

    run_at(
        &tx,
        &mut mgr,
        t0 + CHALLENGE_TIMEOUT - Duration::from_millis(1),
    )
    .await;
    assert!(drain(&mut rx).is_empty());
    assert!(mgr.resources.duels().pending_for(B_PID).is_some());

    run_at(&tx, &mut mgr, t0 + CHALLENGE_TIMEOUT).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert!(mgr.resources.duels().pending_for(B_PID).is_none());
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.challenge_expired")
            && c.has_field("reason", "no_answer")));
    assert_eq!(
        mgr.resources.duels_mut().open_challenge(
            A_PID,
            B_PID,
            t0 + CHALLENGE_TIMEOUT + PAIR_COOLDOWN / 2
        ),
        Err(ChallengeRefusal::PairCooldown)
    );
}

/// The wall-clock tick must not short-circuit past a stored cooldown: with
/// no challenge or duel left, `run` still prunes an expired cooldown.
#[tokio::test]
async fn wall_clock_tick_prunes_an_expired_cooldown() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let past = Instant::now()
        .checked_sub(PAIR_COOLDOWN + Duration::from_secs(1))
        .expect("clock older than 61 s");
    challenge(&mut mgr, &tx, A, B, past).await;
    crate::cell::duel::response::handle_at(B_EID, &[0], &tx, &mut mgr, past).await;
    drain(&mut rx);
    assert_eq!(mgr.resources.duels().cooldown_count(), 1);

    crate::cell::duel::tick::run(&tx, &mut mgr).await;
    assert_eq!(mgr.resources.duels().cooldown_count(), 0);
    assert!(mgr.resources.duels().is_idle());
}
