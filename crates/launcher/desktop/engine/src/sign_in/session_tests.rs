//! Tests of the sign-in session (`session.rs`) against a `wiremock` server that
//! stands in for `/auth/v1`. They use the shipped HTTP client and a memory store:
//! no real network and no OS keyring.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use serde_json::{json, Value};
use url::Url;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use super::api::AccountView;
use super::discovery;
use super::host_slot::Epoch;
use super::oauth::{challenge_for, CodeGrant};
use super::session::{scheme_allowed, Outcome, SignIn};
use super::store::{CredentialStore, MemoryStore, Stored};
use super::SignInError;

/// The host the stored entries are keyed by in these tests.
const HOST: &str = "127.0.0.1";

fn base_of(server: &MockServer) -> Url {
    Url::parse(&format!("{}/", server.uri())).expect("wiremock URI parses")
}

/// An operation epoch nobody bumps: the fence of a step that is never superseded.
fn fresh() -> AtomicU64 {
    AtomicU64::new(0)
}

fn signed_in_body(access: &str, refresh: &str, expires_in: u64) -> Value {
    json!({
        "result": "signed_in",
        "access_token": access,
        "expires_in": expires_in,
        "refresh_token": refresh,
        "account": {
            "account_name": "alice",
            "display_name": "Alice",
            "identities": [],
        },
    })
}

fn store_with(refresh: &str) -> Arc<MemoryStore> {
    let store = Arc::new(MemoryStore::default());
    store
        .set(
            HOST,
            &Stored {
                refresh_token: refresh.to_owned(),
                account_name: "alice".to_owned(),
                display_name: "Alice".to_owned(),
            },
        )
        .unwrap();
    store
}

fn grant() -> CodeGrant {
    CodeGrant {
        code: "c".to_owned(),
        verifier: "v".to_owned(),
        redirect_uri: "http://127.0.0.1:1/cb".to_owned(),
    }
}

/// Mounts the `/token` refresh that a restored session makes on first use.
async fn mount_refresh(server: &MockServer, access: &str, refresh: &str) {
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({ "grant_type": "refresh_token" })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(signed_in_body(access, refresh, 900)),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn complete_signed_in_saves_the_refresh_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(
            json!({ "grant_type": "authorization_code", "code": "c" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a1", "r1", 900)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/account"))
        .and(header("authorization", "Bearer a1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_name": "alice",
            "display_name": "Alice",
            "identities": [],
        })))
        .mount(&server)
        .await;

    let store = Arc::new(MemoryStore::default());
    let session = SignIn::new(base_of(&server), discovery::client(), store.clone());
    let counter = fresh();
    match session.complete(grant(), &Epoch::new(&counter, 0)).await {
        Ok(Outcome::SignedIn(summary)) => assert_eq!(summary.account_name, "alice"),
        other => panic!("expected SignedIn, got {other:?}"),
    }
    assert_eq!(
        store.get(HOST).unwrap().expect("stored").refresh_token,
        "r1"
    );
    let account: AccountView = session
        .account()
        .await
        .expect("account with the access token");
    assert_eq!(account.account_name, "alice");
}

