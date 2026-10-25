//! Sign-in tests. Every server is a local `wiremock` instance or a loopback
//! listener, so nothing reaches the lab or the internet.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::json;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::discovery::{self, Discovery, DiscoveryCache, ProviderView};
use super::oauth::{self, challenge_for, new_pkce};
use super::SignInError;
use cimmeria_launcher_core::client_setup::login_session::Links;

/// Serialises the tests that read or change `CIMMERIA_LAUNCHER_AUTH_BASE`.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn base_of(server: &MockServer) -> Url {
    Url::parse(&format!("{}/", server.uri())).expect("wiremock URI parses")
}

fn providers_body() -> serde_json::Value {
    json!({
        "api": 1,
        "soap_relay": true,
        "signup_open": false,
        "providers": [{"id": "dev", "display_name": "Dev sign-in"}],
        "links": {"website": "https://example.org/", "account": null}
    })
}

#[test]
fn auth_base_from_login_urls() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(
        discovery::auth_base("http://play.example.org:8081").map(String::from),
        Some("https://play.example.org/".to_owned())
    );
    assert_eq!(discovery::auth_base("http://203.0.113.5:8081"), None);
    assert_eq!(
        discovery::auth_base("http://localhost:8081").map(String::from),
        Some("https://localhost/".to_owned())
    );
}

#[cfg(not(debug_assertions))]
#[test]
fn release_ignores_the_auth_base_env() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("CIMMERIA_LAUNCHER_AUTH_BASE", "http://127.0.0.1:9/");
    let base = discovery::auth_base("http://play.example.org:8081");
    std::env::remove_var("CIMMERIA_LAUNCHER_AUTH_BASE");
    assert_eq!(
        base.map(String::from),
        Some("https://play.example.org/".to_owned())
    );
}

#[cfg(debug_assertions)]
#[test]
fn debug_env_overrides_auth_base() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("CIMMERIA_LAUNCHER_AUTH_BASE", "http://127.0.0.1:9/");
    let overridden = discovery::auth_base("http://play.example.org:8081");
    std::env::remove_var("CIMMERIA_LAUNCHER_AUTH_BASE");
    assert_eq!(
        overridden.map(String::from),
        Some("http://127.0.0.1:9/".to_owned())
    );
}

#[tokio::test]
async fn probe_parses_providers_and_links() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(providers_body()))
        .mount(&server)
        .await;
    let http = reqwest::Client::new();
    let found = discovery::probe_at(&http, base_of(&server)).await;
    assert_eq!(
        found,
        Some(Discovery {
            auth_base: base_of(&server),
            soap_relay: true,
            signup_open: false,
            providers: vec![ProviderView {
                id: "dev".to_owned(),
                display_name: "Dev sign-in".to_owned(),
            }],
            links: Links {
                website: Some("https://example.org/".to_owned()),
                account: None,
                forums: None,
            },
        })
    );
}

#[tokio::test]
async fn probe_refuses_unknown_api_and_errors() {
    let future_api = MockServer::start().await;
    let mut body = providers_body();
    body["api"] = json!(2);
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&future_api)
        .await;

    let server_error = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server_error)
        .await;

    let http = reqwest::Client::new();
    assert_eq!(discovery::probe_at(&http, base_of(&future_api)).await, None);
    assert_eq!(
        discovery::probe_at(&http, base_of(&server_error)).await,
        None
    );
}

#[tokio::test]
async fn probe_refuses_an_oversized_reply() {
    let server = MockServer::start().await;
    let mut body = providers_body();
    body["providers"][0]["display_name"] = json!("x".repeat(70 * 1024));
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let http = reqwest::Client::new();
    assert_eq!(discovery::probe_at(&http, base_of(&server)).await, None);
}

