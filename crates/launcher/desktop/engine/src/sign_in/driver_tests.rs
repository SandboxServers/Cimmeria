//! Tests of the sign-in command driver (`driver.rs`, contract C13). `wiremock`
//! stands in for `/auth/v1`, the credential store is in memory, and the auth base
//! is pointed at the mock through the debug-only `CIMMERIA_LAUNCHER_AUTH_BASE`.

use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde_json::{json, Value};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::commands::{SignInCommand, SignInFailure, SignInStateView, SignInStatus};
use super::driver::{Browser, SignInDriver, Site};
use super::lease::RelayLease;
use super::session::SignIn;
use super::store::{CredentialStore, MemoryStore, Stored};
use super::tests::ENV_LOCK;
use crate::storage::DesktopState;

#[path = "driver_store_tests.rs"]
mod store_faults;

/// The host the stored entries are keyed by (the mock listens on loopback).
const HOST: &str = "127.0.0.1";
const AUTH_BASE_ENV: &str = "CIMMERIA_LAUNCHER_AUTH_BASE";

type Reply = Result<SignInStatus, SignInFailure>;

/// Opens nothing in a real browser: records the URL and, on a spawned task, makes
/// the redirect the browser would after the player approves.
#[derive(Default)]
struct CallbackBrowser {
    urls: Mutex<Vec<String>>,
}

impl Browser for CallbackBrowser {
    fn open(&self, url: &str) -> Result<(), ()> {
        self.urls.lock().unwrap().push(url.to_owned());
        let parsed = url::Url::parse(url).unwrap();
        let query = |name: &str| {
            parsed
                .query_pairs()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.into_owned())
                .unwrap()
        };
        let callback = format!("{}?code=c&state={}", query("redirect_uri"), query("state"));
        tokio::spawn(async move {
            let _ = reqwest::Client::new().get(callback).send().await;
        });
        Ok(())
    }
}

/// A browser that opens but never calls back.
#[derive(Default)]
struct SilentBrowser {
    urls: Mutex<Vec<String>>,
}

impl Browser for SilentBrowser {
    fn open(&self, url: &str) -> Result<(), ()> {
        self.urls.lock().unwrap().push(url.to_owned());
        Ok(())
    }
}

/// A browser that cannot be started (Wine, a locked-down host).
struct FailingBrowser;

impl Browser for FailingBrowser {
    fn open(&self, _url: &str) -> Result<(), ()> {
        Err(())
    }
}

/// One driver on a state in a temp dir, talking to a mock auth server. It owns
/// the process-wide env lock for as long as it lives.
struct Fixture {
    _env: MutexGuard<'static, ()>,
    server: MockServer,
    _dir: tempfile::TempDir,
    state: Mutex<DesktopState>,
    store: Arc<dyn CredentialStore>,
    driver: SignInDriver,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::env::remove_var(AUTH_BASE_ENV);
    }
}

impl Fixture {
    /// `providers` mounts a working `/providers`; without it the route is a 404.
    /// `reachable: false` points the auth base at a closed port instead.
    #[allow(clippy::await_holding_lock)] // tests only: serialises the process env
    async fn new(providers: bool, reachable: bool) -> Self {
        Self::with_store(providers, reachable, Arc::new(MemoryStore::default())).await
    }

