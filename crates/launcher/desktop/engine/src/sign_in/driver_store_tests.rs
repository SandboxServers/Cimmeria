//! Driver tests over a credential store that refuses reads, writes or deletes
//! (contract C17.3 and C18.1, F17). The fault flags are set per test; the store
//! otherwise behaves like [`MemoryStore`].

use std::sync::atomic::{AtomicBool, Ordering};

use crate::sign_in::store::StoreError;

use super::*;

/// A store whose `get`, `set` and `delete` each answer `Err(Io)` while their flag
/// is set, and delegate to the inner store otherwise.
#[derive(Default)]
struct FaultyStore {
    inner: MemoryStore,
    fail_get: AtomicBool,
    fail_set: AtomicBool,
    fail_delete: AtomicBool,
}

impl CredentialStore for FaultyStore {
    fn get(&self, host: &str) -> Result<Option<Stored>, StoreError> {
        if self.fail_get.load(Ordering::SeqCst) {
            return Err(StoreError::Io);
        }
        self.inner.get(host)
    }

    fn set(&self, host: &str, stored: &Stored) -> Result<(), StoreError> {
        if self.fail_set.load(Ordering::SeqCst) {
            return Err(StoreError::Io);
        }
        self.inner.set(host, stored)
    }

    fn delete(&self, host: &str) -> Result<(), StoreError> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(StoreError::Io);
        }
        self.inner.delete(host)
    }
}

/// A sign-in whose access token is inside the refresh margin, so the next
/// `access_token` call refreshes.
fn near_expiry_body(refresh: &str) -> Value {
    let mut body = signed_in_body(refresh);
    body["expires_in"] = json!(30);
    body
}

#[tokio::test]
async fn a_sign_in_the_store_refuses_is_kept_for_the_session() {
    let faulty = Arc::new(FaultyStore::default());
    faulty.fail_set.store(true, Ordering::SeqCst);
    let fx = Fixture::with_store(true, true, faulty.clone()).await;
    mount_token(&fx.server, signed_in_body("r1")).await;
    let status = fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
    assert_eq!(status.storage_warning, Some("session_only"));

    fx.state.lock().unwrap().set_sign_in(None);
    let status = fx.inspect().await.unwrap();
    assert!(
        matches!(status.state, SignInStateView::SignedIn { .. }),
        "the session-only sign-in still restores"
    );
    assert!(faulty.inner.get(HOST).unwrap().is_none());
}

#[tokio::test]
async fn a_refresh_the_store_refuses_keeps_the_rotated_token() {
    let faulty = Arc::new(FaultyStore::default());
    faulty.fail_set.store(true, Ordering::SeqCst);
    let fx = Fixture::with_store(true, true, faulty.clone()).await;
    mount_token(&fx.server, near_expiry_body("r1")).await;
    for (spent, next) in [("r1", "r2"), ("r2", "r3")] {
        Mock::given(method("POST"))
            .and(path("/auth/v1/token"))
            .and(body_partial_json(json!({
                "grant_type": "refresh_token",
                "refresh_token": spent,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(near_expiry_body(next)))
            .expect(1)
            .mount(&fx.server)
            .await;
    }
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    let session = fx.state.lock().unwrap().sign_in().expect("signed in");

    session.access_token().await.expect("first refresh");
    session.access_token().await.expect("second refresh");
    let bodies = fx.requests_to("/auth/v1/token").await;
    assert_eq!(bodies.last().unwrap()["refresh_token"], "r2");
}

#[tokio::test]
async fn a_sign_out_the_store_refuses_stays_signed_out() {
    let faulty = Arc::new(FaultyStore::default());
    let fx = Fixture::with_store(true, true, faulty.clone()).await;
    fx.store_entry("r0");
    assert!(matches!(
        fx.inspect().await.unwrap().state,
        SignInStateView::SignedIn { .. }
    ));

    faulty.fail_delete.store(true, Ordering::SeqCst);
    let status = fx
        .run(
            json!({"command": "sign_out", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await
        .unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert_eq!(status.storage_warning, Some("sign_out_incomplete"));

    assert!(faulty.inner.get(HOST).unwrap().is_some());
    fx.state.lock().unwrap().set_sign_in(None);
    let status = fx.inspect().await.unwrap();
    assert!(
        matches!(status.state, SignInStateView::SignedOut),
        "the refused sign-out is not undone by the next restore"
    );
}

#[tokio::test]
async fn an_unreadable_store_is_reported_and_sign_in_still_works() {
    let faulty = Arc::new(FaultyStore::default());
    faulty.fail_get.store(true, Ordering::SeqCst);
    let fx = Fixture::with_store(true, true, faulty.clone()).await;
    fx.store_entry("r0");
    let status = fx.inspect().await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert_eq!(status.storage_warning, Some("store_unreadable"));

    faulty.fail_get.store(false, Ordering::SeqCst);
    mount_token(&fx.server, signed_in_body("r1")).await;
    let status = fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
    assert_eq!(status.storage_warning, None);
}

#[test]
fn deleting_nothing_is_ok() {
    assert_eq!(MemoryStore::default().delete(HOST), Ok(()));
}

#[tokio::test]
async fn a_store_that_recovers_clears_its_unreadable_warning() {
    let faulty = Arc::new(FaultyStore::default());
    faulty.fail_get.store(true, Ordering::SeqCst);
    let fx = Fixture::with_store(true, true, faulty.clone()).await;
    let status = fx.inspect().await.unwrap();
    assert_eq!(status.storage_warning, Some("store_unreadable"));

    faulty.fail_get.store(false, Ordering::SeqCst);
    fx.store_entry("r0");
    let status = fx.inspect().await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
    assert_eq!(status.storage_warning, None);
}

#[tokio::test]
async fn a_refused_sign_out_with_no_login_server_still_warns() {
    let faulty = Arc::new(FaultyStore::default());
    let fx = Fixture::with_store(false, false, faulty.clone()).await;
    fx.store_entry("r0");
    faulty.fail_delete.store(true, Ordering::SeqCst);
    let base = url::Url::parse(&format!("{}/", fx.server.uri())).unwrap();
    // The driver's slot for the host, as a session restored by `run` would share.
    let slot = fx.driver.slot(&base);
    let session = Arc::new(SignIn::new_in(
        base,
        reqwest::Client::new(),
        fx.store.clone(),
        slot,
    ));
    fx.state.lock().unwrap().set_sign_in(Some(session.clone()));
    let site = Site {
        state: &fx.state,
        server: None,
        discovery: None,
        base: None,
    };
    let status = fx.driver.sign_out(&site, Some(session)).await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert_eq!(status.storage_warning, Some("sign_out_incomplete"));
}
