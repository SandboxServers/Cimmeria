//! AP-03 typed captures, automatic ids, and unavailable-server grading.

use serde_json::json;

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{RowResult, RunDir};
use crate::uat::invoke::ServerInvoker;

const ROW: &str = r#"
[[row]]
id = "capture"
title = "typed captures"
expected = "The inventory count drops by one."
anchor = false
step = [
  { capture = "tool", tool = "client_inventory", args = { container = 1 }, pointer = "/items/0/count", var = "before" },
  { capture = "server", tool = "server_inventory", args = { player_id = "${player_id}" }, pointer = "/items/0/count", var = "server_before" },
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
        .answer(
            "server_inventory",
            vec![json!({"items":[{"count":99_u64}]})],
        );
    let row = run(&fake, Some(&server), ROW).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert_eq!(row.vars["before"], 3);
    assert_eq!(row.vars["server_before"], 99);
    assert_eq!(row.vars["player_id"], 700);
    assert_eq!(row.vars["entity_id"], 7);
    assert_eq!(row.clauses[0].observed, 2);
    let calls = server.calls.lock().unwrap();
    assert_eq!(
        calls.iter().find(|c| c.0 == "server_inventory").unwrap().1["player_id"],
        700
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
  { capture = "server", tool = "server_inventory", args = {}, pointer = "/count", var = "count" },
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
