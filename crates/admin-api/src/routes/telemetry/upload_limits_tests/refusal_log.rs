//! The refusal throttle (negative-logging Pattern D): the burst, the
//! independence, the escalating window, the global ceiling, and the
//! `event` that keeps credential refusals SigNoz-only.

use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};

use crate::routes::dev_session::AuthError;
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::refusal_log::{
    Decision, Pool, RefusalLog, GLOBAL_ROWS_PER_WINDOW, MAX_WINDOW, RESET_QUIET, WINDOW,
};
use crate::routes::telemetry::replay_tests::capture;
use crate::routes::telemetry::upload_gate::{Route, Uploader};

/// The burst: repeats of one refusal inside the window write nothing, and
/// the next row carries their count.
#[test]
fn repeated_refusals_are_throttled_with_a_count() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    assert_eq!(
        log.decide(Pool::Server, "busy", "sess-a", t0),
        Decision::Emit { suppressed: 0 }
    );
    for _ in 0..4 {
        assert_eq!(
            log.decide(Pool::Server, "busy", "sess-a", t0),
            Decision::Suppress
        );
    }
    assert_eq!(
        log.decide(Pool::Server, "busy", "sess-a", t0 + WINDOW),
        Decision::Emit { suppressed: 4 }
    );
}

/// Independence: another uploader's first refusal, or another reason for
/// the same uploader, is not swallowed by an open window.
#[test]
fn one_uploaders_refusals_do_not_hide_anothers() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    log.decide(Pool::Server, "busy", "sess-a", t0);
    assert_eq!(
        log.decide(Pool::Server, "busy", "sess-b", t0),
        Decision::Emit { suppressed: 0 }
    );
    assert_eq!(
        log.decide(Pool::Server, "rate_limited", "sess-a", t0),
        Decision::Emit { suppressed: 0 }
    );
}

/// Many distinct uploaders cannot flood the log: past the global ceiling,
/// refusals are counted, not written.
#[test]
fn the_refusal_log_has_a_global_ceiling() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    let emitted = (0..GLOBAL_ROWS_PER_WINDOW + 20)
        .filter(|i| {
            matches!(
                log.decide(
                    Pool::Credential,
                    "missing_token",
                    &format!("198.51.100.{i}"),
                    t0
                ),
                Decision::Emit { .. }
            )
        })
        .count();
    assert_eq!(emitted, GLOBAL_ROWS_PER_WINDOW as usize);
}

/// The 2026-10-12 colo flood: one pair refused every 12 s (a burst of three
/// each time) for an hour. A fixed 10 s window wrote a row per retry, about
/// 300; the escalating window writes a handful.
#[test]
fn a_steady_retrier_slower_than_the_first_window_is_throttled() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    let mut emitted = 0;
    let mut refusals = 0;
    let mut t = Duration::ZERO;
    while t <= Duration::from_secs(3600) {
        for _ in 0..3 {
            refusals += 1;
            if matches!(
                log.decide(Pool::Credential, "token_expired", "sess-a", t0 + t),
                Decision::Emit { .. }
            ) {
                emitted += 1;
            }
        }
        t += Duration::from_secs(12);
    }
    assert_eq!(refusals, 3 * 301);
    assert!(
        emitted <= 15,
        "a 12 s retrier wrote {emitted} rows in an hour (old fixed window: ~300)"
    );
    // It still writes: an operator sees the pair is alive.
    assert!(
        emitted >= 5,
        "a steady retrier still writes rows, got {emitted}"
    );
}

/// The window doubles per written row and stops at the cap; the next row
/// still carries every refusal counted since the last one.
#[test]
fn the_window_doubles_per_row_up_to_the_cap() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    assert_eq!(
        log.decide(Pool::Credential, "bad_token", "sess-a", t0),
        Decision::Emit { suppressed: 0 }
    );
    // Second row at 10 s: the window is now 20 s.
    let mut at = t0 + WINDOW;
    assert_eq!(
        log.decide(Pool::Credential, "bad_token", "sess-a", at),
        Decision::Emit { suppressed: 0 }
    );
    assert_eq!(
        log.decide(Pool::Credential, "bad_token", "sess-a", at + WINDOW),
        Decision::Suppress,
        "the second row widened the window to 20 s"
    );
    assert_eq!(
        log.decide(Pool::Credential, "bad_token", "sess-a", at + 2 * WINDOW),
        Decision::Emit { suppressed: 1 }
    );
    at += 2 * WINDOW;
    // Keep the pair busy until the window would pass the cap.
    let mut window = 4 * WINDOW;
    while window < MAX_WINDOW {
        assert_eq!(
            log.decide(
                Pool::Credential,
                "bad_token",
                "sess-a",
                at + window - Duration::from_secs(1)
            ),
            Decision::Suppress
        );
        assert_eq!(
            log.decide(Pool::Credential, "bad_token", "sess-a", at + window),
            Decision::Emit { suppressed: 1 }
        );
        at += window;
        window *= 2;
    }
    assert_eq!(
        log.decide(
            Pool::Credential,
            "bad_token",
            "sess-a",
            at + MAX_WINDOW - Duration::from_secs(1)
        ),
        Decision::Suppress
    );
    assert_eq!(
        log.decide(Pool::Credential, "bad_token", "sess-a", at + MAX_WINDOW),
        Decision::Emit { suppressed: 1 },
        "the window is capped at MAX_WINDOW"
    );
}

