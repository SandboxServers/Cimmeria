//! Unit tests for the link intent store: single use, the ten minute life and
//! the cap. A child of `stores`, so it reaches the private `locked` helper.

use super::*;

/// A link intent that attaches the identity to `account_id`.
fn intent(account_id: u32) -> LinkIntent {
    LinkIntent {
        account_id,
        family_id: "family".to_owned(),
    }
}

#[test]
fn intent_is_single_use() {
    let store = IntentStore::default();
    let t0 = Instant::now();
    let token = store.issue(intent(7), t0);
    assert_eq!(store.take(&token, t0).map(|i| i.account_id), Some(7));
    assert!(store.take(&token, t0).is_none());
}

#[test]
fn intent_expires_after_ten_minutes() {
    let store = IntentStore::default();
    let t0 = Instant::now();
    let early = store.issue(intent(8), t0);
    let late = store.issue(intent(9), t0);
    let just_inside = t0 + Duration::from_secs(10 * 60 - 1);
    assert_eq!(
        store.take(&early, just_inside).map(|i| i.account_id),
        Some(8)
    );
    let just_outside = t0 + Duration::from_secs(10 * 60 + 1);
    assert!(store.take(&late, just_outside).is_none());
}

#[test]
fn reap_drops_expired_intents() {
    let store = IntentStore::default();
    let t0 = Instant::now();
    let token = store.issue(intent(7), t0);
    store.reap(t0 + INTENT_LIFE + Duration::from_secs(1));
    assert!(store.take(&token, t0).is_none());
}

#[test]
fn unknown_intent_is_refused() {
    let store = IntentStore::default();
    let t0 = Instant::now();
    store.issue(intent(10), t0);
    assert!(store.take("nope", t0).is_none());
}

#[test]
fn intents_are_capped() {
    let store = IntentStore::default();
    let t0 = Instant::now();
    let first = store.issue(intent(11), t0);
    for i in 1..=STORE_CAP {
        store.issue(intent(11), t0 + Duration::from_millis(i as u64));
    }
    // The first intent was issued longest ago, so the cap evicted it.
    assert!(store.take(&first, t0 + Duration::from_secs(60)).is_none());
    assert!(locked(&store.by_hash).len() <= STORE_CAP);
}
