//! Tests of the sign-in routes (CT-24): the ten rows of the packet table, run
//! with `oneshot` against [`router`] and a Discord provider that wiremock stubs.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::body::{to_bytes, Body};
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use cimmeria_auth::auth::{AuthRouteDeps, LaunchTicketStore};
use serde_json::{json, Value};
use tower::ServiceExt;
use url::form_urlencoded::Serializer;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::config;
use crate::limits::RateLimiter;
use crate::link_code::LinkCodeStore;
use crate::provider::IdentityProvider;
use crate::providers::dev_mock::DevMockProvider;
use crate::providers::discord::DiscordProvider;
use crate::providers::Providers;
use crate::routes::ctx::IdentityCtx;
use crate::routes::router;
use crate::stores::{pkce_challenge, Stores};

/// The launcher's own `state` and PKCE verifier for every sign-in in these tests.
const LAUNCHER_STATE: &str = "launcher-state-0123456789";
const VERIFIER: &str = "launcher-verifier-0123456789abcdefghijklmnopqrstuvw";
const REDIRECT: &str = "http://127.0.0.1:5000/cb";

const DISCORD_TABLE: &str = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "discord"
kind = "discord"
display_name = "Discord"
client_id = "000000000000000000"
client_secret_env = "CIMMERIA_IDP_TEST_SECRET"
trusted_email = true
"#;

const DEV_TABLE: &str = r#"
[[provider]]
id = "dev"
kind = "dev_mock"
display_name = "Developer sign-in"
client_id = "dev-client"
client_secret_env = "CIMMERIA_IDP_TEST_SECRET"
trusted_email = true
"#;

/// A router under test, with the stores it shares so a test can redeem codes.
struct Harness {
    router: Router,
    stores: Arc<Stores>,
}

/// Builds the routes with a Discord provider aimed at `base` and, in developer
/// mode only, the dev-mock provider.
fn harness(base: &str, developer_mode: bool, idp_disabled: bool) -> Harness {
    let env =
        |name: &str| (name == "CIMMERIA_IDP_TEST_SECRET").then(|| "s3cr3t-test-value".to_owned());
    let text = if developer_mode {
        format!("{DISCORD_TABLE}{DEV_TABLE}")
    } else {
        DISCORD_TABLE.to_owned()
    };
    let (mut cfg, _) = config::load(&text, &env, developer_mode).expect("the fixture loads");
    cfg.kill.idp_disabled = idp_disabled;

    let http = crate::http::provider_client();
    let base = Url::parse(base).expect("the stub base is a URL");
    let discord = DiscordProvider::with_base(&cfg.providers[0], http, base);
    let mut list: Vec<Arc<dyn IdentityProvider>> = vec![Arc::new(discord)];
    let mut dev_mock = None;
    if developer_mode {
        let dev = Arc::new(DevMockProvider::new(
            &cfg.providers[1],
            &cfg.public_base_url,
        ));
        list.push(dev.clone());
        dev_mock = Some(dev);
    }
    let stores = Arc::new(Stores::default());
    let ctx = IdentityCtx {
        limiter: Arc::new(RateLimiter::with_limits(&cfg.limits)),
        cfg: Arc::new(cfg),
        providers: Arc::new(Providers::from_parts(list, dev_mock)),
        stores: Arc::clone(&stores),
        link_codes: Box::leak(Box::new(LinkCodeStore::default())),
    };
    let deps = AuthRouteDeps {
        db: None,
        launch_tickets: Arc::new(LaunchTicketStore::default()),
        developer_mode,
    };
    Harness {
        router: router(deps, ctx),
        stores,
    }
}

/// Sends a `GET` for `uri` from the loopback peer.
async fn get(router: &Router, uri: &str) -> Response {
    let mut req = Request::get(uri)
        .body(Body::empty())
        .expect("the test URI is valid");
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
    router
        .clone()
        .oneshot(req)
        .await
        .expect("the router is infallible")
}

async fn body_text(resp: Response) -> String {
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the body reads");
    String::from_utf8(bytes.to_vec()).expect("the body is UTF-8")
}

fn location(resp: &Response) -> String {
    resp.headers()
        .get(header::LOCATION)
        .expect("a Location header")
        .to_str()
        .expect("a text Location")
        .to_owned()
}

/// The `/authorize` URI with the valid launcher request, with `overrides`
/// replacing single parameters.
fn authorize_uri(overrides: &[(&str, &str)]) -> String {
    let challenge = pkce_challenge(VERIFIER);
    let mut pairs: Vec<(&str, &str)> = vec![
        ("provider", "discord"),
        ("redirect_uri", REDIRECT),
        ("state", LAUNCHER_STATE),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
    ];
    for (key, value) in overrides {
        pairs.retain(|(k, _)| k != key);
        pairs.push((*key, *value));
    }
    let query = Serializer::new(String::new()).extend_pairs(pairs).finish();
    format!("/authorize?{query}")
}

