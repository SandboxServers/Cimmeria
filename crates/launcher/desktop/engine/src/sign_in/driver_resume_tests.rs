//! Tests of a parked browser flow and `resume` (CT-89, contract C18.1). A browser
//! that will not open parks the flow; the player's redirect, sent by hand to the
//! parked listener, finishes it.

use url::Url;
use wiremock::{Request, Respond};

use super::*;
use crate::sign_in::oauth::SIGN_IN_TIMEOUT;

/// The `link/start` reply, which echoes the flow values into its `authorize_url`
/// the way the server does.
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
            .append_pair("code_challenge_method", "S256");
        ResponseTemplate::new(200).set_body_json(json!({ "authorize_url": url.as_str() }))
    }
}

fn resume() -> Value {
    json!({"command": "resume", "schema_version": 1})
}

fn add_method() -> Value {
    json!({"command": "add_method", "schema_version": 1, "provider": "dev"})
}

/// The redirect the player's browser would send for the parked `url`, with a code.
fn callback_of(url: &str) -> String {
    let parsed = Url::parse(url).unwrap();
    let query = |name: &str| {
        parsed
            .query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
            .unwrap()
    };
    format!("{}?code=c&state={}", query("redirect_uri"), query("state"))
}

/// Parks a `begin` whose browser will not open, and returns the URL it failed with.
async fn park_begin(fx: &Fixture) -> String {
    match fx.run(begin(), &FailingBrowser).await {
        Err(SignInFailure::BrowserFailed { url }) => url,
        other => panic!("expected browser_failed, got {other:?}"),
    }
}

#[tokio::test]
async fn a_failed_browser_parks_the_sign_in_and_resume_finishes_it() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, signed_in_body("r1")).await;
    let url = park_begin(&fx).await;
    assert!(fx.inspect().await.unwrap().waiting);
    let silent = SilentBrowser::default();
    let (resumed, callback) = tokio::join!(
        fx.run(resume(), &silent),
        reqwest::Client::new().get(callback_of(&url)).send(),
    );
    assert!(callback.is_ok(), "the parked listener takes the redirect");
    let status = resumed.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
    assert!(!status.waiting);
    assert_eq!(fx.store.get(HOST).unwrap().unwrap().refresh_token, "r1");
}

#[tokio::test]
async fn a_failed_browser_parks_add_method_and_resume_finishes_it() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, signed_in_body("r2")).await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/start"))
        .respond_with(LinkStartEcho {
            base: format!("{}/", fx.server.uri()),
        })
        .mount(&fx.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_name": "alice", "display_name": "Alice", "identities": [],
        })))
        .mount(&fx.server)
        .await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    let url = match fx.run(add_method(), &FailingBrowser).await {
        Err(SignInFailure::BrowserFailed { url }) => url,
        other => panic!("expected browser_failed, got {other:?}"),
    };
    assert!(url.starts_with(&format!("{}/auth/v1/authorize?", fx.server.uri())));
    let silent = SilentBrowser::default();
    let (resumed, callback) = tokio::join!(
        fx.run(resume(), &silent),
        reqwest::Client::new().get(callback_of(&url)).send(),
    );
    assert!(callback.is_ok(), "the parked listener takes the redirect");
    let status = resumed.unwrap();
    assert!(status.account.is_some(), "resume reads the account");
    assert!(!status.waiting);
}

#[tokio::test]
async fn resume_with_nothing_parked_is_no_pending_flow() {
    let fx = Fixture::new(true, true).await;
    let resumed = fx.run(resume(), &SilentBrowser::default()).await;
    assert_eq!(resumed, Err(SignInFailure::NoPendingFlow));
}

#[tokio::test]
async fn cancel_clears_a_parked_flow() {
    let fx = Fixture::new(true, true).await;
    let url = park_begin(&fx).await;
    let cancel = json!({"command": "cancel", "schema_version": 1});
    fx.run(cancel, &SilentBrowser::default()).await.unwrap();
    let resumed = fx.run(resume(), &SilentBrowser::default()).await;
    assert_eq!(resumed, Err(SignInFailure::NoPendingFlow));
    // The listener is closed, so the redirect is refused.
    let callback = reqwest::Client::new().get(callback_of(&url)).send().await;
    assert!(callback.is_err());
}

#[tokio::test]
async fn a_parked_flow_expires() {
    let fx = Fixture::new(true, true).await;
    park_begin(&fx).await;
    fx.driver.flow.backdate_parked(SIGN_IN_TIMEOUT);
    assert!(!fx.inspect().await.unwrap().waiting);
    let resumed = fx.run(resume(), &SilentBrowser::default()).await;
    assert_eq!(resumed, Err(SignInFailure::NoPendingFlow));
}

#[tokio::test]
async fn sign_out_voids_a_parked_flow() {
    let fx = Fixture::new(true, true).await;
    park_begin(&fx).await;
    let sign_out = json!({"command": "sign_out", "schema_version": 1});
    fx.run(sign_out, &SilentBrowser::default()).await.unwrap();
    let resumed = fx.run(resume(), &SilentBrowser::default()).await;
    assert_eq!(resumed, Err(SignInFailure::NoPendingFlow));
    assert!(fx.store.get(HOST).unwrap().is_none());
}