    /// As [`Fixture::new`], with the driver on `store` (a fault-injecting one in
    /// `store_faults`).
    #[allow(clippy::await_holding_lock)] // tests only: serialises the process env
    async fn with_store(providers: bool, reachable: bool, store: Arc<dyn CredentialStore>) -> Self {
        let env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let server = MockServer::start().await;
        let base = if reachable {
            format!("{}/", server.uri())
        } else {
            "http://127.0.0.1:1/".to_owned()
        };
        std::env::set_var(AUTH_BASE_ENV, base);
        if providers {
            Mock::given(method("GET"))
                .and(path("/auth/v1/providers"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "api": 1,
                    "soap_relay": true,
                    "signup_open": true,
                    "providers": [{"id": "dev", "display_name": "Dev sign-in"}],
                    "links": {"website": null, "account": null}
                })))
                .mount(&server)
                .await;
        }
        let dir = tempfile::tempdir().unwrap();
        let state = DesktopState::open(&dir.path().join("state")).unwrap();
        let driver = SignInDriver::new(store.clone());
        Self {
            _env: env,
            server,
            _dir: dir,
            state: Mutex::new(state),
            store,
            driver,
        }
    }

    fn store_entry(&self, refresh: &str) {
        self.store
            .set(
                HOST,
                &Stored {
                    refresh_token: refresh.to_owned(),
                    account_name: "alice".to_owned(),
                    display_name: "Alice".to_owned(),
                },
            )
            .unwrap();
    }

    async fn run(&self, request: Value, browser: &dyn Browser) -> Reply {
        let request: SignInCommand = serde_json::from_value(request).expect("a C13 command");
        self.driver.run(&self.state, request, browser).await
    }

    async fn inspect(&self) -> Reply {
        self.run(
            json!({"command": "inspect", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await
    }

    fn has_session(&self) -> bool {
        self.state.lock().unwrap().sign_in().is_some()
    }

    async fn requests_to(&self, route: &str) -> Vec<Value> {
        self.server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == route)
            .map(|r| serde_json::from_slice(&r.body).unwrap_or(Value::Null))
            .collect()
    }
}

fn signed_in_body(refresh: &str) -> Value {
    json!({
        "result": "signed_in",
        "access_token": "a1",
        "expires_in": 900,
        "refresh_token": refresh,
        "account": {"account_name": "alice", "display_name": "Alice", "identities": []},
    })
}

fn needs_link_body() -> Value {
    json!({
        "result": "needs_link",
        "link_grant": "lg1",
        "identity": {"provider": "dev", "display_name": "Dev"},
        "hint": "Sign in with the dev provider.",
        "signup_open": true,
    })
}

async fn mount_token(server: &MockServer, body: Value) {
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(
            json!({"grant_type": "authorization_code"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

fn begin() -> Value {
    json!({"command": "begin", "schema_version": 1, "provider": "dev"})
}

fn password_link(ack: bool, password: &str) -> Value {
    json!({
        "command": "link_existing_password", "schema_version": 1,
        "account_name": "alice", "password": password,
        "acknowledge_password_removal": ack,
    })
}

#[test]
fn commands_deserialize_and_refuse_unknown_fields() {
    let good = [
        json!({"command": "inspect", "schema_version": 1}),
        json!({"command": "begin", "schema_version": 1, "provider": "dev"}),
        json!({"command": "cancel", "schema_version": 1}),
        json!({"command": "link_new", "schema_version": 1}),
        password_link(true, "pw"),
        json!({"command": "link_existing_code", "schema_version": 1,
               "link_code": "abc", "acknowledge_password_removal": true}),
        json!({"command": "account", "schema_version": 1}),
        json!({"command": "add_method", "schema_version": 1, "provider": "dev"}),
        json!({"command": "unlink", "schema_version": 1, "identity_id": 7}),
        json!({"command": "sign_out", "schema_version": 1}),
    ];
    for request in good {
        assert!(
            serde_json::from_value::<SignInCommand>(request.clone()).is_ok(),
            "{request}"
        );
    }
    let extra = json!({"command": "inspect", "schema_version": 1, "extra": 1});
    assert!(serde_json::from_value::<SignInCommand>(extra).is_err());
    let v2: SignInCommand =
        serde_json::from_value(json!({"command": "inspect", "schema_version": 2})).unwrap();
    assert_eq!(v2.schema_version(), 2);
}

#[tokio::test]
async fn unsupported_schema_is_refused() {
    let fx = Fixture::new(true, true).await;
    let reply = fx
        .run(
            json!({"command": "inspect", "schema_version": 2}),
            &SilentBrowser::default(),
        )
        .await;
    assert_eq!(reply, Err(SignInFailure::UnsupportedSchema));
}

#[tokio::test]
async fn inspect_without_sign_in_server_is_unavailable() {
    let fx = Fixture::new(false, true).await;
    let status = fx.inspect().await.unwrap();
    assert!(!status.available);
    assert!(status.providers.is_empty());
    let reply = fx.run(begin(), &SilentBrowser::default()).await;
    assert_eq!(reply, Err(SignInFailure::Unavailable));
}

#[tokio::test]
async fn offline_inspect_still_restores_the_session() {
    let fx = Fixture::new(false, true).await;
    fx.store_entry("r0");
    assert!(!fx.has_session());
    let status = fx.inspect().await.unwrap();
    assert!(!status.available);
    assert!(fx.has_session(), "Play must know the player is signed in");
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
}

#[tokio::test]
async fn begin_signs_in_and_stores_the_session() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, signed_in_body("r1")).await;
    let status = fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
    assert!(status.available);
    assert!(fx.has_session());
    assert_eq!(
        fx.store.get(HOST).unwrap().expect("stored").refresh_token,
        "r1"
    );
}

#[tokio::test]
async fn begin_needs_link_then_link_new() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, needs_link_body()).await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/new"))
        .and(body_partial_json(json!({"link_grant": "lg1"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("r2")))
        .mount(&fx.server)
        .await;
    let first = fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    match first.state {
        SignInStateView::NeedsLink { hint, .. } => assert!(hint.contains("dev provider")),
        other => panic!("expected needs_link, got {other:?}"),
    }
    assert!(!fx.has_session());
    let done = fx
        .run(
            json!({"command": "link_new", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await
        .unwrap();
    assert!(matches!(done.state, SignInStateView::SignedIn { .. }));
    assert!(fx.has_session());
    assert_eq!(fx.store.get(HOST).unwrap().unwrap().refresh_token, "r2");
}

#[tokio::test]
async fn link_without_pending_choice_is_refused() {
    let fx = Fixture::new(true, true).await;
    let reply = fx
        .run(
            json!({"command": "link_new", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await;
    assert_eq!(reply, Err(SignInFailure::NoPendingLink));
}

#[tokio::test]
async fn wrong_password_keeps_the_pending_link() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, needs_link_body()).await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/existing"))
        .and(body_partial_json(json!({"password": "bad"})))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({"error": "invalid_credentials"})),
        )
        .mount(&fx.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/existing"))
        .and(body_partial_json(json!({"password": "good"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("r3")))
        .mount(&fx.server)
        .await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    let silent = SilentBrowser::default();
    let wrong = fx.run(password_link(true, "bad"), &silent).await;
    assert_eq!(
        wrong,
        Err(SignInFailure::Server {
            code: "invalid_credentials".to_owned(),
            retry_after: None
        })
    );
    let right = fx.run(password_link(true, "good"), &silent).await.unwrap();
    assert!(matches!(right.state, SignInStateView::SignedIn { .. }));
}

#[tokio::test]
async fn link_without_ack_sends_nothing() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, needs_link_body()).await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/existing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("r4")))
        .mount(&fx.server)
        .await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    let silent = SilentBrowser::default();
    let refused = fx.run(password_link(false, "pw"), &silent).await;
    assert_eq!(refused, Err(SignInFailure::PasswordRemovalNotAcknowledged));
    let code = fx
        .run(
            json!({"command": "link_existing_code", "schema_version": 1,
                   "link_code": "abc", "acknowledge_password_removal": false}),
            &silent,
        )
        .await;
    assert_eq!(code, Err(SignInFailure::PasswordRemovalNotAcknowledged));
    assert!(fx.requests_to("/auth/v1/link/existing").await.is_empty());
    assert!(matches!(
        fx.inspect().await.unwrap().state,
        SignInStateView::NeedsLink { .. }
    ));

    fx.run(password_link(true, "pw"), &silent).await.unwrap();
    let sent = fx.requests_to("/auth/v1/link/existing").await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["password_removal_ack"], json!(true));
}

#[tokio::test]
async fn status_carries_play_refusal() {
    let fx = Fixture::new(true, true).await;
    assert_eq!(fx.inspect().await.unwrap().play_refusal, None);
    fx.state
        .lock()
        .unwrap()
        .set_play_refusal(Some("ticket_unavailable"));
    assert_eq!(
        fx.inspect().await.unwrap().play_refusal,
        Some("ticket_unavailable")
    );
}

#[tokio::test]
async fn second_begin_is_busy_and_cancel_ends_the_first() {
    let fx = Fixture::new(true, true).await;
    let silent = SilentBrowser::default();
    let first = fx.run(begin(), &silent);
    let rest = async {
        for _ in 0..200 {
            if !silent.urls.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let second = fx.run(begin(), &SilentBrowser::default()).await;
        let cancel = fx
            .run(
                json!({"command": "cancel", "schema_version": 1}),
                &SilentBrowser::default(),
            )
            .await;
        (second, cancel)
    };
    let (first, (second, cancel)) = tokio::join!(first, rest);
    assert_eq!(second, Err(SignInFailure::Busy));
    assert!(cancel.is_ok());
    assert_eq!(first, Err(SignInFailure::Cancelled));
}

#[tokio::test]
async fn browser_failure_returns_the_url() {
    let fx = Fixture::new(true, true).await;
    match fx.run(begin(), &FailingBrowser).await {
        Err(SignInFailure::BrowserFailed { url }) => {
            assert!(url.starts_with(&format!("{}/", fx.server.uri())), "{url}");
        }
        other => panic!("expected browser_failed, got {other:?}"),
    }
    // The failed flow must not leave the driver busy.
    let again = fx.run(begin(), &FailingBrowser).await;
    assert!(matches!(again, Err(SignInFailure::BrowserFailed { .. })));
}

#[tokio::test]
async fn rate_limited_carries_retry_after() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, needs_link_body()).await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/new"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(json!({"error": "rate_limited", "retry_after": 30})),
        )
        .mount(&fx.server)
        .await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    let reply = fx
        .run(
            json!({"command": "link_new", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await;
    assert_eq!(
        reply,
        Err(SignInFailure::Server {
            code: "rate_limited".to_owned(),
            retry_after: Some(30)
        })
    );
}

#[tokio::test]
async fn reauth_required_passes_through_as_its_own_failure() {
    let fx = Fixture::new(true, true).await;
    fx.store_entry("r0");
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({"grant_type": "refresh_token"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("r5")))
        .mount(&fx.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/account"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({"error": "reauth_required"})))
        .mount(&fx.server)
        .await;
    let reply = fx
        .run(
            json!({"command": "account", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await;
    assert_eq!(
        reply,
        Err(SignInFailure::Server {
            code: "reauth_required".to_owned(),
            retry_after: None
        })
    );
}

#[tokio::test]
async fn account_when_signed_out_is_a_signed_out_failure() {
    let fx = Fixture::new(true, true).await;
    let reply = fx
        .run(
            json!({"command": "account", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await;
    assert_eq!(
        reply,
        Err(SignInFailure::Server {
            code: "signed_out".to_owned(),
            retry_after: None
        })
    );
}

#[tokio::test]
async fn unlink_replies_with_the_remaining_identities_and_times() {
    let fx = Fixture::new(true, true).await;
    fx.store_entry("r0");
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({"grant_type": "refresh_token"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("r6")))
        .mount(&fx.server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/auth/v1/identities/7"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&fx.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_name": "alice", "display_name": "Alice",
            "identities": [{"identity_id": 8, "provider": "dev", "display_name": "Dev",
                            "created_at": 1_700_000_000_000_i64,
                            "last_used_at": 1_700_000_100_000_i64}],
        })))
        .mount(&fx.server)
        .await;
    let status = fx
        .run(
            json!({"command": "unlink", "schema_version": 1, "identity_id": 7}),
            &SilentBrowser::default(),
        )
        .await
        .unwrap();
    let account = status.account.expect("unlink fills the account view");
    assert_eq!(account.identities.len(), 1);
    assert_eq!(account.identities[0].identity_id, 8);
    assert_eq!(account.identities[0].created_at, 1_700_000_000_000);
    assert_eq!(account.identities[0].last_used_at, 1_700_000_100_000);
    assert_eq!(fx.requests_to("/auth/v1/identities/7").await.len(), 1);
}

#[tokio::test]
async fn sign_out_clears_state_offline() {
    let fx = Fixture::new(false, false).await;
    fx.store_entry("r0");
    // Offline inspect restores the stored sign-in first.
    fx.inspect().await.unwrap();
    assert!(fx.has_session());
    let status = fx
        .run(
            json!({"command": "sign_out", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await
        .unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert!(!fx.has_session());
    assert!(fx.store.get(HOST).unwrap().is_none());
}

/// The `SignOut` arm's work with no login server (`Site.server` is `None`): it
/// still signs out, clears the stored entry and drops the sign-in. The engine
/// reaches that arm only with no default server, so this calls `sign_out`
/// directly; `run` with a `SignOut` and no server is `Unavailable` otherwise.
#[tokio::test]
async fn sign_out_works_with_no_login_server() {
    let fx = Fixture::new(false, false).await;
    fx.store_entry("r0");
    let base = url::Url::parse(&format!("{}/", fx.server.uri())).unwrap();
    let session = Arc::new(SignIn::new(base, reqwest::Client::new(), fx.store.clone()));
    fx.state.lock().unwrap().set_sign_in(Some(session.clone()));
    let site = Site {
        state: &fx.state,
        server: None,
        discovery: None,
        base: None,
    };
    let status = fx.driver.sign_out(&site, Some(session)).await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert!(fx.store.get(HOST).unwrap().is_none());
    assert!(!fx.has_session());
}

static LOCAL: AtomicUsize = AtomicUsize::new(0);

#[test]
fn relay_lease_counts() {
    use std::sync::atomic::Ordering;
    assert_eq!(LOCAL.load(Ordering::SeqCst), 0);
    let lease = RelayLease::on(&LOCAL);
    assert_eq!(LOCAL.load(Ordering::SeqCst), 1);
    drop(lease);
    assert_eq!(LOCAL.load(Ordering::SeqCst), 0);
}

#[path = "driver_handover_tests.rs"]
mod handover;

#[path = "driver_host_tests.rs"]
mod host;

#[path = "driver_epoch_tests.rs"]
mod epoch;
