//! AP-03 typed captures, automatic ids, and unavailable-server grading.

use serde_json::json;

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::{RowCtx, Runner};
use crate::uat::attest::{attest, AttestRequest};
use crate::uat::evidence::{RowResult, RunDir};
use crate::uat::invoke::ServerInvoker;
use crate::uat::spec;
use crate::uat::tier::Role;

const ROW: &str = r#"
[[row]]
id = "capture"
title = "typed captures"
expected = "The inventory count drops by one."
anchor = false
step = [
  { capture = "tool", tool = "client_inventory", args = { container = 1 }, pointer = "/items/0/count", var = "before" },
  { capture = "server", tool = "server_db_query", args = { sql = "SELECT ${player_id}" }, pointer = "/rows/0/n", var = "server_before" },
  { tool = "client_target", args = { entity_id = "${entity_id}" } },
]
[[row.expect]]
id = "one-used"
text = "one item was used"
source = "tool"
tool = "client_inventory"
pointer = "/items/0/count"
op = "delta"
baseline = "${before}"
value = -1
"#;

async fn run(
    fake: &Fake,
    server: Option<&FakeServer>,
    rows: &str,
) -> crate::uat::evidence::RowEvidence {
    let tmp = tempfile::tempdir().unwrap().keep();
    let server = server.map(|s| s as &dyn ServerInvoker);
    let out = Runner::new(fake, server, request(&tmp, rows))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0)
}

#[tokio::test]
async fn tool_and_server_captures_preserve_json_values_and_feed_later_reads() {
    let fake = Fake::new(&[])
        .with("client_player_state")
        .with("client_inventory")
        .with("client_target")
        .answer(
            "client_player_state",
            vec![json!({"name":"Labone", "entity_id":7})],
        )
        .answer(
            "client_inventory",
            vec![
                json!({"items":[{"count":3}]}),
                json!({"items":[{"count":2}]}),
            ],
        );
    let server = FakeServer::new(json!({}))
        .answer(
            "server_sessions",
            vec![json!({"sessions":[{"entity_id":7,"player_id":700,"name":"Labone"}]})],
        )
        .answer("server_db_query", vec![json!({"rows":[{"n":99_u64}]})]);
    let row = run(&fake, Some(&server), ROW).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert_eq!(row.vars["before"], 3);
    assert_eq!(row.vars["server_before"], 99);
    assert_eq!(row.vars["player_id"], 700);
    assert_eq!(row.vars["entity_id"], 7);
    assert_eq!(row.clauses[0].observed, 2);
    let calls = server.calls.lock().unwrap();
    assert_eq!(
        calls.iter().find(|c| c.0 == "server_db_query").unwrap().1["sql"],
        "SELECT 700"
    );
}

#[tokio::test]
async fn unavailable_server_id_is_unverified_before_actions() {
    let fake = Fake::new(&[])
        .with("client_player_state")
        .with("client_inventory")
        .with("client_target");
    let row = run(&fake, None, ROW).await;
    assert_eq!(row.result, RowResult::Unverified, "{:?}", row.reasons);
    assert!(row
        .actions
        .iter()
        .all(|a| a.requested != "client_inventory"));
    assert!(row
        .reasons
        .iter()
        .any(|r| r.contains("player_id unverified")));
}

#[tokio::test]
async fn refused_server_sessions_is_unverified_not_a_failed_step() {
    let fake = Fake::new(&[])
        .with("client_player_state")
        .with("client_inventory")
        .with("client_target");
    let mut server = FakeServer::new(json!({}));
    server.unreachable = true;
    let row = run(&fake, Some(&server), ROW).await;
    assert_eq!(row.result, RowResult::Unverified, "{:?}", row.reasons);
    assert!(row.reasons.iter().any(|r| r.contains("server_sessions")));
}

#[tokio::test]
async fn entity_id_falls_back_to_the_client_entity_table() {
    let fake = Fake::new(&[])
        .with("client_player_state")
        .with("client_entity_find")
        .with("client_target")
        .answer("client_entity_find", vec![json!({"player":{"id":7}})]);
    let rows = r#"
[[row]]
id = "entity-id"
title = "entity id"
expected = "The own entity id is usable."
anchor = false
step = [{ tool = "client_target", args = { entity_id = "${entity_id}" } }]
[[row.expect]]
id = "read"
text = "the player id was resolved"
source = "tool"
tool = "client_entity_find"
pointer = "/player/id"
op = "eq"
value = "${entity_id}"
"#;
    let row = run(&fake, None, rows).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert_eq!(row.vars["entity_id"], 7);
    assert!(fake.names().contains(&"client_entity_find".to_string()));
    assert_eq!(
        row.actions
            .iter()
            .find(|a| a.requested == "client_target")
            .unwrap()
            .args["entity_id"],
        7
    );
}

