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
