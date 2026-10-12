//! The upload retry state machine on an injected clock: no HTTP, no files.
//! Each test names the fix whose revert makes it fail.

use super::*;

const BASE: Duration = Duration::from_secs(2);
const WALL: i64 = 1_800_000_000_000;

fn creds(token: &str, expires_at_ms: i64) -> Credentials {
    Credentials {
        token: token.into(),
        expires_at_ms,
    }
}

fn gate(token: &str) -> UploadGate {
    UploadGate::new(creds(token, 0), BASE, BACKOFF_CAP)
}

/// Repeated 401s with an unchanged token in the file: each refusal holds
/// longer, doubling from the flush interval up to the 10 min cap, and the
/// gate refuses to send inside a hold. Reverting to "retry on the next
/// cadence" (no hold, or a constant one) fails the growth or the cap.
#[test]
fn repeated_refusals_with_the_same_token_back_off_to_the_cap() {
    let mut g = gate("old");
    let mut now = Instant::now();
    let mut holds = Vec::new();
    for _ in 0..12 {
        assert!(
            g.ready(now, WALL, || Some(creds("old", 0))),
            "a hold has run out"
        );
        match g.on_auth_refused(now, WALL, 401, || Some(creds("old", 0))) {
            AfterRefusal::Hold(d) => holds.push(d),
            AfterRefusal::RetryNow => panic!("unchanged token must not retry at once"),
        }
        let d = *holds.last().unwrap();
        // Inside the hold nothing goes out, however often the loop asks.
        assert!(!g.ready(now + d - Duration::from_millis(1), WALL, || None));
        now += d;
    }
    let secs: Vec<u64> = holds.iter().map(Duration::as_secs).collect();
    assert_eq!(secs, [2, 4, 8, 16, 32, 64, 128, 256, 512, 600, 600, 600]);
    assert!(g.is_paused());
}

/// A missing or unreadable session file backs off the same way.
#[test]
fn refusal_with_no_readable_file_holds() {
    let mut g = gate("old");
    let now = Instant::now();
    assert_eq!(
        g.on_auth_refused(now, WALL, 401, || None),
        AfterRefusal::Hold(BASE)
    );
    assert_eq!(
        g.on_auth_refused(now, WALL, 401, || None),
        AfterRefusal::Hold(BASE * 2)
    );
}

/// After several refusals, a rotated token in the file means an immediate
/// retry with it, and the backoff starts over. Reverting the re-read (the
/// token is never swapped) fails both the `RetryNow` and the token check.
#[test]
fn a_changed_token_on_reread_retries_at_once_and_resets_the_backoff() {
    let mut g = gate("old");
    let mut now = Instant::now();
    for _ in 0..5 {
        if let AfterRefusal::Hold(d) = g.on_auth_refused(now, WALL, 401, || Some(creds("old", 0))) {
            now += d;
        }
    }
    assert_eq!(
        g.on_auth_refused(now, WALL, 401, || Some(creds("new", WALL + 60_000))),
        AfterRefusal::RetryNow
    );
    assert_eq!(g.token(), "new");
    // No hold stands in the way of the retry. (Still paused until a POST is
    // accepted, so `ready` re-reads; the file has nothing newer.)
    assert!(g.ready(now, WALL, || Some(creds("new", WALL + 60_000))));
    // The new token is refused too: the backoff starts again from the base.
    assert_eq!(
        g.on_auth_refused(now, WALL, 401, || Some(creds("new", WALL + 60_000))),
        AfterRefusal::Hold(BASE)
    );
}

/// A rotation that lands while uploads are held is picked up at the next
/// backoff deadline: `ready` re-reads the file and the POST goes out with
/// the new token.
#[test]
fn a_rotation_during_a_hold_is_picked_up_at_the_deadline() {
    let mut g = gate("old");
    let now = Instant::now();
    let AfterRefusal::Hold(d) = g.on_auth_refused(now, WALL, 401, || Some(creds("old", 0))) else {
        panic!("expected a hold");
    };
    assert!(g.ready(now + d, WALL, || Some(creds("rotated", 0))));
    assert_eq!(g.token(), "rotated");
}