#[tokio::test]
async fn needs_link_then_link_new() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": "needs_link",
            "link_grant": "lg1",
            "identity": { "provider": "dev", "display_name": "Dev" },
            "hint": "Sign in with the dev provider.",
            "signup_open": true,
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/new"))
        .and(body_partial_json(json!({ "link_grant": "lg1" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a2", "r2", 900)))
        .mount(&server)
        .await;

    let store = Arc::new(MemoryStore::default());
    let session = SignIn::new(base_of(&server), discovery::client(), store.clone());
    let counter = fresh();
    let link = match session.complete(grant(), &Epoch::new(&counter, 0)).await {
        Ok(Outcome::NeedsLink(view)) => view,
        other => panic!("expected NeedsLink, got {other:?}"),
    };
    assert!(
        store.get(HOST).unwrap().is_none(),
        "nothing is stored before the link"
    );
    let outcome = session
        .link_new(&link.link_grant, &Epoch::new(&counter, 0))
        .await
        .expect("link/new signs in");
    assert!(matches!(outcome, Outcome::SignedIn(_)));
    assert_eq!(
        store.get(HOST).unwrap().expect("stored").refresh_token,
        "r2"
    );
}

#[tokio::test]
async fn access_token_refreshes_near_expiry_and_rotates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(
            json!({ "grant_type": "authorization_code" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a1", "r1", 30)))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({
            "grant_type": "refresh_token",
            "refresh_token": "r1",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a2", "r2", 30)))
        .expect(1)
        .mount(&server)
        .await;

    let store = Arc::new(MemoryStore::default());
    let session = SignIn::new(base_of(&server), discovery::client(), store.clone());
    let counter = fresh();
    session
        .complete(grant(), &Epoch::new(&counter, 0))
        .await
        .expect("signed in");
    // 30 s left is inside the 60 s margin, so the next call refreshes.
    assert_eq!(session.access_token().await.expect("refreshed"), "a2");
    assert_eq!(
        store.get(HOST).unwrap().expect("stored").refresh_token,
        "r2"
    );
}

#[tokio::test]
async fn invalid_grant_signs_out() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" })))
        .mount(&server)
        .await;

    let store = store_with("r0");
    let session = SignIn::restore(base_of(&server), discovery::client(), store.clone())
        .expect("a stored entry restores");
    assert_eq!(
        session.access_token().await,
        Err(SignInError::Server("signed_out".to_owned()))
    );
    assert!(
        store.get(HOST).unwrap().is_none(),
        "the revoked entry is deleted"
    );
}

#[tokio::test]
async fn invalid_grant_keeps_a_newer_sign_in() {
    // The refused refresh is answered after a reauth stored a new family (T2)
    // on the same host: the old instance must not delete that entry.
    let server = MockServer::start().await;
    let store = store_with("r1");
    let racing = store.clone();
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .respond_with(move |_: &Request| {
            racing
                .set(
                    HOST,
                    &Stored {
                        refresh_token: "r2".to_owned(),
                        account_name: "bob".to_owned(),
                        display_name: "Bob".to_owned(),
                    },
                )
                .unwrap();
            ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" }))
        })
        .mount(&server)
        .await;

    let session = SignIn::restore(base_of(&server), discovery::client(), store.clone())
        .expect("a stored entry restores");
    assert_eq!(
        session.access_token().await,
        Err(SignInError::Server("signed_out".to_owned()))
    );
    assert_eq!(
        store.get(HOST).unwrap().map(|s| s.refresh_token),
        Some("r2".to_owned()),
        "the newer entry is kept"
    );
    assert!(session.summary().await.is_none());
}

#[tokio::test]
async fn sign_out_works_offline() {
    // A server that is shut down: the port refuses connections.
    let server = MockServer::start().await;
    let base = base_of(&server);
    drop(server);

    let store = store_with("r0");
    let session =
        SignIn::restore(base, discovery::client(), store.clone()).expect("a stored entry restores");
    session.sign_out().await;
    assert!(
        store.get(HOST).unwrap().is_none(),
        "the stored entry is deleted offline"
    );
    assert!(session.summary().await.is_none());
}

#[test]
fn stored_debug_hides_the_token() {
    let stored = Stored {
        refresh_token: "secret-refresh-value".to_owned(),
        account_name: "alice".to_owned(),
        display_name: "Alice".to_owned(),
    };
    let text = format!("{stored:?}");
    assert!(
        !text.contains("secret-refresh-value"),
        "Debug leaked: {text}"
    );
    assert!(text.contains("alice"));
}

#[tokio::test]
async fn launch_ticket_heartbeat_end() {
    let server = MockServer::start().await;
    mount_refresh(&server, "a1", "r1").await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/launch-ticket"))
        .and(header("authorization", "Bearer a1"))
        .and(body_partial_json(json!({ "launch_id": "L1" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_name": "alice",
            "ticket": "t1",
            "expires_in": 60,
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/launch-ticket/heartbeat"))
        .and(header("authorization", "Bearer a1"))
        .and(body_partial_json(json!({ "launch_id": "L1" })))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/launch-ticket/end"))
        .and(header("authorization", "Bearer a1"))
        .and(body_partial_json(json!({ "launch_id": "L1" })))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let session = SignIn::restore(base_of(&server), discovery::client(), store_with("r0"))
        .expect("a stored entry restores");
    let ticket = session.launch_ticket("L1").await.expect("ticket");
    assert_eq!(ticket.ticket, "t1");
    assert_eq!(session.heartbeat("L1").await, Ok(true));
    session.end_ticket("L1").await.expect("ended");
}

#[tokio::test]
async fn heartbeat_for_an_unknown_launch_is_false() {
    let server = MockServer::start().await;
    mount_refresh(&server, "a1", "r1").await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/launch-ticket/heartbeat"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({ "error": "not_found" })))
        .mount(&server)
        .await;

    let session = SignIn::restore(base_of(&server), discovery::client(), store_with("r0"))
        .expect("a stored entry restores");
    assert_eq!(session.heartbeat("gone").await, Ok(false));
}

#[tokio::test]
async fn rate_limit_error_carries_retry_after() {
    let server = MockServer::start().await;
    mount_refresh(&server, "a1", "r1").await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/launch-ticket"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "error": "rate_limited",
            "retry_after": 17,
        })))
        .mount(&server)
        .await;

    let session = SignIn::restore(base_of(&server), discovery::client(), store_with("r0"))
        .expect("a stored entry restores");
    match session.launch_ticket("L1").await {
        Err(SignInError::Server(text)) => {
            assert!(text.contains("retry_after=17"), "text was {text}");
        }
        other => panic!("expected a server error, got {other:?}"),
    }
}

/// Answers `link/start` the way the server does: it echoes the flow values into
/// the `authorize_url` it returns.
struct LinkStartEcho {
    base: String,
}

impl Respond for LinkStartEcho {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&request.body).expect("JSON body");
        let field = |name: &str| body[name].as_str().unwrap_or_default().to_owned();
        let mut url = Url::parse(&format!("{}auth/v1/authorize", self.base)).expect("URL");
        url.query_pairs_mut()
            .append_pair("provider", &field("provider"))
            .append_pair("redirect_uri", &field("redirect_uri"))
            .append_pair("state", &field("state"))
            .append_pair("code_challenge", &field("code_challenge"))
            .append_pair("code_challenge_method", "S256")
            .append_pair("intent", "i1");
        ResponseTemplate::new(200).set_body_json(json!({ "authorize_url": url.as_str() }))
    }
}

#[tokio::test]
async fn add_method_sends_the_flow_values_to_link_start() {
    let server = MockServer::start().await;
    mount_refresh(&server, "a1", "r1").await;
    let base = base_of(&server);
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/start"))
        .and(header("authorization", "Bearer a1"))
        .respond_with(LinkStartEcho {
            base: base.to_string(),
        })
        .mount(&server)
        .await;

    let session = SignIn::restore(base, discovery::client(), store_with("r0"))
        .expect("a stored entry restores");
    let flow = session.add_method("dev").await.expect("link/start");

    let authorize = Url::parse(&flow.authorize_url).expect("authorize URL");
    let query = |name: &str| {
        authorize
            .query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
            .unwrap_or_default()
    };
    let redirect_uri = query("redirect_uri");
    let state = query("state");
    let code_challenge = query("code_challenge");
    assert_eq!(query("intent"), "i1");

    // The browser is the caller here: it follows the redirect to the listener.
    let callback = format!("{redirect_uri}?code=c&state={state}");
    let browser = tokio::spawn(async move { reqwest::get(callback).await });
    let grant = flow
        .finish()
        .await
        .expect("the redirect completes the flow");
    browser.await.expect("task").expect("callback GET");

    assert_eq!(grant.code, "c");
    assert_eq!(challenge_for(&grant.verifier), code_challenge);
}

#[tokio::test]
async fn oversized_reply_is_protocol() {
    let server = MockServer::start().await;
    mount_refresh(&server, "a1", "r1").await;
    let big = "x".repeat(70 * 1024);
    Mock::given(method("GET"))
        .and(path("/auth/v1/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_name": "alice",
            "display_name": big,
            "identities": [],
        })))
        .mount(&server)
        .await;

    let session = SignIn::restore(base_of(&server), discovery::client(), store_with("r0"))
        .expect("a stored entry restores");
    assert_eq!(session.account().await.err(), Some(SignInError::Protocol));
}

