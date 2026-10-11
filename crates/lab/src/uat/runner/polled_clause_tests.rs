//! SV-08: tool and server clauses with `timeout_ms`, read again until they
//! pass, over a fake client and a fake server lab MCP on a paused clock.

use std::sync::Mutex;

use serde_json::{json, Value};

use super::server_wait_tests::{clause, run, Stall};
use super::test_server::FakeServer;
use super::tests::Fake;
use crate::uat::evidence::Verdict;

/// A server clause on the mission step, with an optional `timeout_ms`.
fn step_row(timeout: &str) -> String {
    format!(
        r#"
[[row]]
id = "SV-P"
title = "poll"
expected = "Reads the server."
step = [{{ chat = ".help" }}]
[[row.expect]]
id = "db"
text = "a row reaches step 80623"
source = "server"
tool = "server_db_query"
args = {{ sql = "SELECT 1" }}
pointer = "/rows/0/current_step_id"
op = "eq"
value = 80623
{timeout}
"#
    )
}

fn at_step(step: u32) -> Value {
    json!({ "rows": [{ "current_step_id": step }] })
}

#[tokio::test(start_paused = true)]
async fn a_server_clause_polls_until_it_passes() {
    let fake = Fake::new(&[]);
    let persisted = || {
        FakeServer::new(json!({})).answer(
            "server_db_query",
            vec![at_step(80622), at_step(80622), at_step(80623)],
        )
    };
    // README F9: the step is persisted a moment after the action.
    let server = persisted();
    let (row, ms) = run(&fake, &server, &step_row("timeout_ms = 2000")).await;
    let c = clause(&row, "db");
    assert_eq!(c.verdict, Verdict::Pass, "{:?}", c.detail);
    assert_eq!(c.detail.as_deref(), Some("polls: 3"));
    assert_eq!(c.observed, json!(80623));
    let reads = server
        .names()
        .iter()
        .filter(|n| *n == "server_db_query")
        .count();
    assert_eq!(reads, 3);
    assert_eq!(ms, 500);

    // Without timeout_ms the clause is read once, and fails.
    let server = persisted();
    let (row, ms) = run(&fake, &server, &step_row("")).await;
    let c = clause(&row, "db");
    assert_eq!(c.verdict, Verdict::Fail);
    assert!(c.detail.is_none());
    let reads = server
        .names()
        .iter()
        .filter(|n| *n == "server_db_query")
        .count();
    assert_eq!(reads, 1);
    assert_eq!(ms, 0);
}

#[tokio::test(start_paused = true)]
async fn a_polled_clause_that_never_passes_fails_at_its_timeout() {
    let fake = Fake::new(&[]);
    let server = FakeServer::new(json!({})).answer("server_db_query", vec![at_step(80622)]);
    let (row, ms) = run(&fake, &server, &step_row("timeout_ms = 1000")).await;
    let c = clause(&row, "db");
    assert_eq!(c.id, "db");
    assert_eq!(c.verdict, Verdict::Fail);
    // Reads at 0, 250, 500, 750 and 1000.
    assert_eq!(c.detail.as_deref(), Some("polls: 5"));
    assert_eq!(ms, 1000);
    assert!(
        row.reasons
            .iter()
            .any(|r| r.starts_with("clause db failed")),
        "{:?}",
        row.reasons
    );
}

#[tokio::test(start_paused = true)]
async fn an_unverified_server_clause_does_not_poll() {
    let fake = Fake::new(&[]);
    let mut server = FakeServer::new(json!({}));
    server.unreachable = true;
    let (row, ms) = run(&fake, &server, &step_row("timeout_ms = 2000")).await;
    let c = clause(&row, "db");
    assert_eq!(c.verdict, Verdict::Unverified);
    assert!(
        c.detail.as_deref().unwrap_or_default().contains("403"),
        "{:?}",
        c.detail
    );
    let reads = server
        .names()
        .iter()
        .filter(|n| *n == "server_db_query")
        .count();
    assert_eq!(reads, 1);
    assert_eq!(ms, 0);
}

#[tokio::test(start_paused = true)]
async fn a_blocked_tool_clause_with_a_timeout_is_not_read_again() {
    // Not required, so the row runs and the clause itself finds the tool
    // unrouted.
    let fake = Fake::new(&[]);
    let server = FakeServer::new(json!({}));
    let rows = r#"
[[row]]
id = "SV-P"
title = "poll"
expected = "Reads the client."
step = [{ chat = ".help" }]
[[row.expect]]
id = "inv"
text = "the bag holds it"
source = "tool"
tool = "client_inventory"
pointer = "/count"
op = "eq"
value = 1
required = false
timeout_ms = 2000
"#;
    let (row, ms) = run(&fake, &server, rows).await;
    let c = clause(&row, "inv");
    assert_eq!(c.verdict, Verdict::Blocked);
    assert_eq!(
        c.detail.as_deref(),
        Some("tool client_inventory is not routed")
    );
    assert_eq!(ms, 0);
}

#[tokio::test(start_paused = true)]
async fn a_polled_clause_captures_its_last_read() {
    let fake = Fake::new(&[]);
    let server =
        FakeServer::new(json!({})).answer("server_db_query", vec![at_step(80622), at_step(80623)]);
    let extra = "timeout_ms = 2000\ncapture_var = \"step\"";
    let (row, _) = run(&fake, &server, &step_row(extra)).await;
    let c = clause(&row, "db");
    assert_eq!(c.verdict, Verdict::Pass, "{:?}", c.detail);
    assert_eq!(c.detail.as_deref(), Some("polls: 2"));
    assert_eq!(row.vars["step"], 80623);
}

#[tokio::test(start_paused = true)]
async fn a_read_that_never_returns_fails_at_the_timeout() {
    let fake = Fake::new(&[]);
    let inner = FakeServer::new(json!({})).answer("server_db_query", vec![at_step(80622)]);
    // The first read fails the check; the second hangs.
    let server = Stall {
        inner: &inner,
        tool: "server_db_query",
        after: 1,
        n: Mutex::new(0),
    };
    let (row, ms) = run(&fake, &server, &step_row("timeout_ms = 1000")).await;
    let c = clause(&row, "db");
    assert_eq!(c.verdict, Verdict::Fail);
    assert_eq!(
        c.detail.as_deref(),
        Some("polls: 2; the read did not return within 1000 ms")
    );
    assert_eq!(ms, 1000);
}