#[tokio::test]
async fn unavailable_server_capture_is_unverified_and_stops_actions() {
    let fake = Fake::new(&[]);
    let rows = r#"
[[row]]
id = "server-capture"
title = "capture"
expected = "read server"
anchor = false
step = [
  { capture = "server", tool = "server_sessions", args = {}, pointer = "/count", var = "count" },
  { chat = ".help" },
]
[[row.expect]]
id = "read"
text = "server value is present"
source = "human"
question = "Was the server value present?"
"#;
    let row = run(&fake, None, rows).await;
    assert_eq!(row.result, RowResult::Unverified, "{:?}", row.reasons);
    assert!(!fake.names().contains(&"client_type_text".to_string()));
}

#[tokio::test]
async fn attestation_cannot_erase_an_unavailable_server_capture() {
    let tmp = tempfile::tempdir().unwrap().keep();
    let fake = Fake::new(&[]);
    let rows = r#"
[[row]]
id = "capture-attest"
title = "capture then attest"
expected = "The server read succeeds."
state = "any"
anchor = false
step = [
  { wait_ms = 1, label = "before" },
  { capture = "server", tool = "server_sessions", args = {}, pointer = "/count", var = "count" },
]
[[row.expect]]
id = "log"
text = "An unrelated server event occurred."
source = "signoz"
filter = "event = 'capture-test'"
at = "before"
"#;
    let out = Runner::new(&fake, None, request(&tmp, rows))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let run = RunDir::open(std::path::Path::new(&out.run_dir)).unwrap();
    let row = run.rows().unwrap().remove(0);
    assert_eq!(row.result, RowResult::Unverified);
    assert_eq!(row.clauses.len(), 1);
    assert_eq!(row.unverified.len(), 1);
    assert!(row.unverified[0].contains("server lab MCP not configured"));

    let attested = attest(&AttestRequest {
        run_dir: run.root.clone(),
        row: Some("capture-attest".into()),
        clause: Some("log".into()),
        row_count: Some(1),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(attested["result"], "UNVERIFIED", "{attested}");
    let persisted = run.rows().unwrap().remove(0);
    assert_eq!(persisted.result, RowResult::Unverified);
    assert_eq!(persisted.unverified, row.unverified);
    assert!(persisted
        .reasons
        .iter()
        .any(|reason| reason.contains("server lab MCP not configured")));
}

#[test]
fn capture_specs_reject_mutating_client_and_server_tools() {
    let tmp = tempfile::tempdir().unwrap();
    let mut valid = request(tmp.path(), ROW).sections.remove(0).spec;
    for (source, tool) in [
        ("tool", "client_lua_eval"),
        ("server", "server_console_exec"),
        ("server", "server_chaos_clear"),
        ("server", "server_packet_tap_stop"),
        ("server", "server_packet_tap_start"),
        ("server", "server_packet_tap_read"),
    ] {
        valid.rows[0].steps[0].capture = Some(source.into());
        valid.rows[0].steps[0].tool = Some(tool.into());
        let error = spec::validate(&valid).unwrap_err();
        assert!(
            error.contains(&format!("capture tool {tool} is not read-only")),
            "{error}"
        );
    }
}

#[tokio::test]
async fn mutated_capture_is_blocked_before_driving() {
    let tmp = tempfile::tempdir().unwrap().keep();
    let fake = Fake::new(&[]);
    let server = FakeServer::new(json!({}));
    let mut req = request(&tmp, ROW);
    for (source, tool) in [
        ("tool", "client_lua_eval"),
        ("server", "server_console_exec"),
        ("server", "server_chaos_clear"),
        ("server", "server_packet_tap_stop"),
        ("server", "server_packet_tap_read"),
    ] {
        let action = &mut req.sections[0].spec.rows[0].steps[0];
        action.capture = Some(source.into());
        action.tool = Some(tool.into());
        let out = Runner::new(&fake, Some(&server), req.clone())
            .unwrap()
            .run_all()
            .await
            .unwrap();
        let row = RunDir::open(std::path::Path::new(&out.run_dir))
            .unwrap()
            .rows()
            .unwrap()
            .remove(0);
        assert_eq!(row.result, RowResult::Blocked, "{tool}: {:?}", row.reasons);
        assert!(row.reasons.iter().any(|r| r.contains("not read-only")));
        assert!(!fake.names().contains(&tool.to_string()));
        assert!(!server.calls.lock().unwrap().iter().any(|c| c.0 == tool));
    }
}

#[tokio::test]
async fn capture_execution_rechecks_read_only_before_invoking() {
    let tmp = tempfile::tempdir().unwrap().keep();
    let fake = Fake::new(&[]);
    let server = FakeServer::new(json!({}));
    let mut runner = Runner::new(&fake, Some(&server), request(&tmp, ROW)).unwrap();
    for (source, tool) in [
        ("tool", "client_lua_eval"),
        ("server", "server_console_exec"),
        ("server", "server_chaos_clear"),
        ("server", "server_packet_tap_stop"),
        ("server", "server_packet_tap_read"),
    ] {
        let mut action = runner.req.sections[0].spec.rows[0].steps[0].clone();
        action.capture = Some(source.into());
        action.tool = Some(tool.into());
        let mut ctx = RowCtx {
            section: "test".into(),
            row_id: "capture".into(),
            vars: Default::default(),
            actions: vec![],
            blocked: vec![],
            unverified: vec![],
            chat_marks: Default::default(),
            attachments: vec![],
            character: json!({}),
            anchor: None,
            started_ms: 0,
            tap: None,
            event_marks: Default::default(),
            fp: None,
            fixture_mode: None,
            session_end: vec![],
            parity: None,
            poses: vec![],
            log_mark: None,
            chaos: Default::default(),
        };
        let record = runner.exec(&action, Role::Step, &mut ctx).await;
        assert!(!record.ok, "{tool}");
        assert!(record.error.as_deref().unwrap().contains("not read-only"));
        assert!(!fake.names().contains(&tool.to_string()));
        assert!(!server.calls.lock().unwrap().iter().any(|c| c.0 == tool));
    }
}

#[test]
fn capture_specs_reject_cross_backend_reads() {
    let tmp = tempfile::tempdir().unwrap();
    let mut valid = request(tmp.path(), ROW).sections.remove(0).spec;
    for (source, tool) in [("tool", "server_sessions"), ("server", "client_inventory")] {
        valid.rows[0].steps[0].capture = Some(source.into());
        valid.rows[0].steps[0].tool = Some(tool.into());
        let error = spec::validate(&valid).unwrap_err();
        assert!(
            error.contains(&format!("capture source {source} cannot call {tool}")),
            "{error}"
        );
    }
}

#[tokio::test]
async fn capture_execution_rejects_cross_backend_reads_before_invoking() {
    let tmp = tempfile::tempdir().unwrap().keep();
    let fake = Fake::new(&[]).with("client_inventory");
    let server = FakeServer::new(json!({}));
    let mut runner = Runner::new(&fake, Some(&server), request(&tmp, ROW)).unwrap();
    for (source, tool) in [("tool", "server_sessions"), ("server", "client_inventory")] {
        let mut action = runner.req.sections[0].spec.rows[0].steps[0].clone();
        action.capture = Some(source.into());
        action.tool = Some(tool.into());
        let mut ctx = RowCtx {
            section: "test".into(),
            row_id: "capture".into(),
            vars: Default::default(),
            actions: vec![],
            blocked: vec![],
            unverified: vec![],
            chat_marks: Default::default(),
            attachments: vec![],
            character: json!({}),
            anchor: None,
            started_ms: 0,
            tap: None,
            event_marks: Default::default(),
            fp: None,
            fixture_mode: None,
            session_end: vec![],
            parity: None,
            poses: vec![],
            log_mark: None,
            chaos: Default::default(),
        };
        let record = runner.exec(&action, Role::Step, &mut ctx).await;
        assert!(!record.ok, "{source}:{tool}");
        assert!(record.error.as_deref().unwrap().contains("cannot call"));
        assert!(!fake.names().contains(&tool.to_string()));
        assert!(!server.calls.lock().unwrap().iter().any(|c| c.0 == tool));
    }
}

#[tokio::test]
async fn literal_id_mentions_in_guide_prose_do_not_bind_runtime_ids() {
    let fake = Fake::new(&[]).with("client_player_state");
    let rows = r#"
[[row]]
id = "prose-ids"
title = "Describe ${player_id}"
expected = "Show the literal ${entity_id} token."
notes = "Example: ${player_id}"
known_issues = ["An old ${entity_id} example"]
anchor = false
step = [{ wait_ms = 1, label = "${player_id}" }]
[[row.expect]]
id = "read"
text = "The guide says ${entity_id} here."
source = "tool"
tool = "client_player_state"
pointer = "/name"
op = "eq"
value = "Labone"
"#;
    let row = run(&fake, None, rows).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert!(!fake.names().contains(&"client_entity_find".to_string()));
}
