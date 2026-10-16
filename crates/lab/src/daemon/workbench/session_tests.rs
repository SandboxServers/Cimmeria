use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::header::{HeaderMap, HeaderValue, COOKIE, HOST, ORIGIN, SET_COOKIE};
use serde_json::Value;
use tracing::field::{Field, Visit};
use tracing::Event;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::Layer;

use crate::daemon::http_tests::{initialize_body, spawn_daemon, spawn_router, test_server, TOKEN};
use crate::server::{Hosted, Instances, LabServer};

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
    let nonce = sessions.open_at("default", now);
    assert_eq!(nonce.len(), 64);
    assert!(sessions
        .redeem_at(&nonce, now + Duration::from_secs(59))
        .is_some());
    assert!(sessions
        .redeem_at(&nonce, now + Duration::from_secs(59))
        .is_none());

    let expired = sessions.open_at("default", now);
    assert!(sessions
        .redeem_at(&expired, now + Duration::from_secs(60))
        .is_none());
}

#[test]
fn browser_session_is_bound_to_its_daemon_store() {
    let first = SessionStore::default();
    let second = SessionStore::default();
    let cookie = first.redeem(&first.open("default")).unwrap().cookie;
    assert!(first.identity(&cookie).is_some());
    assert!(second.identity(&cookie).is_none());
}

async fn open_workbench(http: &reqwest::Client, base: &str) -> String {
    open_named_workbench(http, base, "default").await
}

async fn open_named_workbench(http: &reqwest::Client, base: &str, instance: &str) -> String {
    let response = http
        .post(format!("{base}/workbench/session/open"))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({ "instance": instance }))
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
        .get(format!("{base}/workbench/session?instance=default"))
        .header(COOKIE, cookie.as_str())
        .send()
        .await
        .unwrap();
    assert_eq!(csrf_response.status(), reqwest::StatusCode::OK);
    let csrf: Value = csrf_response.json().await.unwrap();
    let csrf = csrf["csrf"].as_str().unwrap();
    let endpoint = format!("{base}/workbench/session?instance=default");
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
        .json(&serde_json::json!({ "instance": "default" }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_host.status(), reqwest::StatusCode::FORBIDDEN);
    let bad_origin = http
        .post(format!("{base}/workbench/session/open"))
        .bearer_auth(TOKEN)
        .header(ORIGIN, base.as_str())
        .json(&serde_json::json!({ "instance": "default" }))
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

#[tokio::test]
async fn one_daemon_binds_each_opener_and_cookie_to_its_hosted_instance() {
    let server = test_server();
    let supervisor = server.instances().first().supervisor.clone();
    let server = LabServer::new_multi(Instances::new(vec![
        Hosted {
            label: "default".into(),
            supervisor: supervisor.clone(),
        },
        Hosted {
            label: "p2".into(),
            supervisor,
        },
    ]));
    let base = spawn_daemon(server).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let unknown = http
        .post(format!("{base}/workbench/session/open"))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({ "instance": "not-hosted" }))
        .send()
        .await
        .unwrap();
    assert_eq!(unknown.status(), reqwest::StatusCode::BAD_REQUEST);

    let path = open_named_workbench(&http, &base, "p2").await;
    let response = redeem_workbench(&http, &base, &path).await;
    assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);
    let cookie = response.headers()[SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let wrong = http
        .get(format!("{base}/workbench/session?instance=default"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), reqwest::StatusCode::FORBIDDEN);
    let missing = http
        .get(format!("{base}/workbench/session"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), reqwest::StatusCode::FORBIDDEN);
    let right = http
        .get(format!("{base}/workbench/session?instance=p2"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(right.status(), reqwest::StatusCode::OK);
    let body: Value = right.json().await.unwrap();
    assert_eq!(body["instance"], "p2");
    let csrf = body["csrf"].as_str().unwrap();
    let wrong_mutation = http
        .post(format!("{base}/workbench/session?instance=default"))
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_mutation.status(), reqwest::StatusCode::FORBIDDEN);
    let right_mutation = http
        .post(format!("{base}/workbench/session?instance=p2"))
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(right_mutation.status(), reqwest::StatusCode::NO_CONTENT);
}

#[derive(Clone, Default)]
struct RefusalCapture(Arc<Mutex<Vec<String>>>);

#[derive(Default)]
struct Fields(Vec<String>);

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push(format!("{}={value}", field.name()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push(format!("{}={value:?}", field.name()));
    }
}

impl<S: tracing::Subscriber> Layer<S> for RefusalCapture {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() == "lab.workbench.auth" {
            let mut fields = Fields::default();
            event.record(&mut fields);
            self.0.lock().unwrap().push(fields.0.join(" "));
        }
    }
}

#[tokio::test]
async fn refusals_log_static_reasons_without_credentials() {
    let _ = tracing::subscriber::set_global_default(tracing_subscriber::registry());
    let capture = RefusalCapture::default();
    let _guard =
        tracing::subscriber::set_default(tracing_subscriber::registry().with(capture.clone()));
    let base = spawn_daemon(test_server()).await.replace("/mcp", "");
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
        .get(format!("{base}/workbench/session?instance=default"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    let csrf: Value = csrf_response.json().await.unwrap();
    let csrf = csrf["csrf"].as_str().unwrap();

    let host = http
        .get(format!("{base}/workbench/"))
        .header(HOST, "evil.example:8779")
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(host.status(), reqwest::StatusCode::FORBIDDEN);
    let origin = http
        .post(format!("{base}/workbench/session?instance=default"))
        .header(COOKIE, &cookie)
        .header(ORIGIN, "http://evil.example")
        .header("x-cimmeria-csrf", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(origin.status(), reqwest::StatusCode::FORBIDDEN);
    let no_cookie = http.get(format!("{base}/workbench/")).send().await.unwrap();
    assert_eq!(no_cookie.status(), reqwest::StatusCode::UNAUTHORIZED);
    let bad_csrf = http
        .post(format!("{base}/workbench/session?instance=default"))
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", "invalid")
        .send()
        .await
        .unwrap();
    assert_eq!(bad_csrf.status(), reqwest::StatusCode::FORBIDDEN);
    let nonce = redeem_workbench(&http, &base, &path).await;
    assert_eq!(nonce.status(), reqwest::StatusCode::UNAUTHORIZED);

    let rows = capture.0.lock().unwrap().clone();
    for reason in [
        "host_invalid",
        "origin_invalid",
        "cookie_invalid",
        "csrf_invalid",
        "nonce_invalid",
    ] {
        assert!(
            rows.iter()
                .any(|row| row.contains(&format!("reason={reason}"))),
            "{reason}: {rows:?}"
        );
    }
    let logged = rows.join(" ");
    for secret in [TOKEN, path.as_str(), cookie.as_str(), csrf] {
        assert!(
            !logged.contains(secret),
            "credential reached refusal log: {rows:?}"
        );
    }
}
