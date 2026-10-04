//! Packet-clause runner tests (AB-L4) over the fake client and a fake
//! `cimmeria-lab-mcp`: the tap goes on the lab character's session at the
//! anchor, is read and stopped before teardown on every path, its rows
//! become a row attachment, and an unreachable endpoint is UNVERIFIED.

use std::sync::Mutex;

use serde_json::{json, Value};

use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{RowEvidence, RowResult, RunDir, Verdict};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};

/// A pretend lab endpoint: two sessions, a canned tap read.
struct FakeServer {
    calls: Mutex<Vec<(String, Value)>>,
    tap: Value,
    /// Every call fails like the colo's 2026-09-29 HTTP 403.
    unreachable: bool,
}

impl FakeServer {
    fn new(tap: Value) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            tap,
            unreachable: false,
        }
    }

    fn names(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.0.clone())
            .collect()
    }
}

impl ServerInvoker for FakeServer {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
        self.calls.lock().unwrap().push((name.to_string(), args));
        let ok = |json: Value| ToolOutcome {
            ok: true,
            json,
            ..Default::default()
        };
        let out = if self.unreachable {
            ToolOutcome::err("lab-mcp HTTP 403 Forbidden: Host header is not allowed")
        } else {
            match name {
                "server_sessions" => ok(json!({ "count": 2, "sessions": [
                    { "entity_id": 7, "name": "Labone", "status": "in_world" },
                    { "entity_id": 9, "name": "Labtwo", "status": "in_world" },
                ]})),
                "server_packet_tap_start" => {
                    ok(json!({ "entity_id": 7, "replaced_existing": false }))
                }
                "server_packet_tap_read" => ok(self.tap.clone()),
                "server_packet_tap_stop" => ok(json!({ "entity_id": 7, "stopped": true })),
                _ => ToolOutcome::err(format!("unknown tool {name}")),
            }
        };
        Box::pin(async move { out })
    }
}

fn tap_with_cooldown(seconds: f64) -> Value {
    json!({
        "entity_id": 7, "capacity": 5000, "count": 2, "dropped": 0,
        "messages": [
            { "ts_ms": 1, "dir": "in", "msg_id": 9, "method_index": 12, "msg_name": "useAbility",
              "target_entity_id": null, "args_len": 8, "args_hex": "00", "decoded": null },
            { "ts_ms": 2, "dir": "out", "msg_id": null, "method_index": 40, "msg_name": "onTimerUpdate",
              "target_entity_id": 7, "args_len": 12, "args_hex": "00",
              "decoded": { "complete_in_s": seconds } },
        ]
    })
}

const COOLDOWN_ROW: &str = r#"
[[row]]
id = "AB-1"
title = "cooldown sent"
expected = "The server sends a 15 s cooldown."
step = [{ chat = ".help" }]
teardown = [{ chat = ".cleareffects" }]
[[row.expect]]
id = "timer"
text = "a 15 s timer reaches the client"
source = "packet"
message = "onTimerUpdate"
direction = "to_client"
entity = "${player_entity_id}"
min_rows = 1
field = "complete_in_s"
op = "approx"
value = 15
tolerance = 1
"#;

