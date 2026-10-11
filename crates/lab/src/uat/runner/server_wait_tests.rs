//! SV-08: `wait_server` steps and polled tool/server clauses, over a fake
//! client and a fake server lab MCP, on a paused clock so every poll count
//! and elapsed time is exact.

use std::sync::Mutex;

use serde_json::{json, Value};
use tokio::time::Instant;

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{ActionRecord, ClauseResult, RowEvidence, RunDir, Verdict};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker};

/// A server that notes when (on the paused clock) each call was made.
struct Timed<'a> {
    inner: &'a FakeServer,
    t0: Instant,
    at: Mutex<Vec<(String, Value, u64)>>,
}

impl<'a> Timed<'a> {
    fn new(inner: &'a FakeServer) -> Self {
        Self {
            inner,
            t0: Instant::now(),
            at: Mutex::new(vec![]),
        }
    }

    /// `(args, ms)` of every call to `tool`, in order.
    fn calls_to(&self, tool: &str) -> Vec<(Value, u64)> {
        self.at
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.0 == tool)
            .map(|c| (c.1.clone(), c.2))
            .collect()
    }
}

impl ServerInvoker for Timed<'_> {
    fn url(&self) -> &str {
        self.inner.url()
    }

    fn call<'b>(&'b self, name: &'b str, args: Value) -> BoxedOutcome<'b> {
        let ms = self.t0.elapsed().as_millis() as u64;
        self.at
            .lock()
            .unwrap()
            .push((name.to_string(), args.clone(), ms));
        self.inner.call(name, args)
    }
}

/// Run the one row; also return how long it took on the paused clock.
async fn run(fake: &Fake, server: &dyn ServerInvoker, rows: &str) -> (RowEvidence, u64) {
    let tmp = tempfile::tempdir().unwrap().keep();
    let t0 = Instant::now();
    let out = Runner::new(fake, Some(server), request(&tmp, rows))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let ms = t0.elapsed().as_millis() as u64;
    let row = RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0);
    (row, ms)
}

fn wait_action(row: &RowEvidence) -> &ActionRecord {
    row.actions
        .iter()
        .find(|a| a.kind == "wait_server")
        .expect("the wait ran")
}

fn clause<'r>(row: &'r RowEvidence, id: &str) -> &'r ClauseResult {
    row.clauses.iter().find(|c| c.id == id).expect("clause ran")
}

/// One `server_log_tail` entry.
fn line(ts: u64, target: &str, message: &str, fields: Value) -> Value {
    json!({ "timestamp_ms": ts, "level": "INFO", "target": target, "message": message, "fields": fields })
}

fn tail(newest_ms: u64, entries: Vec<Value>) -> Value {
    json!({ "count": entries.len(), "total_buffered": entries.len(), "newest_ms": newest_ms, "entries": entries })
}

const HOLD: &str = "aoi.cinematic_hold";

/// A row: `pre` (TOML action, or empty), then the wait.
fn wait_row(pre: &str, wait: &str) -> String {
    format!(
        r#"
[[row]]
id = "SV-W"
title = "wait"
expected = "Waits."
step = [{pre}{{ wait_server = {wait}, label = "released" }}]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "x"
"#
    )
}

/// The player is entity 8 on the fake client.
fn client() -> Fake {
    Fake::new(&[]).answer("client_entity_find", vec![json!({ "player": { "id": 8 } })])
}

#[tokio::test(start_paused = true)]
async fn a_wait_passes_on_the_release_for_this_player() {
    let fake = client();
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![
            tail(100, vec![]),
            tail(
                125,
                vec![
                    line(110, HOLD, "hold released", json!({ "witness_id": 9 })),
                    line(
                        120,
                        HOLD,
                        "hold released",
                        json!({ "witness_id": 8, "other": 1 }),
                    ),
                ],
            ),
        ],
    );
    let timed = Timed::new(&server);
    let rows = wait_row(
        "{ wait_ms = 300 }, ",
        r#"{ log = "hold released", target = "aoi.cinematic_hold", fields = { witness_id = "${player_entity_id}" }, timeout_ms = 2000 }"#,
    );
    let (row, ms) = run(&fake, &timed, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    assert_eq!(a.requested, "wait for server log 'hold released'");
    assert!(a.tier.is_none());
    // The 110 line is another player's; only the named field comes back.
    assert_eq!(
        a.result,
        json!({ "matched": { "timestamp_ms": 120, "message": "hold released", "fields": { "witness_id": 8 } } })
    );
    assert_eq!(row.vars["player_entity_id"], 8);
    let calls = timed.calls_to("server_log_tail");
    assert_eq!(calls.len(), 2, "{calls:?}");
    // The mark is read before the 300 ms action, not when the wait starts.
    assert_eq!(calls[0].1, 0);
    assert!(calls[0].0.get("since_ms").is_none());
    assert_eq!(calls[0].0["contains"], "hold released");
    // The first poll reads from the mark's millisecond on.
    assert_eq!(calls[1].1, 300);
    assert_eq!(calls[1].0["since_ms"], 99);
    assert_eq!(calls[1].0["target"], HOLD);
    assert_eq!(ms, 300);
}

