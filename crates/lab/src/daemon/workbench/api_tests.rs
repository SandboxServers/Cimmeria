use std::sync::{Arc, Mutex};

use reqwest::header::{COOKIE, ORIGIN, SET_COOKIE};
use serde_json::{json, Value};

use super::{router_with_mission_server, SessionStore};
use crate::daemon::http_tests::{spawn_daemon, spawn_router, test_server, TOKEN};
use crate::lease::AcquireRequest;
use crate::server::{Hosted, Instances, LabServer};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};

#[derive(Clone, Default)]
struct MissionQueryFixture(Arc<Mutex<Vec<String>>>);

impl ServerInvoker for MissionQueryFixture {
    fn url(&self) -> &str {
        "fixture"
    }

    fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
        Box::pin(async move {
            assert_eq!(name, "server_db_query");
            let sql = args["sql"].as_str().expect("read-only SQL argument");
            self.0.lock().unwrap().push(sql.to_owned());
            let rows = if sql.contains("WHERE mission_defn = (SELECT mission_defn") {
                json!([{"mission_id": 622, "mission_defn": "Arm Yourself!"}])
            } else if sql.contains("FROM resources.missions WHERE") {
                json!([{"mission_id": 622, "mission_defn": "Arm Yourself!", "mission_label": "General"}])
            } else if sql.contains("FROM resources.mission_steps WHERE") {
                json!([{"mission_id": 622, "step_id": 80623, "step_display_log_text": "Search the Guard"}])
            } else if sql.contains("FROM resources.content_chains WHERE") {
                json!([])
            } else {
                panic!("unapproved mission query: {sql}");
            };
            ToolOutcome {
                ok: true,
                json: json!({"truncated": false, "rows": rows}),
                ..Default::default()
            }
        })
    }
}

#[tokio::test]
async fn bookmark_actions_refuse_before_any_client_call_even_with_matching_world_data() {
    use super::{router_with_store, SessionStore};
    use crate::daemon::http_tests::spawn_router;
    use crate::supervisor::events::fake_bridge::{self, Responder};
    use crate::workbench::bookmarks::{BookmarkDraft, BookmarkStore};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let responder: Responder = Arc::new(move |_, _| {
        counted.fetch_add(1, Ordering::SeqCst);
        Err("client call was not expected".into())
    });
    let supervisor = Arc::new(fake_bridge::supervisor(responder).await);
    let lease = supervisor
        .leases()
        .acquire(AcquireRequest {
            owner: "bookmark-action-test".into(),
            purpose: "offline refusal guard".into(),
            ..Default::default()
        })
        .unwrap();
    let instances = Instances::new(vec![Hosted {
        label: "default".into(),
        supervisor,
    }]);
    let sessions = Arc::new(SessionStore::default());
    let cookie = sessions.redeem(&sessions.open("default")).unwrap().cookie;
    let csrf = sessions.identity(&cookie).unwrap().csrf;
    let cookie = format!("cimmeria_workbench={cookie}");

    let dir = tempfile::tempdir().unwrap();
    let store = BookmarkStore::new(dir.path().to_path_buf());
    let saved = store
        .save(
            "default",
            None,
            BookmarkDraft {
                world_id: 7,
                space_id: None,
                position_m: [1.0, 2.0, 3.0],
                facing_deg: Some(90.0),
                label: "same-world marker".into(),
                source: "operator".into(),
                screenshot_path: None,
                observed_at_ms: 1_000,
            },
        )
        .unwrap();
    assert_eq!(store.list("default").unwrap(), vec![saved.clone()]);

    let router = router_with_store(instances, store.clone())
        .route_layer(axum::middleware::from_fn_with_state(
            sessions.clone(),
            super::super::require_session,
        ))
        .with_state(sessions);
    let base = spawn_router(router).await.replace("/mcp", "");
    let http = reqwest::Client::new();
    let endpoint = format!("{base}/workbench/api/v1/bookmarks/action?instance=default");

    for action in ["click", "teleport"] {
        let body = json!({
            "instance":"default", "id":saved.id, "lease_id":lease.lease_id, "action":action
        });
        let missing_csrf = http
            .post(&endpoint)
            .header(COOKIE, &cookie)
            .header(ORIGIN, &base)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(missing_csrf.status(), reqwest::StatusCode::FORBIDDEN);
        let response = http
            .post(&endpoint)
            .header(COOKIE, &cookie)
            .header(ORIGIN, &base)
            .header("x-cimmeria-csrf", &csrf)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["data"]["error"], "bookmark_action_unavailable");
        assert_eq!(result["gaps"][0]["code"], "session_identity_unbound");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "{action} reached the client bridge"
        );
    }
    store.delete("default", &saved.id).unwrap();
    assert!(store.list("default").unwrap().is_empty());
}