#[tokio::test]
async fn client_does_not_follow_redirects() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", format!("{}/moved", server.uri()).as_str()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/moved"))
        .respond_with(ResponseTemplate::new(200).set_body_json(providers_body()))
        .expect(0)
        .mount(&server)
        .await;
    assert_eq!(
        discovery::probe_at(&discovery::client(), base_of(&server)).await,
        None
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
#[allow(clippy::await_holding_lock)] // tests only: serialises the process env
async fn probe_cached_reuses_a_fresh_answer() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(providers_body()))
        .expect(1)
        .mount(&server)
        .await;
    std::env::set_var("CIMMERIA_LAUNCHER_AUTH_BASE", base_of(&server).to_string());
    let http = reqwest::Client::new();
    let mut cache = DiscoveryCache::default();
    let t0 = Instant::now();
    let login = "http://play.example.org:8081";
    let first = discovery::probe_cached(&mut cache, &http, login, t0).await;
    let second =
        discovery::probe_cached(&mut cache, &http, login, t0 + Duration::from_secs(1)).await;
    std::env::remove_var("CIMMERIA_LAUNCHER_AUTH_BASE");
    assert!(first.is_some());
    assert_eq!(first, second);
}

#[cfg(debug_assertions)]
#[tokio::test]
#[allow(clippy::await_holding_lock)] // tests only: serialises the process env
async fn probe_cached_does_not_store_a_failed_probe() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(500))
        .expect(2)
        .mount(&server)
        .await;
    std::env::set_var("CIMMERIA_LAUNCHER_AUTH_BASE", base_of(&server).to_string());
    let http = reqwest::Client::new();
    let mut cache = DiscoveryCache::default();
    let t0 = Instant::now();
    let login = "http://play.example.org:8081";
    let first = discovery::probe_cached(&mut cache, &http, login, t0).await;
    let second =
        discovery::probe_cached(&mut cache, &http, login, t0 + Duration::from_secs(1)).await;
    std::env::remove_var("CIMMERIA_LAUNCHER_AUTH_BASE");
    assert_eq!(first, None);
    assert_eq!(second, None);
}

#[cfg(debug_assertions)]
#[test]
fn debug_env_supplies_the_extra_root_pem() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let pem = dir.path().join("root.pem");
    std::fs::write(&pem, b"PEM-BYTES").unwrap();
    std::env::set_var("CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM", &pem);
    let found = discovery::extra_root_pem();
    std::env::set_var(
        "CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM",
        dir.path().join("missing.pem"),
    );
    let missing = discovery::extra_root_pem();
    std::env::remove_var("CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM");
    let unset = discovery::extra_root_pem();
    assert_eq!(found, Some(b"PEM-BYTES".to_vec()));
    assert_eq!(missing, None);
    assert_eq!(unset, None);
}

#[cfg(not(debug_assertions))]
#[test]
fn release_ignores_the_extra_root_env() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let pem = dir.path().join("root.pem");
    std::fs::write(&pem, b"PEM-BYTES").unwrap();
    std::env::set_var("CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM", &pem);
    let found = discovery::extra_root_pem();
    std::env::remove_var("CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM");
    assert_eq!(found, None);
}

#[test]
fn discovery_cache_expires_after_ten_minutes() {
    let mut cache = DiscoveryCache::default();
    let stored = Instant::now();
    let found = Discovery {
        auth_base: Url::parse("https://play.example.org/").unwrap(),
        soap_relay: false,
        signup_open: true,
        providers: Vec::new(),
        links: Links::default(),
    };
    cache.insert("http://play.example.org:8081", stored, found.clone());
    let before = stored + Duration::from_secs(599);
    let after = stored + Duration::from_secs(600);
    assert_eq!(
        cache.get("http://play.example.org:8081", before),
        Some(found)
    );
    assert_eq!(cache.get("http://play.example.org:8081", after), None);
}