#[tokio::test(start_paused = true)]
async fn an_entry_before_the_mark_does_not_count() {
    let fake = client();
    // An old server ignores `since_ms`: the 90 line comes back every poll.
    let old = line(90, HOLD, "hold released", json!({ "witness_id": 8 }));
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![tail(100, vec![]), tail(100, vec![old])],
    );
    let timed = Timed::new(&server);
    let rows = wait_row(
        "",
        r#"{ log = "hold released", fields = { witness_id = "${player_entity_id}" }, timeout_ms = 1000 }"#,
    );
    let (row, ms) = run(&fake, &timed, &rows).await;
    let a = wait_action(&row);
    assert!(!a.ok);
    let err = r#"no server log 'hold released' {"witness_id":8} within 1000 ms"#;
    assert_eq!(a.error.as_deref(), Some(err));
    assert!(
        row.reasons.iter().any(|r| r.contains(err)),
        "{:?}",
        row.reasons
    );
    // The first action takes its own mark, then polls at 0, 250 .. 1000.
    let at: Vec<u64> = timed
        .calls_to("server_log_tail")
        .iter()
        .map(|c| c.1)
        .collect();
    assert_eq!(at, vec![0, 0, 250, 500, 750, 1000]);
    assert_eq!(ms, 1000);
}

#[tokio::test(start_paused = true)]
async fn a_line_in_the_marks_own_millisecond_is_still_seen() {
    let fake = client();
    let before = line(100, HOLD, "hold released early", json!({ "witness_id": 8 }));
    let after = line(100, HOLD, "hold released now", json!({ "witness_id": 8 }));
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![
            tail(100, vec![before.clone()]),
            tail(100, vec![before, after]),
        ],
    );
    let timed = Timed::new(&server);
    let rows = wait_row(
        "{ wait_ms = 300 }, ",
        r#"{ log = "hold released", fields = { witness_id = 8 }, timeout_ms = 1000 }"#,
    );
    let (row, _) = run(&fake, &timed, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    // Not the line that was already there when the mark was read.
    assert_eq!(a.result["matched"]["message"], "hold released now");
    assert_eq!(a.result["matched"]["timestamp_ms"], 100);
    assert_eq!(timed.calls_to("server_log_tail")[1].0["since_ms"], 99);
}

#[tokio::test(start_paused = true)]
async fn the_waits_own_audit_rows_never_satisfy_it() {
    let fake = client();
    // Every poll is audited as "lab MCP tool call" with its args, and an
    // old server returns unrelated lines too.
    let audit = |ts| {
        line(
            ts,
            "lab.tool_call",
            "lab MCP tool call",
            json!({ "tool": "server_log_tail", "args": "{\"contains\":\"tool call\"}" }),
        )
    };
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![
            tail(100, vec![]),
            tail(
                130,
                vec![
                    audit(110),
                    line(120, "cell.other", "unrelated", json!({})),
                    audit(130),
                ],
            ),
        ],
    );
    let timed = Timed::new(&server);
    let rows = wait_row("", r#"{ log = "tool call", timeout_ms = 500 }"#);
    let (row, ms) = run(&fake, &timed, &rows).await;
    let a = wait_action(&row);
    assert_eq!(
        a.error.as_deref(),
        Some("no server log 'tool call' within 500 ms")
    );
    let calls = timed.calls_to("server_log_tail");
    // Mark, then polls at 0, 250 and 500; each one names the text.
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|c| c.0["contains"] == "tool call"));
    assert_eq!(ms, 500);
}

#[tokio::test(start_paused = true)]
async fn a_string_number_matches_a_number() {
    let fake = client();
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![
            tail(100, vec![]),
            tail(
                105,
                vec![line(
                    105,
                    HOLD,
                    "hold released",
                    json!({ "witness_id": "8" }),
                )],
            ),
        ],
    );
    let rows = wait_row(
        "",
        r#"{ log = "hold released", fields = { witness_id = 8 }, timeout_ms = 1000 }"#,
    );
    let (row, _) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    assert_eq!(a.result["matched"]["fields"]["witness_id"], "8");
}

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
