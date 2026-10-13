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

/// Drive `g` from `now` until `ready` says yes or `limit` passes, jumping
/// from one hold's end to the next the way the uploader sleeps. Returns the
/// instant it said yes, and how many times the file was re-read.
fn next_send(
    g: &mut UploadGate,
    mut now: Instant,
    limit: Instant,
    file: &Credentials,
) -> (Option<Instant>, u32) {
    let mut reads = 0;
    while now <= limit {
        if g.ready(now, WALL, || {
            reads += 1;
            Some(file.clone())
        }) {
            return (Some(now), reads);
        }
        // Inside the hold nothing is read or sent.
        assert!(!g.ready(now + Duration::from_millis(1), WALL, || panic!(
            "re-read inside a hold"
        )));
        now = g.next_attempt().expect("a refusal to send sets a hold");
    }
    (None, reads)
}

/// A token whose `expires_at_ms` is behind this clock goes out as one
/// probe at once, then at most once per cap: between probes the gate holds
/// with backoff and re-reads the file at each deadline. Reverting the
/// expiry check sends on every deadline; dropping the probe never sends.
#[test]
fn a_token_that_looks_expired_is_probed_once_per_cap() {
    let dead = creds("dead", WALL - 1);
    let mut g = UploadGate::new(dead.clone(), BASE, BACKOFF_CAP);
    let t0 = Instant::now();
    // The first probe goes at once, so a skewed client loses no time.
    assert!(g.ready(t0, WALL, || Some(dead.clone())));
    let AfterRefusal::Hold(_) = g.on_auth_refused(t0, WALL, 401, || Some(dead.clone())) else {
        panic!("the refused probe holds");
    };
    assert!(
        matches!(
            g.take_transitions().as_slice(),
            [Transition::Paused {
                reason: PauseReason::Expired,
                status: Some(401),
                ..
            }]
        ),
        "a refused token that looks expired is reported as an expiry"
    );
    let (sent, reads) = next_send(&mut g, t0, t0 + BACKOFF_CAP * 3, &dead);
    let sent = sent.expect("a second probe within the window");
    assert!(
        sent >= t0 + BACKOFF_CAP,
        "no probe before a cap has passed: {:?}",
        sent - t0
    );
    assert!(reads >= 8, "re-read at each deadline in between: {reads}");
    // The launcher (or the lab) writes a new token: the next deadline sends.
    g.on_auth_refused(sent, WALL, 401, || Some(dead.clone()));
    let fresh = creds("fresh", WALL + 3_600_000);
    let at = g.next_attempt().unwrap();
    assert!(g.ready(at, WALL, || Some(fresh.clone())));
    assert_eq!(g.token(), "fresh");
}

/// The client's clock runs ahead of the server's: the token looks expired
/// here but the server takes the probe. From then on the server decides,
/// and uploads go on every flush. Without the trust the gate would drop back
/// to one POST per cap.
#[test]
fn a_probe_the_server_accepts_resumes_normal_uploads() {
    let skewed = creds("live", WALL - 1);
    let mut g = UploadGate::new(skewed.clone(), BASE, BACKOFF_CAP);
    let t0 = Instant::now();
    assert!(g.ready(t0, WALL, || Some(skewed.clone())));
    g.on_success(t0);
    for i in 1..5 {
        assert!(
            g.ready(t0 + BASE * i, WALL, || Some(skewed.clone())),
            "flush {i} after an accepted probe"
        );
        g.on_success(t0 + BASE * i);
    }
    assert!(g.ready_for_final(WALL, || None), "the exit flush goes too");
    // The next refusal ends the trust.
    g.on_auth_refused(t0, WALL, 401, || Some(skewed.clone()));
    let at = g.next_attempt().unwrap();
    assert!(
        !g.ready(at, WALL, || Some(skewed.clone())),
        "back to probing"
    );
}

/// The final flush at exit never spends a probe on a token that looks
/// expired.
#[test]
fn the_exit_flush_skips_a_token_that_looks_expired() {
    let mut g = UploadGate::new(creds("dead", WALL - 1), BASE, BACKOFF_CAP);
    assert!(!g.ready_for_final(WALL, || None));
}

/// A rotated token that is itself already expired is reported as an
/// expiry, not a refusal.
#[test]
fn a_rotated_token_already_expired_reports_expired() {
    let mut g = gate("old");
    let r = g.on_auth_refused(Instant::now(), WALL, 401, || {
        Some(creds("also-dead", WALL - 5))
    });
    assert!(matches!(r, AfterRefusal::Hold(_)));
    assert_eq!(g.token(), "also-dead");
    assert!(matches!(
        g.take_transitions().as_slice(),
        [Transition::Paused {
            reason: PauseReason::Expired,
            ..
        }]
    ));
}

/// Two rotations in a row: each refusal finds a new token and asks for an
/// immediate retry, and the backoff never starts.
#[test]
fn a_second_rotation_retries_at_once_again() {
    let mut g = gate("old");
    let now = Instant::now();
    assert_eq!(
        g.on_auth_refused(now, WALL, 401, || Some(creds("new1", 0))),
        AfterRefusal::RetryNow
    );
    assert_eq!(
        g.on_auth_refused(now, WALL, 401, || Some(creds("new2", 0))),
        AfterRefusal::RetryNow
    );
    assert_eq!(g.token(), "new2");
    assert_eq!(g.next_attempt(), None, "no hold");
    assert!(g.ready(now, WALL, || None));
    assert!(g.take_transitions().is_empty(), "no pause");
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
