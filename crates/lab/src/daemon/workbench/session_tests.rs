use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::{HeaderMap, HeaderValue, COOKIE, HOST, ORIGIN, SET_COOKIE};
use serde_json::Value;

use crate::daemon::http_tests::{initialize_body, spawn_daemon, spawn_router, test_server, TOKEN};

use super::SessionStore;

#[test]
fn host_must_be_a_canonical_loopback_authority() {
    let mut headers = HeaderMap::new();
    for accepted in ["127.0.0.1:8779", "localhost:8779", "[::1]:8779"] {
        headers.insert(HOST, HeaderValue::from_str(accepted).unwrap());
        assert_eq!(super::super::local_host(&headers), Some(accepted));
    }
    for rejected in [
        "evil.example:8779",
        "127.0.0.1:0",
        "127.0.0.1:08779",
        "user@127.0.0.1:8779",
    ] {
        headers.insert(HOST, HeaderValue::from_str(rejected).unwrap());
        assert!(super::super::local_host(&headers).is_none(), "{rejected}");
    }
}

#[test]
fn opener_is_one_use_and_expires_at_sixty_seconds() {
    let sessions = SessionStore::default();
    let now = Instant::now();
    let nonce = sessions.open_at(now);
    assert_eq!(nonce.len(), 64);
    assert!(sessions
        .redeem_at(&nonce, now + Duration::from_secs(59))
        .is_some());
    assert!(sessions
        .redeem_at(&nonce, now + Duration::from_secs(59))
        .is_none());

    let expired = sessions.open_at(now);
    assert!(sessions
        .redeem_at(&expired, now + Duration::from_secs(60))
        .is_none());
}

#[test]
fn browser_session_is_bound_to_its_daemon_store() {
    let first = SessionStore::default();
    let second = SessionStore::default();
    let cookie = first.redeem(&first.open()).unwrap().cookie;
    assert!(first.csrf(&cookie).is_some());
    assert!(second.csrf(&cookie).is_none());
}

