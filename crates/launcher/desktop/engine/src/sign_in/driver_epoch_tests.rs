//! Driver tests of the operation epoch (CT-87): a Sign out or a Cancel voids every
//! sign-in step still in flight, and a server switch stores a late completion
//! without installing it. Each test interleaves the slow step with the interrupting
//! command through `tokio::join!`; no delay is longer than 500 ms.

use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::NativeCommand;

fn sign_out() -> Value {
    json!({"command": "sign_out", "schema_version": 1})
}

fn cancel() -> Value {
    json!({"command": "cancel", "schema_version": 1})
}

fn link_new() -> Value {
    json!({"command": "link_new", "schema_version": 1})
}

/// Waits until `browser` has been asked to open a flow, so the flow is waiting.
async fn until_opened(browser: &SilentBrowser) {
    for _ in 0..200 {
        if !browser.urls.lock().unwrap().is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the flow never opened the browser");
}

/// Waits until `server` has received a request on `route`.
async fn until_requested(server: &MockServer, route: &str) {
    for _ in 0..200 {
        let seen = server.received_requests().await.unwrap_or_default();
        if seen.iter().any(|r| r.url.path() == route) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the server never received {route}");
}

/// The loopback callback a browser follows once the player approves the flow at `url`.
fn approval_of(url: &str) -> String {
    let parsed = url::Url::parse(url).unwrap();
    let query = |name: &str| {
        parsed
            .query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
            .unwrap()
    };
    format!("{}?code=c&state={}", query("redirect_uri"), query("state"))
}

/// The player approves the flow that `browser` opened. The flow may already be gone.
async fn approve(browser: &SilentBrowser) {
    let url = browser.urls.lock().unwrap()[0].clone();
    let _ = reqwest::Client::new().get(approval_of(&url)).send().await;
}

/// The token route, with its reply held back by `delay_ms`.
async fn mount_slow_token(server: &MockServer, body: Value, delay_ms: u64) {
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(
            json!({"grant_type": "authorization_code"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(body)
                .set_delay(Duration::from_millis(delay_ms)),
        )
        .mount(server)
        .await;
}

/// Points the login server at a second mock auth server, as if the player had
/// switched servers on the login screen. The returned server must stay alive.
async fn switch_to_second_server(fx: &Fixture) -> MockServer {
    let other = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "api": 1,
            "soap_relay": true,
            "signup_open": true,
            "providers": [{"id": "dev", "display_name": "Dev sign-in"}],
            "links": {"website": null, "account": null}
        })))
        .mount(&other)
        .await;
    std::env::set_var(AUTH_BASE_ENV, format!("{}/", other.uri()));
    fx.state
        .lock()
        .unwrap()
        .dispatch(NativeCommand::SaveClientOptions {
            schema_version: 1,
            expected_revision: 0,
            login_servers: vec![LoginServer {
                name: "Other".to_owned(),
                url: "https://other.example/login".to_owned(),
            }],
            client_patches_enabled: false,
        })
        .expect("the server list saves");
    other
}

#[tokio::test]
async fn sign_out_during_the_browser_wait_voids_the_callback() {
    let fx = Fixture::new(true, true).await;
    let browser = SilentBrowser::default();
    let (reply, _) = tokio::join!(fx.run(begin(), &browser), async {
        until_opened(&browser).await;
        fx.run(sign_out(), &SilentBrowser::default()).await.unwrap();
        approve(&browser).await;
    });
    assert_eq!(reply, Err(SignInFailure::Cancelled));
    assert!(!fx.has_session());
    assert!(fx.store.get(HOST).is_none());
    assert!(fx.requests_to("/auth/v1/token").await.is_empty());
}

#[tokio::test]
async fn sign_out_during_the_token_exchange_discards_the_reply() {
    let fx = Fixture::new(true, true).await;
    mount_slow_token(&fx.server, signed_in_body("r9"), 400).await;
    let browser = CallbackBrowser::default();
    let (reply, _) = tokio::join!(fx.run(begin(), &browser), async {
        until_requested(&fx.server, "/auth/v1/token").await;
        fx.run(sign_out(), &SilentBrowser::default()).await.unwrap();
    });
    assert_eq!(reply, Err(SignInFailure::Cancelled));
    assert!(!fx.has_session());
    assert!(fx.store.get(HOST).is_none());
    let logouts = fx.requests_to("/auth/v1/logout").await;
    assert_eq!(logouts, vec![json!({"refresh_token": "r9"})]);
}

#[tokio::test]
async fn sign_out_during_a_link_call_discards_the_reply() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, needs_link_body()).await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/new"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(signed_in_body("r8"))
                .set_delay(Duration::from_millis(400)),
        )
        .mount(&fx.server)
        .await;
    let browser = SilentBrowser::default();
    let (reply, _) = tokio::join!(fx.run(link_new(), &browser), async {
        until_requested(&fx.server, "/auth/v1/link/new").await;
        fx.run(sign_out(), &browser).await.unwrap();
    });
    assert_eq!(reply, Err(SignInFailure::Cancelled));
    assert!(fx.store.get(HOST).is_none());
    assert!(!fx.has_session());
    let status = fx.inspect().await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert_eq!(fx.requests_to("/auth/v1/link/new").await.len(), 1);
}

#[tokio::test]
async fn cancel_after_the_callback_discards_the_reply() {
    let fx = Fixture::new(true, true).await;
    mount_slow_token(&fx.server, signed_in_body("r9"), 400).await;
    let browser = CallbackBrowser::default();
    let (reply, _) = tokio::join!(fx.run(begin(), &browser), async {
        until_requested(&fx.server, "/auth/v1/token").await;
        fx.run(cancel(), &SilentBrowser::default()).await.unwrap();
    });
    assert_eq!(reply, Err(SignInFailure::Cancelled));
    assert!(fx.store.get(HOST).is_none());
    assert!(!fx.has_session());
}

#[tokio::test]
async fn a_server_switch_mid_flow_stores_but_does_not_install() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, signed_in_body("r7")).await;
    let browser = SilentBrowser::default();
    let (_, _) = tokio::join!(fx.run(begin(), &browser), async {
        until_opened(&browser).await;
        let _other = switch_to_second_server(&fx).await;
        approve(&browser).await;
    });
    assert!(!fx.has_session());
}
