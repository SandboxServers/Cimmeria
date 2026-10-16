use reqwest::header::{COOKIE, ORIGIN, SET_COOKIE};
use serde_json::{json, Value};

use crate::daemon::http_tests::{spawn_daemon, test_server, TOKEN};
use crate::lease::AcquireRequest;
use crate::server::{Hosted, Instances, LabServer};

async fn browser_session(http: &reqwest::Client, base: &str, instance: &str) -> (String, String) {
    let response = http
        .post(format!("{base}/workbench/session/open"))
        .bearer_auth(TOKEN)
        .json(&json!({ "instance": instance }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let opener: Value = response.json().await.unwrap();
    let redeem = http
        .get(format!("{base}{}", opener["path"].as_str().unwrap()))
        .send()
        .await
        .unwrap();
    assert_eq!(redeem.status(), reqwest::StatusCode::SEE_OTHER);
    let cookie = redeem.headers()[SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let response = http
        .get(format!("{base}/workbench/session?instance={instance}"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    let session: Value = response.json().await.unwrap();
    (cookie, session["csrf"].as_str().unwrap().to_owned())
}

#[tokio::test]
async fn shell_assets_and_api_are_inside_the_session_gate() {
    let base = spawn_daemon(test_server()).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    for path in [
        "/workbench/",
        "/workbench/app.js",
        "/workbench/app.css",
        "/workbench/api/status?instance=default",
    ] {
        let response = http.get(format!("{base}{path}")).send().await.unwrap();
        assert_eq!(
            response.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
    let (cookie, _) = browser_session(&http, &base, "default").await;
    let index = http
        .get(format!("{base}/workbench/"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(index.status(), reqwest::StatusCode::OK);
    let html = index.text().await.unwrap();
    assert!(html.contains("data-instance=\"default\""));
    assert!(!html.contains(TOKEN));
    for (path, mime) in [("app.js", "text/javascript"), ("app.css", "text/css")] {
        let response = http
            .get(format!("{base}/workbench/{path}"))
            .header(COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with(mime));
    }
    let response = http
        .get(format!("{base}/workbench/api/status?instance=default"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["instance"], "default");
    assert_eq!(body["lease"]["held"], false);
}

#[tokio::test]
async fn input_mutation_requires_matching_query_body_session_csrf_and_lease() {
    let server = test_server();
    let supervisor = server.instances().first().supervisor.clone();
    let server = LabServer::new_multi(Instances::new(vec![
        Hosted {
            label: "default".into(),
            supervisor: supervisor.clone(),
        },
        Hosted {
            label: "p2".into(),
            supervisor: supervisor.clone(),
        },
    ]));
    let base = spawn_daemon(server).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let (cookie, csrf) = browser_session(&http, &base, "p2").await;
    let endpoint = format!("{base}/workbench/api/input/key?instance=p2");
    let body = json!({"instance":"p2","lease_id":"missing","key":"F1"});
    let response = http
        .post(&endpoint)
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let response = http
        .post(&endpoint)
        .header(COOKIE, &cookie)
        .header("x-cimmeria-csrf", &csrf)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let response = http
        .post(format!("{base}/workbench/api/input/key?instance=default"))
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", &csrf)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let response = http
        .post(&endpoint)
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", &csrf)
        .json(&json!({"instance":"default","lease_id":"missing","key":"F1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let response = http
        .post(&endpoint)
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", &csrf)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let lease = supervisor
        .leases()
        .acquire(AcquireRequest {
            owner: "workbench-test".into(),
            purpose: "offline API test".into(),
            ..Default::default()
        })
        .unwrap();
    let response = http
        .post(&endpoint)
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", &csrf)
        .json(&json!({"instance":"p2","lease_id":lease.lease_id,"key":"F1"}))
        .send()
        .await
        .unwrap();
    // The lease gate passed; the fixture has no client window to receive input.
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let result: Value = response.json().await.unwrap();
    assert_eq!(result["error"], "input_failed");
}