/// A pair that has been quiet for `RESET_QUIET` starts again at the first
/// window, so its next burst is reported as promptly as a new pair's.
#[test]
fn a_quiet_pair_resets_to_the_first_window() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    // Escalate: refused every 12 s for ten minutes widens the window well
    // past 10 s.
    let mut at = t0;
    let end = t0 + Duration::from_secs(600);
    while at <= end {
        log.decide(Pool::Server, "busy", "sess-a", at);
        at += Duration::from_secs(12);
    }
    let last = at - Duration::from_secs(12);
    // Just short of the quiet spell, the pair is still escalated.
    let probe = RefusalLog::new();
    let mut p = t0;
    while p <= end {
        probe.decide(Pool::Server, "busy", "sess-a", p);
        p += Duration::from_secs(12);
    }
    let early = last + RESET_QUIET - Duration::from_secs(1);
    assert!(matches!(
        probe.decide(Pool::Server, "busy", "sess-a", early),
        Decision::Emit { .. }
    ));
    assert_eq!(
        probe.decide(Pool::Server, "busy", "sess-a", early + WINDOW),
        Decision::Suppress,
        "not quiet long enough: the window stays wide"
    );
    // Quiet for RESET_QUIET since the last refusal.
    at = last + RESET_QUIET;
    // This row still reports what the wide window counted.
    assert!(matches!(
        log.decide(Pool::Server, "busy", "sess-a", at),
        Decision::Emit { suppressed } if suppressed > 0
    ));
    assert_eq!(
        log.decide(
            Pool::Server,
            "busy",
            "sess-a",
            at + WINDOW - Duration::from_secs(1)
        ),
        Decision::Suppress
    );
    assert_eq!(
        log.decide(Pool::Server, "busy", "sess-a", at + WINDOW),
        Decision::Emit { suppressed: 1 },
        "after a quiet spell the window is back to 10 s"
    );
}

fn uploader() -> Uploader {
    Uploader {
        peer: IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7)),
        session_id: Some("sess-a".into()),
        install_id: Some("install-a".into()),
        session_kind: Some("lab".into()),
    }
}

/// Credential refusals carry the SigNoz-only `event`; server-state and
/// budget refusals carry the ordinary one, and both stay WARN. Reverting
/// the `event` split (or `is_credential_refusal`) trips this. The strings
/// are literals on purpose: they must match the discord crate's
/// `SIGNOZ_ONLY_EVENTS` row, which the server crate's
/// `discord_ingest_tests` checks end to end.
#[test]
fn credential_refusals_carry_the_signoz_only_event() {
    let cases: Vec<(IngestError, &str)> = vec![
        (
            IngestError::Auth(AuthError::Expired { exp: 1, now: 2 }),
            "upload_auth_refused",
        ),
        (
            IngestError::Auth(AuthError::BadSignature),
            "upload_auth_refused",
        ),
        (IngestError::MissingAuth, "upload_auth_refused"),
        (
            IngestError::Auth(AuthError::MissingScope { wanted: "x" }),
            "upload_auth_refused",
        ),
        (
            IngestError::Auth(AuthError::SessionLifetimeExceeded { elapsed: 2, cap: 1 }),
            "upload_auth_refused",
        ),
        (
            IngestError::Auth(AuthError::SecretTooShort { got: 1, min: 2 }),
            "upload_refused",
        ),
        (
            IngestError::Auth(AuthError::KillSwitchActive),
            "upload_refused",
        ),
        (
            IngestError::Auth(AuthError::SecretMissing),
            "upload_refused",
        ),
        (
            IngestError::OverBudget {
                what: "rows",
                limit: 1,
            },
            "upload_refused",
        ),
        (IngestError::TooLarge(2, 1), "upload_refused"),
    ];
    for (err, want) in cases {
        let log = RefusalLog::new();
        let rows = capture(|| log.report(Route::Chunk, &uploader(), &err, Instant::now()));
        assert_eq!(rows.len(), 1, "{}: one row", err.reason());
        let row = &rows[0];
        assert_eq!(row.target, "launcher.ingest");
        assert_eq!(
            row.level,
            tracing::Level::WARN,
            "{}: level unchanged",
            err.reason()
        );
        assert_eq!(
            row.fields.get("event").map(String::as_str),
            Some(want),
            "{}: event",
            err.reason()
        );
    }
}