/// A token whose `expires_at_ms` has passed is never sent: `ready` says no
/// and holds with backoff, re-reading the file each time. Reverting the
/// expiry check sends the dead token (`ready` returns true).
#[test]
fn an_expired_token_is_not_sent() {
    let mut g = UploadGate::new(creds("dead", WALL - 1), BASE, BACKOFF_CAP);
    let mut now = Instant::now();
    let mut reads = 0;
    let mut holds = Vec::new();
    for _ in 0..4 {
        assert!(!g.ready(now, WALL, || {
            reads += 1;
            Some(creds("dead", WALL - 1))
        }));
        // The hold runs: the next tick does not even re-read.
        assert!(!g.ready(now + Duration::from_millis(1), WALL, || panic!(
            "re-read inside a hold"
        )));
        let d = g.delay_for(holds.len() as u32 + 1);
        holds.push(d);
        now += d;
    }
    assert_eq!(reads, 4, "one re-read per backoff deadline");
    assert_eq!(
        holds.iter().map(Duration::as_secs).collect::<Vec<_>>(),
        [2, 4, 8, 16]
    );
    // The launcher (or the lab) writes a new token: the next deadline sends.
    assert!(g.ready(now, WALL, || Some(creds("fresh", WALL + 3_600_000))));
    assert_eq!(g.token(), "fresh");
    // The final flush at exit also refuses a known-dead token.
    let mut dead = UploadGate::new(creds("dead", WALL - 1), BASE, BACKOFF_CAP);
    assert!(!dead.ready_for_final(WALL, || None));
}

/// An empty token (a lab launch whose mint failed) is held the same way.
#[test]
fn an_empty_token_is_not_sent() {
    let mut g = gate("");
    assert!(!g.ready(Instant::now(), WALL, || Some(creds("", 0))));
    assert!(matches!(
        g.take_transitions().as_slice(),
        [Transition::Paused {
            reason: PauseReason::NoToken,
            ..
        }]
    ));
}

/// Pause and resume are reported once each, however many attempts lie
/// between them.
#[test]
fn pause_and_resume_are_reported_once() {
    let mut g = gate("old");
    let mut now = Instant::now();
    for _ in 0..6 {
        if let AfterRefusal::Hold(d) = g.on_auth_refused(now, WALL, 401, || None) {
            now += d;
        }
    }
    let t = g.take_transitions();
    assert_eq!(
        t,
        [Transition::Paused {
            reason: PauseReason::Refused,
            status: Some(401),
            retry_in: BASE,
        }]
    );
    assert!(g.ready(now, WALL, || Some(creds("new", 0))));
    g.on_success(now);
    let t = g.take_transitions();
    assert!(
        matches!(t.as_slice(), [Transition::Resumed { attempts: 6, .. }]),
        "{t:?}"
    );
    g.on_success(now);
    assert!(g.take_transitions().is_empty(), "no second resume");
}

/// 5xx and network errors back off too (they used to retry every flush),
/// and a longer `Retry-After` wins, capped. A success clears the backoff.
#[test]
fn transient_failures_back_off_and_honour_retry_after() {
    let mut g = gate("t");
    let now = Instant::now();
    assert_eq!(g.on_transient(now, None), BASE);
    assert_eq!(g.on_transient(now, None), BASE * 2);
    assert!(
        !g.ready(now + BASE, WALL, || None),
        "still inside the 4 s hold"
    );
    assert_eq!(
        g.on_transient(now, Some(Duration::from_secs(30))),
        Duration::from_secs(30)
    );
    assert_eq!(
        g.on_transient(now, Some(Duration::from_secs(86_400))),
        BACKOFF_CAP
    );
    // A shorter Retry-After does not cut the backoff short.
    assert_eq!(
        g.on_transient(now, Some(Duration::from_secs(1))),
        Duration::from_secs(32)
    );
    g.on_success(now);
    assert!(g.ready(now, WALL, || None));
    assert_eq!(g.on_transient(now, None), BASE);
    assert!(!g.is_paused(), "a transient failure is not an auth pause");
}
