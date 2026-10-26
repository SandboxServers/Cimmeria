//! Tests of the in-memory sign-in stores.

use super::*;
use crate::provider::ProviderId;

/// A pending record with the given provider; the rest is fixed test text.
fn pending(provider: &str) -> PendingAuth {
    PendingAuth {
        provider: provider.to_owned(),
        redirect_uri: "http://127.0.0.1:4000/cb".to_owned(),
        launcher_state: "launcher-state-0001".to_owned(),
        launcher_challenge: pkce_challenge("launcher-verifier"),
        pkce_verifier: "server-verifier".to_owned(),
        nonce: "nonce-0001".to_owned(),
        intent: None,
    }
}

/// A verified identity for `subject`, with fixed test values.
fn identity(subject: &str) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new("dev").expect("a valid id"),
        issuer: "https://example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "Player_1234".to_owned(),
        email: None,
        email_verified: false,
        provider_account_created_ms: None,
    }
}

/// A handoff outcome for `subject` with no link intent.
fn outcome(subject: &str) -> HandoffOutcome {
    HandoffOutcome {
        identity: identity(subject),
        intent: None,
    }
}

#[test]
fn link_grants_are_capped_and_the_oldest_goes_first() {
    let grants = LinkGrants::default();
    let t0 = Instant::now();
    let first = grants.issue(identity("first"), t0);
    let t1 = t0 + Duration::from_secs(1);
    for i in 0..STORE_CAP {
        grants.issue(identity(&format!("subject-{i}")), t1);
    }
    assert!(grants.peek(&first, t1).is_none());
    assert!(locked(&grants.grants).len() <= STORE_CAP);
}

#[test]
fn make_room_drops_the_entry_issued_longest_ago() {
    let mut map: HashMap<u32, Instant> = HashMap::new();
    let t0 = Instant::now();
    map.insert(1, t0 + Duration::from_secs(2));
    map.insert(2, t0);
    map.insert(3, t0 + Duration::from_secs(1));
    make_room(&mut map, 3, |issued| *issued);
    assert_eq!(map.len(), 2);
    assert!(!map.contains_key(&2));
}

#[test]
fn pending_is_single_use_and_expires() {
    let store = PendingStore::default();
    let t0 = Instant::now();
    let state = store.insert(pending("dev"), t0);
    assert!(store.take(&state, t0).is_some());
    assert!(store.take(&state, t0).is_none(), "second take is refused");

    let late = store.insert(pending("dev"), t0);
    let after = t0 + PENDING_LIFE;
    assert!(store.take(&late, after).is_none(), "gone after 10 min");
}

#[test]
fn pending_cap_drops_the_oldest() {
    let store = PendingStore::default();
    let t0 = Instant::now();
    let first = store.insert(pending("dev"), t0);
    let mut last = String::new();
    for _ in 0..PENDING_CAP {
        last = store.insert(pending("dev"), t0);
    }
    assert!(store.take(&first, t0).is_none(), "the oldest was dropped");
    assert!(store.take(&last, t0).is_some(), "the newest is kept");
}

#[test]
fn handoff_needs_the_right_verifier_and_redirect() {
    let store = HandoffStore::default();
    let t0 = Instant::now();
    let challenge = pkce_challenge("launcher-verifier");
    let redirect = "http://127.0.0.1:4000/cb";

    let code = store.issue(outcome("s1"), &challenge, redirect, t0);
    assert!(store
        .redeem(&code, "wrong-verifier", redirect, t0)
        .is_none());
    assert!(
        store
            .redeem(&code, "launcher-verifier", redirect, t0)
            .is_none(),
        "a failed check still spent the code"
    );

    let code = store.issue(outcome("s1"), &challenge, redirect, t0);
    assert!(store
        .redeem(&code, "launcher-verifier", "http://127.0.0.1:4001/cb", t0)
        .is_none());

    let code = store.issue(outcome("s1"), &challenge, redirect, t0);
    let got = store.redeem(&code, "launcher-verifier", redirect, t0);
    assert_eq!(got.map(|o| o.identity.subject), Some("s1".to_owned()));
}

#[test]
fn handoff_expires_after_60_s() {
    let store = HandoffStore::default();
    let t0 = Instant::now();
    let challenge = pkce_challenge("launcher-verifier");
    let redirect = "http://127.0.0.1:4000/cb";

    let code = store.issue(outcome("s1"), &challenge, redirect, t0);
    let at_61 = t0 + Duration::from_secs(61);
    assert!(store
        .redeem(&code, "launcher-verifier", redirect, at_61)
        .is_none());
}

