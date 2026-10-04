//! Lease rules, one test per rule, on an explicit clock. Each fails when its
//! rule is reverted.

use super::*;

const T0: i64 = 1_800_000_000_000;

fn req(owner: &str) -> AcquireRequest {
    AcquireRequest {
        owner: owner.into(),
        purpose: "test".into(),
        ..Default::default()
    }
}

fn force(owner: &str, reason: Option<&str>) -> AcquireRequest {
    AcquireRequest {
        force: true,
        reason: reason.map(String::from),
        ..req(owner)
    }
}

#[test]
fn a_guarded_call_without_a_lease_is_refused() {
    let book = LeaseBook::default();
    let e = book.check_at(None, "client_lua_eval", T0).unwrap_err();
    assert!(e.contains("needs a lease"), "{e}");
    assert!(e.contains("lab_lease_acquire"), "{e}");
    let e = book
        .check_at(Some("lease-made-up"), "client_lua_eval", T0)
        .unwrap_err();
    assert!(e.contains("not the current lease"), "{e}");
}

#[test]
fn the_current_lease_passes_the_gate() {
    let book = LeaseBook::default();
    let l = book.acquire_at(req("a"), T0).unwrap();
    book.check_at(Some(&l.lease_id), "client_lua_eval", T0 + 1)
        .unwrap();
}

#[test]
fn a_second_acquire_is_refused_while_held_and_names_the_holder() {
    let book = LeaseBook::default();
    book.acquire_at(
        AcquireRequest {
            purpose: "ability UAT".into(),
            ..req("session-a")
        },
        T0,
    )
    .unwrap();
    let e = book.acquire_at(req("session-b"), T0 + 1000).unwrap_err();
    assert!(e.contains("session-a"), "{e}");
    assert!(e.contains("ability UAT"), "{e}");
    assert!(e.contains("since"), "{e}");
    // The refused acquire changed nothing.
    assert_eq!(book.status_at(T0 + 1000)["lease"]["owner"], "session-a");
}

#[test]
fn a_lease_expires_after_its_ttl() {
    let book = LeaseBook::default();
    let l = book
        .acquire_at(
            AcquireRequest {
                ttl_s: Some(60),
                ..req("a")
            },
            T0,
        )
        .unwrap();
    assert!(book.is_held_at(T0 + 59_999));
    assert!(!book.is_held_at(T0 + 60_000));
    let e = book
        .check_at(Some(&l.lease_id), "client_ui_click", T0 + 60_001)
        .unwrap_err();
    assert!(e.contains("expired"), "{e}");
    // After expiry someone else may take the lab.
    book.acquire_at(req("b"), T0 + 60_001).unwrap();
    assert_eq!(book.status_at(T0 + 60_002)["recent"][0]["how"], "expired");
}

#[test]
fn a_guarded_call_renews_the_lease() {
    let book = LeaseBook::default();
    let l = book
        .acquire_at(
            AcquireRequest {
                ttl_s: Some(60),
                ..req("a")
            },
            T0,
        )
        .unwrap();
    book.check_at(Some(&l.lease_id), "client_input_key", T0 + 50_000)
        .unwrap();
    // Without the touch this would have expired at T0 + 60 s.
    assert!(book.is_held_at(T0 + 100_000));
    assert!(!book.is_held_at(T0 + 110_000));
}

#[test]
fn explicit_renew_extends_and_a_stale_id_cannot_renew() {
    let book = LeaseBook::default();
    let l = book
        .acquire_at(
            AcquireRequest {
                ttl_s: Some(60),
                ..req("a")
            },
            T0,
        )
        .unwrap();
    let r = book.renew_at(&l.lease_id, Some(600), T0 + 30_000).unwrap();
    assert_eq!(r.expires_ms, T0 + 630_000);
    assert!(book.renew_at("lease-other", None, T0).is_err());
    assert!(book.renew_at(&l.lease_id, Some(MAX_TTL_S + 1), T0).is_err());
}

#[test]
fn force_takes_over_and_records_the_previous_holder() {
    let book = LeaseBook::default();
    let a = book.acquire_at(req("session-a"), T0).unwrap();
    // force needs a reason.
    assert!(book.acquire_at(force("session-b", None), T0 + 1).is_err());
    assert!(book
        .acquire_at(force("session-b", Some("  ")), T0 + 1)
        .is_err());
    let b = book
        .acquire_at(force("session-b", Some("a crashed mid-run")), T0 + 2)
        .unwrap();
    let prev = b.took_over_from.as_ref().expect("previous holder recorded");
    assert_eq!(prev.owner, "session-a");
    assert_eq!(prev.how, "taken_over");
    // The old holder's next call is refused and told who took over.
    let e = book
        .check_at(Some(&a.lease_id), "client_lua_eval", T0 + 3)
        .unwrap_err();
    assert!(e.contains("taken over"), "{e}");
    assert!(e.contains("session-b"), "{e}");
    assert!(e.contains("a crashed mid-run"), "{e}");
    book.check_at(Some(&b.lease_id), "client_lua_eval", T0 + 3)
        .unwrap();
}