async fn open_workbench(http: &reqwest::Client, base: &str) -> String {
    let response = http
        .post(format!("{base}/workbench/session/open"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(response.headers().get(SET_COOKIE).is_none());
    let body: Value = response.json().await.unwrap();
    body["path"].as_str().unwrap().to_owned()
}

async fn redeem_workbench(http: &reqwest::Client, base: &str, path: &str) -> reqwest::Response {
    http.get(format!("{base}{path}")).send().await.unwrap()
}

/// A bearer caller obtains a one-use path. The browser receives only a scoped,
/// HttpOnly cookie; the opener never turns `/mcp` or `/status` into cookie APIs.
#[tokio::test]
async fn workbench_opener_redeems_once_and_cannot_authenticate_mcp() {
    let base = spawn_daemon(test_server()).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let path = open_workbench(&http, &base).await;
    let response = redeem_workbench(&http, &base, &path).await;
    assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);
    assert_eq!(response.headers()["location"], "/workbench/");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert_eq!(
        response.headers()["content-security-policy"],
        "default-src 'self'"
    );
    let set_cookie = response.headers()[SET_COOKIE].to_str().unwrap().to_owned();
    assert!(set_cookie.contains("HttpOnly; SameSite=Strict; Path=/workbench/"));
    let cookie = set_cookie.split(';').next().unwrap();
    assert!(!response.text().await.unwrap().contains(TOKEN));
    assert_eq!(
        redeem_workbench(&http, &base, &path).await.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );

    let root = http
        .get(format!("{base}/workbench/"))
        .header(COOKIE, cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(root.status(), reqwest::StatusCode::NO_CONTENT);
    let mcp = http
        .post(format!("{base}/mcp"))
        .header(COOKIE, cookie)
        .json(&initialize_body())
        .send()
        .await
        .unwrap();
    assert_eq!(mcp.status(), reqwest::StatusCode::UNAUTHORIZED);
    let status = http
        .get(format!("{base}/status"))
        .header(COOKIE, cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(status.status(), reqwest::StatusCode::UNAUTHORIZED);
}

/// Mutations need both exact same-origin provenance and the CSRF value from
/// the cookie-authenticated session endpoint.
#[tokio::test]
async fn workbench_rejects_missing_csrf_foreign_origin_and_cross_instance_cookie() {
    let base = spawn_daemon(test_server()).await.replace("/mcp", "");
    let other = spawn_daemon(test_server()).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let path = open_workbench(&http, &base).await;
    let response = redeem_workbench(&http, &base, &path).await;
    let cookie = response.headers()[SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let csrf_response = http
        .get(format!("{base}/workbench/session"))
        .header(COOKIE, cookie.as_str())
        .send()
        .await
        .unwrap();
    assert_eq!(csrf_response.status(), reqwest::StatusCode::OK);
    let csrf: Value = csrf_response.json().await.unwrap();
    let csrf = csrf["csrf"].as_str().unwrap();
    let endpoint = format!("{base}/workbench/session");
    let missing = http
        .post(&endpoint)
        .header(COOKIE, cookie.as_str())
        .header(ORIGIN, base.as_str())
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), reqwest::StatusCode::FORBIDDEN);
    let no_origin = http
        .post(&endpoint)
        .header(COOKIE, cookie.as_str())
        .header("x-cimmeria-csrf", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(no_origin.status(), reqwest::StatusCode::FORBIDDEN);
    let foreign = http
        .post(&endpoint)
        .header(COOKIE, cookie.as_str())
        .header(ORIGIN, "http://evil.example")
        .header("x-cimmeria-csrf", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status(), reqwest::StatusCode::FORBIDDEN);
    let valid = http
        .post(&endpoint)
        .header(COOKIE, cookie.as_str())
        .header(ORIGIN, base.as_str())
        .header("x-cimmeria-csrf", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(valid.status(), reqwest::StatusCode::NO_CONTENT);
    let cross = http
        .get(format!("{other}/workbench/"))
        .header(COOKIE, cookie.as_str())
        .send()
        .await
        .unwrap();
    assert_eq!(cross.status(), reqwest::StatusCode::UNAUTHORIZED);
}

/// The GUI boundary rejects DNS rebinding even when the caller has a token
/// or a previously valid browser cookie; expired opener paths also fail.
#[tokio::test]
async fn workbench_rejects_foreign_host_and_expired_opener() {
    let store = Arc::new(SessionStore::default());
    let router = crate::daemon::build_router_with_sessions(test_server(), TOKEN, store.clone());
    let base = spawn_router(router).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let no_bearer = http
        .post(format!("{base}/workbench/session/open"))
        .send()
        .await
        .unwrap();
    assert_eq!(no_bearer.status(), reqwest::StatusCode::UNAUTHORIZED);
    let bad_host = http
        .post(format!("{base}/workbench/session/open"))
        .bearer_auth(TOKEN)
        .header(HOST, "evil.example:8779")
        .send()
        .await
        .unwrap();
    assert_eq!(bad_host.status(), reqwest::StatusCode::FORBIDDEN);
    let bad_origin = http
        .post(format!("{base}/workbench/session/open"))
        .bearer_auth(TOKEN)
        .header(ORIGIN, base.as_str())
        .send()
        .await
        .unwrap();
    assert_eq!(bad_origin.status(), reqwest::StatusCode::FORBIDDEN);
    let path = open_workbench(&http, &base).await;
    store.expire_openers();
    assert_eq!(
        redeem_workbench(&http, &base, &path).await.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let path = open_workbench(&http, &base).await;
    let bad_redeem = http
        .get(format!("{base}{path}"))
        .header(HOST, "evil.example:8779")
        .send()
        .await
        .unwrap();
    assert_eq!(bad_redeem.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        redeem_workbench(&http, &base, &path).await.status(),
        reqwest::StatusCode::SEE_OTHER
    );
}