#[tokio::test]
async fn a_rejected_access_token_is_refreshed_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({
            "grant_type": "refresh_token",
            "refresh_token": "r0",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a1", "r1", 900)))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({
            "grant_type": "refresh_token",
            "refresh_token": "r1",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a2", "r2", 900)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/account"))
        .and(header("authorization", "Bearer a1"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "error": "invalid_token" })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/account"))
        .and(header("authorization", "Bearer a2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_name": "alice",
            "display_name": "Alice",
            "identities": [],
        })))
        .mount(&server)
        .await;

    let store = store_with("r0");
    let session = SignIn::restore(base_of(&server), discovery::client(), store.clone())
        .expect("a stored entry restores");
    assert!(
        session.account().await.is_ok(),
        "the refreshed token is used"
    );
    assert_eq!(store.get(HOST).unwrap().unwrap().refresh_token, "r2");
}

#[tokio::test]
async fn concurrent_calls_spend_the_refresh_token_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({
            "grant_type": "refresh_token",
            "refresh_token": "r0",
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(signed_in_body("a1", "r1", 900))
                .set_delay(std::time::Duration::from_millis(300)),
        )
        .mount(&server)
        .await;

    let session = SignIn::restore(base_of(&server), discovery::client(), store_with("r0"))
        .expect("a stored entry restores");
    let (first, second) = tokio::join!(session.access_token(), session.access_token());
    assert_eq!(first.expect("first caller"), "a1");
    assert_eq!(second.expect("second caller"), "a1");
    let posts = server
        .received_requests()
        .await
        .expect("request recording is on")
        .len();
    assert_eq!(posts, 1, "the refresh token was spent more than once");
}

