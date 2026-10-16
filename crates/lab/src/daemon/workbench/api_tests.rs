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
        .get(format!(
            "{base}/workbench/api/v1/session?instance={instance}"
        ))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    let session: Value = response.json().await.unwrap();
    assert_eq!(session["schema_version"], 1);
    assert_eq!(session["context"]["instance"], instance);
    assert_eq!(session["data"]["instance"], instance);
    assert!(session["gaps"].is_array());
    (cookie, session["data"]["csrf"].as_str().unwrap().to_owned())
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
        "/workbench/api/v1/session?instance=default",
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
        .get(format!("{base}/workbench/api/v1/session?instance=default"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["context"]["instance"], "default");
    assert_eq!(body["data"]["instance"], "default");
    assert_eq!(body["data"]["lease"]["held"], false);
    assert!(body["data"]["csrf"].as_str().is_some());
    assert!(body["gaps"].is_array());
}

#[tokio::test]
async fn dossier_read_needs_session_but_no_lease_and_reports_missing_client() {
    let base = spawn_daemon(test_server()).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let path = format!("{base}/workbench/api/v1/dossier?instance=default&entity_id=42");
    assert_eq!(
        http.get(&path).send().await.unwrap().status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let (cookie, _) = browser_session(&http, &base, "default").await;
    let response = http
        .get(&path)
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let dossier: Value = response.json().await.unwrap();
    assert_eq!(dossier["schema_version"], 1);
    assert_eq!(dossier["context"]["current"], false);
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|gap| gap["source"] == "client.target_slot"));
    let other = http
        .get(format!(
            "{base}/workbench/api/v1/dossier?instance=other&entity_id=42"
        ))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(other.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn input_mutation_requires_matching_query_body_session_csrf_and_lease() {
    let default_supervisor = test_server().instances().first().supervisor.clone();
    let p2_supervisor = test_server().instances().first().supervisor.clone();
    let server = LabServer::new_multi(Instances::new(vec![
        Hosted {
            label: "default".into(),
            supervisor: default_supervisor.clone(),
        },
        Hosted {
            label: "p2".into(),
            supervisor: p2_supervisor.clone(),
        },
    ]));
    let base = spawn_daemon(server).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let (cookie, csrf) = browser_session(&http, &base, "p2").await;
    let endpoint = format!("{base}/workbench/api/v1/input/key?instance=p2");
    let p2_lease = p2_supervisor
        .leases()
        .acquire(AcquireRequest {
            owner: "workbench-test-p2".into(),
            purpose: "offline API test".into(),
            ..Default::default()
        })
        .unwrap();
    let default_lease = default_supervisor
        .leases()
        .acquire(AcquireRequest {
            owner: "workbench-test-default".into(),
            purpose: "offline API test".into(),
            ..Default::default()
        })
        .unwrap();
    let body = json!({"instance":"p2","lease_id":p2_lease.lease_id,"key":"F1"});
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
        .post(format!(
            "{base}/workbench/api/v1/input/key?instance=default"
        ))
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
        .json(&json!({"instance":"default","lease_id":default_lease.lease_id,"key":"F1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let refusal: Value = response.json().await.unwrap();
    assert_eq!(refusal["data"]["error"], "instance_mismatch");
    assert_eq!(refusal["context"]["instance"], "p2");
    let response = http
        .post(&endpoint)
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", &csrf)
        .json(&json!({"instance":"p2","lease_id":default_lease.lease_id,"key":"F1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let refusal: Value = response.json().await.unwrap();
    assert_eq!(refusal["data"]["error"], "lease_invalid");
    assert_eq!(refusal["context"]["instance"], "p2");
    let response = http
        .post(&endpoint)
        .header(COOKIE, &cookie)
        .header(ORIGIN, &base)
        .header("x-cimmeria-csrf", &csrf)
        .json(&body)
        .send()
        .await
        .unwrap();
    // The lease gate passed; the fixture has no client window to receive input.
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let result: Value = response.json().await.unwrap();
    assert_eq!(result["data"]["error"], "input_failed");
    assert_eq!(result["context"]["instance"], "p2");
}