#[tokio::test]
async fn storage_failures_have_distinct_safe_categories() {
    use super::spatial_error;
    use axum::{body::to_bytes, http::StatusCode};

    for (reason, category) in [
        (
            "expected value at line 1 column 1 private-marker",
            "storage_corrupt_json",
        ),
        (
            "Access is denied. (os error 5) private-marker",
            "storage_permission_denied",
        ),
        (
            "The disk is full. (os error 112) private-marker",
            "storage_disk_full",
        ),
    ] {
        let response = spatial_error(
            "default",
            StatusCode::INTERNAL_SERVER_ERROR,
            "bookmark_read_failed",
            reason,
        );
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"]["category"], category);
        assert!(!String::from_utf8(bytes.to_vec())
            .unwrap()
            .contains("private-marker"));
    }
}

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
        "/workbench/views/dossier.js",
        "/workbench/views/probe.js",
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
    for (path, mime) in [
        ("app.js", "text/javascript"),
        ("views/dossier.js", "text/javascript"),
        ("views/probe.js", "text/javascript"),
        ("app.css", "text/css"),
    ] {
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
async fn mission_route_keeps_player_state_unknown_without_trusted_binding() {
    let sessions = Arc::new(SessionStore::default());
    let cookie = sessions.redeem(&sessions.open("default")).unwrap().cookie;
    let fixture = MissionQueryFixture::default();
    let router = router_with_mission_server(
        test_server().instances().clone(),
        Some(Arc::new(fixture.clone())),
    )
    .with_state(sessions);
    let base = spawn_router(router).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let path = format!("{base}/workbench/api/v1/missions?instance=default&mission_id=622");
    assert_eq!(
        http.get(&path).send().await.unwrap().status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let cookie = format!("cimmeria_workbench={cookie}");
    let response = http
        .get(&path)
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let model: Value = response.json().await.unwrap();
    assert_eq!(model["context"]["mission_id"], 622);
    assert!(model["data"]["server"]["value"].is_null());
    assert!(model["data"]["step_disagreement"].is_null());
    assert!(model["gaps"].as_array().unwrap().iter().any(|gap| {
        gap["source"] == "sgw_mission"
            && gap["detail"] == "trusted client-to-server session binding unavailable"
    }));
    let queries = fixture.0.lock().unwrap().clone();
    assert_eq!(queries.len(), 4);
    assert_eq!(queries[0], "SELECT mission_id, mission_label, mission_defn, is_enabled, is_hidden, script_name FROM resources.missions WHERE mission_id = 622");
    assert_eq!(queries[1], "SELECT mission_id, mission_defn FROM resources.missions WHERE mission_defn = (SELECT mission_defn FROM resources.missions WHERE mission_id = 622) ORDER BY mission_id");
    assert_eq!(queries[2], "SELECT step_id, mission_id, step_display_log_text, index, step_enabled FROM resources.mission_steps WHERE mission_id = 622 ORDER BY index, step_id");
    assert_eq!(queries[3], "SELECT chain_id, description, scope_type, scope_id, enabled FROM resources.content_chains WHERE scope_type = 'mission' AND scope_id = 622 ORDER BY chain_id");
    assert!(queries.iter().all(|query| !query.contains("sgw_mission")));
    assert_eq!(
        http.get(format!(
            "{base}/workbench/api/v1/missions?instance=other&mission_id=622"
        ))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .status(),
        reqwest::StatusCode::FORBIDDEN
    );
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

#[tokio::test]
async fn probe_route_requires_origin_csrf_matching_instance_and_lease() {
    let server = test_server();
    let supervisor = server.instances().first().supervisor.clone();
    let base = spawn_daemon(server).await.replace("/mcp", "");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let (cookie, csrf) = browser_session(&http, &base, "default").await;
    let endpoint = format!("{base}/workbench/api/v1/probes/globals?instance=default");
    let body = json!({"instance":"default","lease_id":"missing","name":"a"});
    let send = |body: Value, origin: bool, csrf_header: bool| {
        let mut request = http.post(&endpoint).header(COOKIE, &cookie).json(&body);
        if origin {
            request = request.header(ORIGIN, &base);
        }
        if csrf_header {
            request = request.header("x-cimmeria-csrf", &csrf);
        }
        request
    };
    assert_eq!(
        send(body.clone(), false, true)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(body.clone(), true, false)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    let mismatch = json!({"instance":"other","lease_id":"missing","name":"a"});
    let response = send(mismatch, true, true).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        response.json::<Value>().await.unwrap()["data"]["error"],
        "instance_mismatch"
    );
    let response = send(body, true, true).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        response.json::<Value>().await.unwrap()["data"]["error"],
        "lease_invalid"
    );

    let lease = supervisor
        .leases()
        .acquire(AcquireRequest {
            owner: "workbench-probe-test".into(),
            purpose: "offline API test".into(),
            ..Default::default()
        })
        .unwrap();
    let response = send(
        json!({"instance":"default","lease_id":lease.lease_id,"name":"x".repeat(129)}),
        true,
        true,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json::<Value>().await.unwrap()["data"]["error"],
        "global_invalid"
    );
}