#[tokio::test]
async fn link_existing_sends_the_removal_ack() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/existing"))
        .and(body_partial_json(json!({
            "link_grant": "lg1",
            "account_name": "alice",
            "password": "pw",
            "password_removal_ack": true,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a1", "r1", 900)))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/existing"))
        .and(body_partial_json(json!({
            "link_grant": "lg2",
            "link_code": "ABCDE23456",
            "password_removal_ack": true,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("a1", "r1", 900)))
        .mount(&server)
        .await;

    let session = SignIn::new(
        base_of(&server),
        discovery::client(),
        Arc::new(MemoryStore::default()),
    );
    let counter = fresh();
    let by_password = session
        .link_existing_password("lg1", "alice", "pw", &Epoch::new(&counter, 0))
        .await;
    let by_code = session
        .link_existing_code("lg2", "ABCDE23456", &Epoch::new(&counter, 0))
        .await;
    assert!(
        matches!(by_password, Ok(Outcome::SignedIn(_))),
        "{by_password:?}"
    );
    assert!(matches!(by_code, Ok(Outcome::SignedIn(_))), "{by_code:?}");
}

#[test]
fn scheme_allowed_needs_https_outside_debug() {
    let plain = Url::parse("http://play.example.org/").unwrap();
    let secure = Url::parse("https://play.example.org/").unwrap();
    assert!(!scheme_allowed(&plain, false));
    assert!(scheme_allowed(&plain, true));
    assert!(scheme_allowed(&secure, false));
}
