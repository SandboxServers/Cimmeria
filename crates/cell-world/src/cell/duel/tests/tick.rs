//! The duel tick on an injected clock: expiry (D-SS18) and the interim
//! countdown end.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use super::*;
use crate::cell::duel::limits::{CHALLENGE_TIMEOUT, COUNTDOWN, PAIR_COOLDOWN};
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
    assert!(mgr.duels.pending_for(B_PID).is_some());

    run_at(&tx, &mut mgr, t0 + CHALLENGE_TIMEOUT).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert!(mgr.duels.pending_for(B_PID).is_none());
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.challenge_expired")
            && c.has_field("reason", "no_answer")));
    assert_eq!(
        mgr.duels
            .open_challenge(A_PID, B_PID, t0 + CHALLENGE_TIMEOUT + PAIR_COOLDOWN / 2),
        Err(ChallengeRefusal::PairCooldown)
    );
}

/// Until SS-D2 engages duels, the end of the countdown aborts the duel with
/// 878 so neither player stays busy.
#[tokio::test]
async fn countdown_end_aborts_until_ss_d2() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, B, t0).await;
    crate::cell::duel::response::handle_at(B_EID, &[1], &tx, &mut mgr, t0).await;
    drain(&mut rx);

    run_at(&tx, &mut mgr, t0 + COUNTDOWN - Duration::from_millis(1)).await;
    assert!(drain(&mut rx).is_empty());
    assert!(mgr.duels.duel_of(A_PID).is_some());

    run_at(&tx, &mut mgr, t0 + COUNTDOWN).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert!(!mgr.duels.is_busy(A_PID) && !mgr.duels.is_busy(B_PID));
    assert!(mgr.duels.is_idle());
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.aborted")
            && c.has_field("reason", "engage_not_implemented")));
}
