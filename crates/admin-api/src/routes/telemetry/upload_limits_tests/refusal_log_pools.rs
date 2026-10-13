//! The refusal throttle's shared state: slot collisions and the two global
//! ceilings (server and credential refusals).

use std::time::{Duration, Instant};

use crate::routes::telemetry::refusal_log::{
    Decision, Pool, RefusalLog, GLOBAL_ROWS_PER_WINDOW, RESET_QUIET, WINDOW,
};

/// Two active pairs on one slot do not evict each other. Before, each
/// takeover restarted the incumbent at the 10 s window, so two colliding
/// 12 s retriers each wrote a row per retry, forever. The newcomer's rows
/// are written untracked (its first is never lost).
#[test]
fn colliding_pairs_do_not_reset_each_others_escalation() {
    // One slot: every pair collides on both of its candidate slots.
    let log = RefusalLog::with_slots(1);
    let t0 = Instant::now();
    let (mut a_rows, mut b_rows) = (0, 0);
    let mut b_first = None;
    let mut t = Duration::ZERO;
    while t <= Duration::from_secs(3600) {
        if matches!(
            log.decide(Pool::Credential, "token_expired", "sess-a", t0 + t),
            Decision::Emit { .. }
        ) {
            a_rows += 1;
        }
        let b = log.decide(
            Pool::Credential,
            "token_expired",
            "sess-b",
            t0 + t + Duration::from_secs(1),
        );
        b_first.get_or_insert(b);
        if matches!(b, Decision::Emit { .. }) {
            b_rows += 1;
        }
        t += Duration::from_secs(12);
    }
    assert!(
        a_rows <= 15,
        "the incumbent kept escalating despite the collision, wrote {a_rows}"
    );
    assert_eq!(
        b_first,
        Some(Decision::Emit { suppressed: 0 }),
        "the newcomer's first refusal is written"
    );
    assert!(b_rows > 0);
}

/// A quiet incumbent gives its slot up: the newcomer takes it and is
/// throttled like any tracked pair.
#[test]
fn a_quiet_incumbent_yields_its_slot() {
    let log = RefusalLog::with_slots(1);
    let t0 = Instant::now();
    log.decide(Pool::Server, "rate_limited", "sess-a", t0);
    let t1 = t0 + RESET_QUIET;
    assert_eq!(
        log.decide(Pool::Server, "rate_limited", "sess-b", t1),
        Decision::Emit { suppressed: 0 }
    );
    assert_eq!(
        log.decide(
            Pool::Server,
            "rate_limited",
            "sess-b",
            t1 + Duration::from_secs(1)
        ),
        Decision::Suppress,
        "the newcomer holds the slot now, so its repeat is throttled"
    );
}

/// Fill one ceiling at `at` with `GLOBAL_ROWS_PER_WINDOW` distinct pairs.
fn fill(log: &RefusalLog, pool: Pool, reason: &'static str, at: Instant) {
    for i in 0..GLOBAL_ROWS_PER_WINDOW {
        assert!(matches!(
            log.decide(pool, reason, &format!("filler-{i}"), at),
            Decision::Emit { .. }
        ));
    }
}

/// A widened pair the ceiling holds back writes as soon as the ceiling
/// reopens. Before, the hold restarted the pair's (wide) window at the
/// hold, silencing it for up to 15 min.
#[test]
fn a_ceiling_hold_does_not_silence_a_widened_pair() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    // Rows at 0, 10, 30 and 70 s widen the window to 80 s.
    for s in [0, 10, 30, 70] {
        assert!(matches!(
            log.decide(
                Pool::Server,
                "rate_limited",
                "sess-a",
                t0 + Duration::from_secs(s)
            ),
            Decision::Emit { .. }
        ));
    }
    let hold = t0 + Duration::from_secs(150);
    fill(&log, Pool::Server, "over_budget", hold);
    assert_eq!(
        log.decide(Pool::Server, "rate_limited", "sess-a", hold),
        Decision::Suppress,
        "the ceiling is full"
    );
    assert_eq!(
        log.decide(Pool::Server, "rate_limited", "sess-a", hold + WINDOW),
        Decision::Emit { suppressed: 1 },
        "the owed row is written once the ceiling reopens"
    );
}

/// A new pair the ceiling holds back is owed its first row too, and writes
/// it when the ceiling reopens, even inside its own first window.
#[test]
fn a_ceiling_hold_owes_a_new_pair_its_first_row() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    fill(&log, Pool::Server, "over_budget", t0);
    // Held 5 s into the ceiling's window; the ceiling reopens 5 s later,
    // inside the pair's own 10 s window.
    assert_eq!(
        log.decide(
            Pool::Server,
            "kill_switch",
            "sess-a",
            t0 + Duration::from_secs(5)
        ),
        Decision::Suppress
    );
    assert_eq!(
        log.decide(Pool::Server, "kill_switch", "sess-a", t0 + WINDOW),
        Decision::Emit { suppressed: 1 }
    );
}

/// Credential refusals have their own ceiling: a crowd of stale tokens
/// does not hold back the first rows of refusals that still post to
/// Discord, and is itself still bounded.
#[test]
fn credential_rows_do_not_crowd_out_server_first_rows() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    let credential_rows = (0..GLOBAL_ROWS_PER_WINDOW + 20)
        .filter(|i| {
            matches!(
                log.decide(Pool::Credential, "token_expired", &format!("stale-{i}"), t0),
                Decision::Emit { .. }
            )
        })
        .count();
    assert_eq!(credential_rows, GLOBAL_ROWS_PER_WINDOW as usize);
    for reason in ["kill_switch", "secret_unusable", "rate_limited"] {
        assert_eq!(
            log.decide(Pool::Server, reason, "sess-a", t0),
            Decision::Emit { suppressed: 0 },
            "{reason}: its first row is not crowded out"
        );
    }
}