#[test]
fn access_token_expiry_and_family_revoke() {
    let store = AccessTokens::default();
    let t0 = Instant::now();
    let token = store.mint(7, "fam-a", true, t0);
    let grant = store.check(&token, t0).expect("live at once");
    assert_eq!(grant.account_id, 7);
    assert_eq!(grant.family_id, "fam-a");
    assert!(grant.via_code);
    assert!(store.check(&token, t0 + ACCESS_LIFE).is_none(), "15 min");

    let token = store.mint(7, "fam-b", false, t0);
    store.revoke_family("fam-b");
    assert!(store.check(&token, t0).is_none(), "family revoke ends it");
}

#[test]
fn link_grant_single_use() {
    let store = LinkGrants::default();
    let t0 = Instant::now();
    let grant = store.issue(identity("s2"), t0);
    assert!(store.take(&grant, t0).is_some());
    assert!(store.take(&grant, t0).is_none(), "second take is refused");
}

#[test]
fn link_grant_peek_keeps_the_grant() {
    let store = LinkGrants::default();
    let t0 = Instant::now();
    let grant = store.issue(identity("s3"), t0);
    let subject = |store: &LinkGrants, at: Instant| store.peek(&grant, at).map(|i| i.subject);
    assert_eq!(subject(&store, t0), Some("s3".to_owned()));
    assert_eq!(
        subject(&store, t0),
        Some("s3".to_owned()),
        "peek spends nothing"
    );
    assert!(
        store.take(&grant, t0).is_some(),
        "take still works after peeks"
    );
    assert!(store.peek(&grant, t0).is_none(), "a taken grant is gone");
    let late = store.issue(identity("s4"), t0);
    let after = t0 + LINK_GRANT_LIFE;
    assert!(store.peek(&late, after).is_none(), "gone after 10 min");
}

#[test]
fn pkce_rfc7636_vector() {
    assert_eq!(
        pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn revoke_account_ends_every_token_of_the_account() {
    let store = AccessTokens::default();
    let t0 = Instant::now();
    let a1 = store.mint(7, "fam-a", true, t0);
    let a2 = store.mint(7, "fam-b", false, t0);
    let other = store.mint(8, "fam-c", true, t0);
    store.revoke_account(7);
    assert!(store.check(&a1, t0).is_none());
    assert!(store.check(&a2, t0).is_none());
    assert!(store.check(&other, t0).is_some(), "another account is kept");
}

#[test]
fn revoke_account_except_keeps_the_callers_family() {
    let store = AccessTokens::default();
    let t0 = Instant::now();
    let own = store.mint(7, "fam-a", true, t0);
    let own_older = store.mint(7, "fam-a", false, t0);
    let other_device = store.mint(7, "fam-b", false, t0);
    let other_account = store.mint(8, "fam-c", true, t0);
    store.revoke_account_except(7, "fam-a");
    assert!(
        store.check(&own, t0).is_some(),
        "the caller's token is kept"
    );
    assert!(
        store.check(&own_older, t0).is_some(),
        "an older token of the caller's family is kept"
    );
    assert!(store.check(&other_device, t0).is_none());
    assert!(
        store.check(&other_account, t0).is_some(),
        "another account is kept"
    );
}

#[test]
fn stores_reap_drops_expired_records_and_keeps_live_ones() {
    let stores = Stores::default();
    let t0 = Instant::now();
    let redirect = "http://127.0.0.1:4000/cb";
    let challenge = pkce_challenge("launcher-verifier");
    let state = stores.pending.insert(pending("dev"), t0);
    let code = stores
        .handoff
        .issue(outcome("s1"), &challenge, redirect, t0);
    let token = stores.access.mint(7, "fam-a", true, t0);
    let grant = stores.grants.issue(identity("s2"), t0);
    // 5 min on: the 60 s handoff code is spent-by-age; the rest are live.
    stores.reap(t0 + Duration::from_secs(5 * 60));
    assert!(stores
        .handoff
        .codes
        .lock()
        .expect("lock")
        .get(&code)
        .is_none());
    assert!(stores
        .pending
        .inner
        .lock()
        .expect("lock")
        .by_state
        .contains_key(&state));
    assert!(stores.access.check(&token, t0).is_some());
    assert!(stores.grants.take(&grant, t0).is_some());
    // 16 min on: everything is past its life.
    let late = t0 + Duration::from_secs(16 * 60);
    stores.reap(late);
    let p = stores.pending.inner.lock().expect("lock");
    assert!(p.by_state.is_empty() && p.by_age.is_empty());
    drop(p);
    assert!(stores.access.check(&token, late).is_none());
}
