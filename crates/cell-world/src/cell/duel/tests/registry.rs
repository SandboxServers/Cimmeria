//! `DuelRegistry` on an injected clock.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use crate::cell::duel::limits::{CHALLENGE_TIMEOUT, COUNTDOWN, PAIR_COOLDOWN};
use crate::cell::duel::{ChallengeRefusal, DuelRegistry, DuelState, ResponseRefusal};

#[test]
fn open_marks_both_busy_and_indexes_the_target() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    let p = r.open_challenge(1, 2, t0).unwrap();
    assert_eq!(
        (p.challenger, p.target, p.expires_at),
        (1, 2, t0 + CHALLENGE_TIMEOUT)
    );
    assert!(r.is_busy(1) && r.is_busy(2) && !r.is_busy(3));
    assert_eq!(r.pending_for(2), Some(&p));
    assert_eq!(r.pending_for(1), None, "only the target can answer");
}

#[test]
fn refusals_in_ledger_order() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    assert_eq!(
        r.open_challenge(1, 1, t0),
        Err(ChallengeRefusal::SelfChallenge)
    );
    r.open_challenge(1, 2, t0).unwrap();
    assert_eq!(
        r.open_challenge(1, 3, t0),
        Err(ChallengeRefusal::ChallengerBusy)
    );
    assert_eq!(
        r.open_challenge(3, 2, t0),
        Err(ChallengeRefusal::TargetBusy)
    );
    assert_eq!(
        r.open_challenge(3, 1, t0),
        Err(ChallengeRefusal::TargetBusy)
    );
}

/// Taking the pending challenge consumes it: the second take finds nothing,
/// and the challenger cannot take their own challenge.
#[test]
fn take_consumes_once() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    let p = r.open_challenge(1, 2, t0).unwrap();
    assert_eq!(r.take_pending_for(1, t0), Err(ResponseRefusal::NoPending));
    assert_eq!(r.take_pending_for(2, t0), Ok(p));
    assert_eq!(r.take_pending_for(2, t0), Err(ResponseRefusal::NoPending));
    assert!(!r.is_busy(1) && !r.is_busy(2));
}

/// A take at `expires_at` is refused as expired, removes the challenge and
/// starts the pair cooldown.
#[test]
fn take_after_expiry_is_expired_and_starts_cooldown() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    let p = r.open_challenge(1, 2, t0).unwrap();
    let t1 = t0 + CHALLENGE_TIMEOUT;
    assert_eq!(r.take_pending_for(2, t1), Err(ResponseRefusal::Expired(p)));
    assert_eq!(r.take_pending_for(2, t1), Err(ResponseRefusal::NoPending));
    assert_eq!(
        r.open_challenge(1, 2, t1),
        Err(ChallengeRefusal::PairCooldown)
    );
}

/// D-SS21: after a decline the same directed pair waits 60 s; the reverse
/// direction and other targets do not.
#[test]
fn decline_cooldown_is_per_directed_pair() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    let p = r.open_challenge(1, 2, t0).unwrap();
    r.take_pending_for(2, t0).unwrap();
    r.decline(&p, t0);
    let almost = t0 + PAIR_COOLDOWN - Duration::from_millis(1);
    assert_eq!(
        r.open_challenge(1, 2, almost),
        Err(ChallengeRefusal::PairCooldown)
    );
    let q = r
        .open_challenge(2, 1, almost)
        .expect("reverse direction is free");
    r.take_pending_for(1, almost).unwrap();
    r.decline(&q, almost);
    assert!(r.open_challenge(1, 3, almost).is_ok());
    r.take_pending_for(3, almost).unwrap();
    assert!(r.open_challenge(1, 2, t0 + PAIR_COOLDOWN).is_ok());
}

#[test]
fn expire_pending_returns_only_expired_and_frees_both() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    let p = r.open_challenge(1, 2, t0).unwrap();
    r.open_challenge(3, 4, t0 + Duration::from_secs(10))
        .unwrap();
    assert!(r
        .expire_pending(t0 + CHALLENGE_TIMEOUT - Duration::from_millis(1))
        .is_empty());
    assert_eq!(r.expire_pending(t0 + CHALLENGE_TIMEOUT), vec![p]);
    assert!(!r.is_busy(1) && !r.is_busy(2));
    assert!(r.is_busy(3) && r.is_busy(4));
}

/// An accepted duel is `StartPending` and busy; `can_harm` stays false
/// until SS-D2 engages it.
#[test]
fn start_duel_is_start_pending_and_cannot_harm() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    let p = r.open_challenge(1, 2, t0).unwrap();
    r.take_pending_for(2, t0).unwrap();
    let d = r.start_duel(&p, 7, Vector3::new(1.0, 2.0, 3.0), t0);
    assert_eq!(
        d.state,
        DuelState::StartPending {
            engage_at: t0 + COUNTDOWN
        }
    );
    assert!(r.is_busy(1) && r.is_busy(2));
    assert!(!r.can_harm(1, 2) && !r.can_harm(2, 1));
    assert_eq!(r.duel_of(2).and_then(|d| d.opponent_of(2)), Some(1));
    assert!(r
        .countdowns_due(t0 + COUNTDOWN - Duration::from_millis(1))
        .is_empty());
    assert_eq!(r.countdowns_due(t0 + COUNTDOWN), vec![d.duel_id]);
    assert!(r.end_duel(d.duel_id).is_some());
    assert!(!r.is_busy(1) && !r.is_busy(2) && r.is_idle());
}