#[test]
fn release_frees_the_lab_and_only_the_holder_can_release() {
    let book = LeaseBook::default();
    let l = book.acquire_at(req("a"), T0).unwrap();
    assert!(book.release_at("lease-not-mine", T0 + 1).is_err());
    assert!(book.is_held_at(T0 + 1));
    book.release_at(&l.lease_id, T0 + 2).unwrap();
    assert!(!book.is_held_at(T0 + 2));
    let e = book
        .check_at(Some(&l.lease_id), "client_lua_eval", T0 + 3)
        .unwrap_err();
    assert!(e.contains("released"), "{e}");
}

#[test]
fn status_never_shows_the_lease_id() {
    let book = LeaseBook::default();
    let l = book.acquire_at(req("a"), T0).unwrap();
    let s = book.status_at(T0).to_string();
    assert!(!s.contains(&l.lease_id), "{s}");
    assert!(s.contains("\"owner\":\"a\""), "{s}");
}

#[test]
fn a_run_lease_of_its_own_is_released_when_the_run_ends() {
    let book = Arc::new(LeaseBook::default());
    {
        let run = RunLease::acquire_own(book.clone(), "UAT run".into()).unwrap();
        assert_eq!(book.status()["lease"]["owner"], run::UAT_RUN_OWNER);
        run.touch("client_ui_click").unwrap();
    }
    assert!(!book.is_held());
    assert_eq!(book.status()["recent"][0]["how"], "released");
}

#[test]
fn a_run_lease_is_refused_while_held_and_stops_after_a_takeover() {
    let book = Arc::new(LeaseBook::default());
    let held = book.acquire(req("session-a")).unwrap();
    let e = RunLease::acquire_own(book.clone(), "UAT run".into()).unwrap_err();
    assert!(e.contains("session-a"), "{e}");

    // A run under the caller's lease: a takeover stops it at the next step,
    // and ending the run does not release a lease it never owned.
    let run = RunLease::caller(book.clone(), held.lease_id.clone());
    run.touch("client_ui_click").unwrap();
    let b = book
        .acquire(force("session-b", Some("run is stuck")))
        .unwrap();
    let e = run.touch("client_ui_click").unwrap_err();
    assert!(e.contains("taken over"), "{e}");
    drop(run);
    book.check(Some(&b.lease_id), "client_ui_click").unwrap();
}

/// Regression guard (review 2026-10-04): a run's `wait_ms` steps make no
/// tool calls, so only the keep-alive renews the lease through them. A
/// minimum-ttl (30 s) lease survives a 65 s wait with it, and lapses
/// without it. Explicit clock: tokio's paused time drives both.
#[tokio::test(start_paused = true)]
async fn a_keep_alive_holds_a_minimum_ttl_lease_through_a_long_wait() {
    let book = Arc::new(LeaseBook::default());
    let start = tokio::time::Instant::now();
    let clock: Arc<dyn Fn() -> i64 + Send + Sync> =
        Arc::new(move || T0 + start.elapsed().as_millis() as i64);
    let l = book
        .acquire_at(
            AcquireRequest {
                ttl_s: Some(MIN_TTL_S),
                ..req("a")
            },
            T0,
        )
        .unwrap();
    let run = RunLease::caller(book.clone(), l.lease_id.clone());
    let keep = run.keep_alive_with(clock.clone());
    tokio::time::sleep(std::time::Duration::from_secs(65)).await;
    assert!(book.is_held_at(clock()), "renewed through the wait");
    assert!(keep.revoked().borrow().is_none());
    drop(keep);
    tokio::time::sleep(std::time::Duration::from_secs(31)).await;
    assert!(
        !book.is_held_at(clock()),
        "without the keep-alive it lapses"
    );
}

#[test]
fn the_renew_interval_is_a_third_of_the_ttl() {
    assert_eq!(run::renew_interval(30).as_secs(), 10);
    assert_eq!(run::renew_interval(600).as_secs(), 200);
}

/// A takeover reaches a running keep-alive at once, not at its next
/// renewal (which for a 600 s lease is 200 s away).
#[tokio::test]
async fn a_keep_alive_reports_a_takeover_at_once() {
    let book = Arc::new(LeaseBook::default());
    let l = book.acquire(req("a")).unwrap();
    let run = RunLease::caller(book.clone(), l.lease_id.clone());
    let keep = run.keep_alive();
    let mut rx = keep.revoked();
    book.acquire(force("session-b", Some("stuck run"))).unwrap();
    let got = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        rx.wait_for(|r| r.is_some()),
    )
    .await
    .expect("revocation within 2 s")
    .unwrap()
    .clone()
    .unwrap();
    assert!(got.contains("taken over"), "{got}");
}

#[test]
fn acquire_validates_owner_purpose_and_ttl() {
    let book = LeaseBook::default();
    assert!(book.acquire_at(req(" "), T0).is_err());
    let no_purpose = AcquireRequest {
        purpose: String::new(),
        ..req("a")
    };
    assert!(book.acquire_at(no_purpose, T0).is_err());
    for bad in [0, MIN_TTL_S - 1, MAX_TTL_S + 1] {
        let r = AcquireRequest {
            ttl_s: Some(bad),
            ..req("a")
        };
        assert!(book.acquire_at(r, T0).is_err(), "ttl {bad}");
    }
    let l = book.acquire_at(req("a"), T0).unwrap();
    assert_eq!(l.ttl_s, DEFAULT_TTL_S);
}