#[test]
fn pkce_challenge_matches_rfc7636() {
    // The verifier and challenge pair from RFC 7636, appendix B.
    assert_eq!(
        challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    let pair = new_pkce();
    assert_eq!(pair.verifier.len(), 64);
    assert_eq!(pair.challenge, challenge_for(&pair.verifier));
}

#[tokio::test]
async fn browser_flow_round_trip() {
    let base = Url::parse("https://play.example.org/").unwrap();
    let flow = oauth::begin(&base, "dev")
        .await
        .expect("the loopback listener binds");
    let authorize = Url::parse(&flow.authorize_url).unwrap();
    let query: std::collections::HashMap<String, String> =
        authorize.query_pairs().into_owned().collect();
    let state = query["state"].clone();
    let redirect_uri = query["redirect_uri"].clone();
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(state.len(), 43);

    // The browser's redirect, sent to the loopback listener from this test.
    let browser = reqwest::Client::new().get(format!("{redirect_uri}?code=c&state={state}"));
    let (grant, reply) = tokio::join!(flow.finish(), browser.send());
    assert!(reply.is_ok());
    let grant = grant.expect("the redirect carries the code");
    assert_eq!(grant.code, "c");
    assert_eq!(grant.redirect_uri, redirect_uri);
    assert_eq!(grant.verifier.len(), 64);
    assert_eq!(query["code_challenge"], challenge_for(&grant.verifier));
}

#[tokio::test]
async fn browser_flow_ignores_a_mismatched_state() {
    let base = Url::parse("https://play.example.org/").unwrap();
    let flow = oauth::begin(&base, "dev")
        .await
        .expect("the loopback listener binds");
    let authorize = Url::parse(&flow.authorize_url).unwrap();
    let query = |name: &str| {
        authorize
            .query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
            .unwrap()
    };
    let (redirect_uri, state) = (query("redirect_uri"), query("state"));

    // A redirect carrying some other state is refused with 400, and the wait goes on:
    // only the launcher's own redirect, for the expected state, completes the grant.
    let browser = async {
        let wrong = reqwest::Client::new()
            .get(format!("{redirect_uri}?code=c&state=not-the-launchers"))
            .send()
            .await
            .expect("the listener answers");
        assert_eq!(wrong.status(), 400);
        reqwest::Client::new()
            .get(format!("{redirect_uri}?code=c&state={state}"))
            .send()
            .await
    };
    let (grant, reply) = tokio::join!(flow.finish(), browser);
    assert!(reply.is_ok());
    assert_eq!(
        grant.expect("the matching redirect carries the code").code,
        "c"
    );
}

#[tokio::test]
async fn intent_url_must_point_at_the_same_server() {
    let base = Url::parse("https://play.example.org/").unwrap();
    let pending = oauth::prepare().await.expect("the loopback listener binds");
    assert_eq!(
        pending
            .with_intent(
                &base,
                "https://evil.example/auth/v1/authorize?intent=abc".to_owned()
            )
            .err(),
        Some(SignInError::Protocol)
    );
    let own = "https://play.example.org/auth/v1/authorize?intent=abc".to_owned();
    let pending = oauth::prepare().await.expect("the loopback listener binds");
    let flow = pending
        .with_intent(&base, own.clone())
        .expect("the same server's intent is accepted");
    assert_eq!(flow.authorize_url, own);
}

#[tokio::test]
async fn intent_flow_round_trip() {
    let base = Url::parse("https://play.example.org/").unwrap();
    let pending = oauth::prepare().await.expect("the loopback listener binds");
    let redirect_uri = pending.redirect_uri();
    let state = pending.state().to_owned();
    let challenge = pending.code_challenge().to_owned();
    // The URL CT-30's `link_start` builds from the values it was sent.
    let mut url = base.join("auth/v1/authorize").unwrap();
    url.query_pairs_mut()
        .append_pair("provider", "dev")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("intent", "i1");
    let flow = pending
        .with_intent(&base, url.to_string())
        .expect("the same server's intent is accepted");
    assert_eq!(flow.authorize_url, url.to_string());

    let browser = reqwest::Client::new().get(format!("{redirect_uri}?code=c&state={state}"));
    let (grant, reply) = tokio::join!(flow.finish(), browser.send());
    assert!(reply.is_ok());
    let grant = grant.expect("the redirect carries the code");
    assert_eq!(grant.code, "c");
    assert_eq!(grant.redirect_uri, redirect_uri);
    assert_eq!(challenge_for(&grant.verifier), challenge);
    assert_eq!(state.len(), 43);
}