/// Runs a valid `/authorize` and returns the server `state` the provider saw.
async fn server_state(router: &Router) -> String {
    let resp = get(router, &authorize_uri(&[])).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
    let url = Url::parse(&location(&resp)).expect("the redirect is a URL");
    url.query_pairs()
        .find(|(key, _)| key == "state")
        .expect("the redirect carries a state")
        .1
        .into_owned()
}

/// The query pairs of an absolute redirect URL.
fn query_pairs(raw: &str) -> Vec<(String, String)> {
    Url::parse(raw)
        .expect("the redirect is a URL")
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn pair<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Mounts Discord's token and user answers for a sign-in as `ada`.
async fn mount_discord(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/api/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "tok"})))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/users/@me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "80351110224678912",
            "username": "ada",
            "verified": true,
            "email": "ada@example.org"
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn providers_lists_configured_providers_and_links() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), true, false);
    let resp = get(&h.router, "/providers").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_str(&body_text(resp).await).expect("JSON");
    assert_eq!(
        body,
        json!({
            "api": 1,
            "soap_relay": true,
            "signup_open": true,
            "providers": [
                {"id": "discord", "display_name": "Discord"},
                {"id": "dev", "display_name": "Developer sign-in"}
            ]
        })
    );
}

#[tokio::test]
async fn authorize_refuses_open_redirects() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, false);
    let refused = [
        "http://example.org/cb",
        "http://127.0.0.1:80/cb",
        "http://127.0.0.1:5000/cb?x=1",
        "http://user@127.0.0.1:5000/cb",
        "https://127.0.0.1:5000/cb",
    ];
    for redirect in refused {
        let resp = get(&h.router, &authorize_uri(&[("redirect_uri", redirect)])).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{redirect}");
        assert!(resp.headers().get(header::LOCATION).is_none(), "{redirect}");
    }
}

#[tokio::test]
async fn authorize_refuses_bad_state_and_challenge() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, false);
    let short_state = "abcdefghijklmno";
    let short_challenge = "a".repeat(42);
    let cases = [
        vec![("state", short_state)],
        vec![("code_challenge", short_challenge.as_str())],
        vec![("code_challenge_method", "plain")],
    ];
    for overrides in cases {
        let resp = get(&h.router, &authorize_uri(&overrides)).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{overrides:?}");
    }
}

#[tokio::test]
async fn authorize_redirects_to_the_provider() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, false);
    let resp = get(&h.router, &authorize_uri(&[])).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
    let target = location(&resp);
    assert!(target.starts_with(&server.uri()), "{target}");
    let pairs = query_pairs(&target);
    let server_state = pair(&pairs, "state").expect("a server state");
    assert_ne!(
        server_state, LAUNCHER_STATE,
        "the two PKCE legs stay separate"
    );
    assert!(pair(&pairs, "code_challenge").is_some());
}

#[tokio::test]
async fn callback_round_trip_gives_a_handoff_code() {
    let server = MockServer::start().await;
    mount_discord(&server).await;
    let h = harness(&server.uri(), false, false);
    let state = server_state(&h.router).await;
    let resp = get(
        &h.router,
        &format!("/callback/discord?code=code&state={state}"),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FOUND);
    let target = location(&resp);
    assert!(target.starts_with(REDIRECT), "{target}");
    let pairs = query_pairs(&target);
    assert_eq!(pair(&pairs, "state"), Some(LAUNCHER_STATE));
    let code = pair(&pairs, "code").expect("a handoff code");
    let outcome = h
        .stores
        .handoff
        .redeem(code, VERIFIER, REDIRECT, Instant::now());
    assert!(
        outcome.is_some(),
        "the launcher's verifier redeems the code"
    );
}

#[tokio::test]
async fn callback_with_unknown_state_is_the_expired_page() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, false);
    let resp = get(
        &h.router,
        "/callback/discord?code=code&state=presented-state-xyz",
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(content_type.starts_with("text/html"), "{content_type}");
    let body = body_text(resp).await;
    assert!(!body.contains("presented-state-xyz"), "no reflection");
}

#[tokio::test]
async fn provider_denial_redirects_access_denied() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, false);
    let state = server_state(&h.router).await;
    let resp = get(
        &h.router,
        &format!("/callback/discord?error=access_denied&state={state}"),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FOUND);
    let pairs = query_pairs(&location(&resp));
    assert_eq!(pair(&pairs, "error"), Some("access_denied"));
    assert_eq!(pair(&pairs, "state"), Some(LAUNCHER_STATE));
}

#[tokio::test]
async fn authorize_is_rate_limited() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, false);
    for _ in 0..20 {
        let resp = get(&h.router, &authorize_uri(&[])).await;
        assert_eq!(resp.status(), StatusCode::FOUND);
    }
    let resp = get(&h.router, &authorize_uri(&[])).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(resp.headers().contains_key(header::RETRY_AFTER));
}

#[tokio::test]
async fn idp_disabled_answers_503_except_providers() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, true);
    let resp = get(&h.router, &authorize_uri(&[])).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(body_text(resp).await.contains("disabled"));
    let resp = get(&h.router, "/providers").await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn dev_mock_absent_without_developer_mode() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), false, false);
    let resp = get(
        &h.router,
        &format!("/dev-mock/authorize?state={LAUNCHER_STATE}"),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
