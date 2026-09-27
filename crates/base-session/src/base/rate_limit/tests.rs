//! Token-bucket guards on an injected clock: burst, refill, and the
//! once-per-5-seconds `notify` throttle, for every category.

use std::time::{Duration, Instant};

use super::limits::{self, NOTIFY_INTERVAL};
use super::*;

fn drain(state: &mut PlayerRateState, category: RateCategory, now: Instant) {
    for i in 0..category.spec().burst {
        assert_eq!(
            state.check(category, now),
            RateDecision::Allowed,
            "{}: action {} of the burst must pass",
            category.name(),
            i + 1
        );
    }
}

#[test]
fn limits_are_the_ledger_numbers() {
    // D-SS14 and D-SS21. A change here is a policy change: update the ledger
    // and `docs/architecture/server-infrastructure-proposals.md` §2 in the same PR.
    assert_eq!(limits::CHAT.burst, 5);
    assert_eq!(limits::CHAT.refill_every, Duration::from_secs(1));
    assert_eq!(limits::MAIL_SEND.burst, 3);
    assert_eq!(limits::MAIL_SEND.refill_every, Duration::from_secs(10));
    assert_eq!(limits::DUEL_CHALLENGE.burst, 2);
    assert_eq!(limits::DUEL_CHALLENGE.refill_every, Duration::from_secs(15));
    assert_eq!(NOTIFY_INTERVAL, Duration::from_secs(5));
    assert_eq!(limits::MAX_CHAT_TEXT_UNITS, 255);
    assert_eq!(limits::CHAT_EXEMPT_ACCESS_LEVEL, 2);
}

#[test]
fn burst_passes_then_the_next_action_is_limited() {
    for category in RateCategory::ALL {
        let t0 = Instant::now();
        let mut state = PlayerRateState::default();
        drain(&mut state, category, t0);
        assert_eq!(
            state.check(category, t0),
            RateDecision::Limited { notify: true },
            "{}: burst + 1 must be limited, and the first limit notifies",
            category.name()
        );
    }
}

#[test]
fn chat_sixth_line_in_one_second_is_limited() {
    let t0 = Instant::now();
    let mut state = PlayerRateState::default();
    for i in 0..5u64 {
        assert_eq!(
            state.check(RateCategory::Chat, t0 + Duration::from_millis(i * 150)),
            RateDecision::Allowed
        );
    }
    // 900 ms in: 0.9 of a token earned, which is none.
    assert!(matches!(
        state.check(RateCategory::Chat, t0 + Duration::from_millis(900)),
        RateDecision::Limited { .. }
    ));
}

#[test]
fn refill_returns_one_token_per_period_and_keeps_partial_progress() {
    for category in RateCategory::ALL {
        let period = category.spec().refill_every;
        let t0 = Instant::now();
        let mut state = PlayerRateState::default();
        drain(&mut state, category, t0);

        // Just short of one period: still empty.
        let almost = t0 + period - Duration::from_millis(1);
        assert!(matches!(
            state.check(category, almost),
            RateDecision::Limited { .. }
        ));
        // One and a half periods: one token, and only one. The half period
        // left over is progress towards the next token, not lost.
        let t_first = t0 + period + period / 2;
        assert_eq!(state.check(category, t_first), RateDecision::Allowed);
        assert!(matches!(
            state.check(category, t_first),
            RateDecision::Limited { .. }
        ));
        // Two periods after the drain (half a period after the last check)
        // the second token is due.
        assert_eq!(
            state.check(category, t0 + period * 2),
            RateDecision::Allowed,
            "{}: the half period left over must carry over to the next token",
            category.name()
        );
    }
}

#[test]
fn refill_caps_at_the_burst() {
    for category in RateCategory::ALL {
        let spec = category.spec();
        let t0 = Instant::now();
        let mut state = PlayerRateState::default();
        drain(&mut state, category, t0);
        // A long idle refills to the burst and no further.
        let later = t0 + spec.refill_every * (spec.burst * 10);
        drain(&mut state, category, later);
        assert!(matches!(
            state.check(category, later),
            RateDecision::Limited { .. }
        ));
    }
}

#[test]
fn notify_is_true_at_most_once_per_five_seconds() {
    // Duel challenge refills slowest (15 s), so the bucket stays empty
    // across the whole window and only the notify throttle is under test.
    let category = RateCategory::DuelChallenge;
    let t0 = Instant::now();
    let mut state = PlayerRateState::default();
    drain(&mut state, category, t0);

    assert_eq!(
        state.check(category, t0),
        RateDecision::Limited { notify: true }
    );
    for ms in [0u64, 1, 2_500, 4_999] {
        assert_eq!(
            state.check(category, t0 + Duration::from_millis(ms)),
            RateDecision::Limited { notify: false },
            "a second notify {ms} ms after the first must be suppressed"
        );
    }
    assert_eq!(
        state.check(category, t0 + NOTIFY_INTERVAL),
        RateDecision::Limited { notify: true },
        "5 s after the last notify, the next drop notifies again"
    );
    assert_eq!(
        state.check(category, t0 + NOTIFY_INTERVAL + Duration::from_secs(1)),
        RateDecision::Limited { notify: false }
    );
}

#[test]
fn categories_do_not_share_a_bucket() {
    let t0 = Instant::now();
    let mut state = PlayerRateState::default();
    drain(&mut state, RateCategory::Chat, t0);
    assert!(matches!(
        state.check(RateCategory::Chat, t0),
        RateDecision::Limited { .. }
    ));
    // An empty chat bucket leaves mail and duels untouched.
    drain(&mut state, RateCategory::MailSend, t0);
    drain(&mut state, RateCategory::DuelChallenge, t0);
}

#[test]
fn a_clock_that_goes_backwards_earns_nothing() {
    let t0 = Instant::now() + Duration::from_secs(60);
    let mut state = PlayerRateState::default();
    drain(&mut state, RateCategory::Chat, t0);
    assert!(matches!(
        state.check(RateCategory::Chat, t0 - Duration::from_secs(30)),
        RateDecision::Limited { .. }
    ));
    assert!(matches!(
        state.check(RateCategory::Chat, t0),
        RateDecision::Limited { .. }
    ));
}
