use std::sync::{Arc, Mutex};

use reqwest::header::COOKIE;
use serde_json::{json, Value};

use super::{router_with_mission_server, SessionStore};
use crate::daemon::http_tests::{spawn_router, test_server};
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
            } else if sql.contains("FROM resources.mission_objectives o JOIN") {
                json!([{"step_id": 80623, "objective_id": 91023, "display_log_text": "Search the Guard", "is_enabled": true, "is_hidden": false, "is_optional": false}])
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
    assert_eq!(
        model["data"]["objectives"]["value"][0]["objective_id"],
        91023
    );
    assert!(model["data"]["server"]["value"].is_null());
    assert!(model["data"]["live_server"]["value"].is_null());
    assert!(model["data"]["step_disagreement"].is_null());
    assert!(model["gaps"].as_array().unwrap().iter().any(|gap| {
        gap["source"] == "sgw_mission"
            && gap["detail"] == "trusted client-to-server session binding unavailable"
    }));
    let queries = fixture.0.lock().unwrap().clone();
    assert_eq!(queries.len(), 5);
    assert_eq!(queries[0], "SELECT mission_id, mission_label, mission_defn, is_enabled, is_hidden, script_name FROM resources.missions WHERE mission_id = 622");
    assert_eq!(queries[1], "SELECT mission_id, mission_defn FROM resources.missions WHERE mission_defn = (SELECT mission_defn FROM resources.missions WHERE mission_id = 622) ORDER BY mission_id");
    assert_eq!(queries[2], "SELECT step_id, mission_id, step_display_log_text, index, step_enabled FROM resources.mission_steps WHERE mission_id = 622 ORDER BY index, step_id");
    assert_eq!(queries[3], "SELECT o.objective_id, o.step_id, o.display_log_text, o.is_enabled, o.is_hidden, o.is_optional FROM resources.mission_objectives o JOIN resources.mission_steps s ON s.step_id = o.step_id WHERE s.mission_id = 622 ORDER BY s.index, o.objective_id");
    assert_eq!(queries[4], "SELECT chain_id, description, scope_type, scope_id, enabled FROM resources.content_chains WHERE scope_type = 'mission' AND scope_id = 622 ORDER BY chain_id");
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