async fn run_with(fake: &Fake, server: Option<&FakeServer>, rows: &str) -> RowEvidence {
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
async fn a_packet_clause_grades_the_tap_and_keeps_its_rows() {
    let fake = Fake::new(&[]);
    let server = FakeServer::new(tap_with_cooldown(15.3));
    let row = run_with(&fake, Some(&server), COOLDOWN_ROW).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert_eq!(row.clauses[0].observed["matching_rows"], 1);

    // Session found by the character's name, tap on its entity, then read
    // and stop before the teardown's chat line.
    let calls = server.calls.lock().unwrap().clone();
    let names: Vec<&str> = calls.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(
        names,
        [
            "server_sessions",
            "server_packet_tap_start",
            "server_packet_tap_read",
            "server_packet_tap_stop"
        ]
    );
    assert_eq!(calls[1].1["entity_id"], 7);
    let stop_at = row
        .actions
        .iter()
        .position(|a| a.requested == "server_packet_tap_stop")
        .unwrap();
    let teardown_at = row
        .actions
        .iter()
        .position(|a| a.requested == ".cleareffects")
        .unwrap();
    assert!(stop_at < teardown_at, "the tap must stop before teardown");

    let att = row
        .attachments
        .iter()
        .find(|a| a.name == "packet_tap")
        .unwrap();
    assert_eq!(row.clauses[0].evidence_refs, vec![att.path.clone()]);
    assert_eq!(row.vars["player_entity_id"], 7);
}

#[tokio::test]
async fn a_wrong_field_fails_and_an_empty_tap_fails() {
    let fake = Fake::new(&[]);
    let server = FakeServer::new(tap_with_cooldown(30.0));
    let row = run_with(&fake, Some(&server), COOLDOWN_ROW).await;
    assert_eq!(row.result, RowResult::Fail, "{:?}", row.reasons);

    let server = FakeServer::new(json!({ "messages": [], "dropped": 0, "count": 0 }));
    let row = run_with(&fake, Some(&server), COOLDOWN_ROW).await;
    assert_eq!(row.clauses[0].verdict, Verdict::Fail);
    assert_eq!(row.result, RowResult::Fail);
}

/// The endpoint refusing (or not configured) never passes a packet
/// clause: it is UNVERIFIED, naming why.
#[tokio::test]
async fn an_unreachable_endpoint_is_unverified_not_pass() {
    let fake = Fake::new(&[]);
    let mut server = FakeServer::new(tap_with_cooldown(15.0));
    server.unreachable = true;
    let row = run_with(&fake, Some(&server), COOLDOWN_ROW).await;
    assert_eq!(row.result, RowResult::Unverified, "{:?}", row.reasons);
    let c = &row.clauses[0];
    assert_eq!(c.verdict, Verdict::Unverified);
    assert!(
        c.detail.as_deref().unwrap().contains("403"),
        "{:?}",
        c.detail
    );
    // No tap was started, so none is read or stopped.
    assert_eq!(server.names(), ["server_sessions"]);

    let row = run_with(&fake, None, COOLDOWN_ROW).await;
    assert_eq!(row.result, RowResult::Unverified);
    assert!(row.clauses[0]
        .detail
        .as_deref()
        .unwrap()
        .contains("not configured"));
}

/// A row whose setup fails is BLOCKED and skips its teardown, but the
/// tap it started is still read and stopped.
#[tokio::test]
async fn the_tap_is_stopped_when_the_row_fails() {
    let fake = Fake::new(&[]).with("lab_fail");
    let server = FakeServer::new(tap_with_cooldown(15.0));
    let rows = COOLDOWN_ROW.replace(
        "step = [{ chat = \".help\" }]",
        "setup = [{ tool = \"lab_fail\", tier = \"N1\" }]\nstep = [{ chat = \".help\" }]",
    );
    let row = run_with(&fake, Some(&server), &rows).await;
    assert_eq!(row.result, RowResult::Blocked, "{:?}", row.reasons);
    assert!(server
        .names()
        .contains(&"server_packet_tap_stop".to_string()));
    assert!(!row.actions.iter().any(|a| a.requested == ".cleareffects"));

    // A failed step: the row FAILs and the tap is stopped all the same.
    let rows = COOLDOWN_ROW.replace(
        "step = [{ chat = \".help\" }]",
        "step = [{ tool = \"lab_fail\", tier = \"N1\" }]",
    );
    let server = FakeServer::new(tap_with_cooldown(15.0));
    let row = run_with(&fake, Some(&server), &rows).await;
    assert_eq!(row.result, RowResult::Fail, "{:?}", row.reasons);
    assert_eq!(
        server.names().last().map(String::as_str),
        Some("server_packet_tap_stop")
    );
}
