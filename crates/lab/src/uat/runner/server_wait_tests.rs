//! SV-08: `wait_server` steps and polled tool/server clauses, over a fake
//! client and a fake server lab MCP, on a paused clock so every poll count
//! and elapsed time is exact.

use std::sync::Mutex;

use serde_json::{json, Value};
use tokio::time::Instant;

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{ActionRecord, ClauseResult, RowEvidence, RunDir};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};

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

/// When a [`Ring`] line is in the server's log ring.
#[derive(Clone, Copy)]
enum When {
    Always,
    /// Once the client has typed this chat line.
    Typed(&'static str),
    /// From this many ms on the paused clock.
    At(u64),
}

/// A server whose log ring fills as the row runs, read the way
/// `server_log_tail` reads it: `newest_ms` over the whole ring, then
/// `contains`, `target` and `since_ms`, then the newest `limit` lines.
struct Ring<'a> {
    fake: &'a Fake,
    t0: Instant,
    /// In push order.
    lines: Vec<(When, Value)>,
}

impl<'a> Ring<'a> {
    fn new(fake: &'a Fake, lines: Vec<(When, Value)>) -> Self {
        Self {
            fake,
            t0: Instant::now(),
            lines,
        }
    }

    fn logged(&self, when: When) -> bool {
        match when {
            When::Always => true,
            When::At(ms) => self.t0.elapsed().as_millis() as u64 >= ms,
            When::Typed(text) => self
                .fake
                .log
                .lock()
                .unwrap()
                .iter()
                .any(|(n, a)| n == "client_type_text" && a["text"] == text),
        }
    }

    fn tail(&self, args: &Value) -> Value {
        let ring: Vec<&Value> = self
            .lines
            .iter()
            .filter(|(w, _)| self.logged(*w))
            .map(|(_, l)| l)
            .collect();
        let newest = ring.iter().filter_map(|l| l["timestamp_ms"].as_u64()).max();
        let matched: Vec<Value> = ring
            .into_iter()
            .filter(|l| {
                args["contains"]
                    .as_str()
                    .is_none_or(|c| l["message"].as_str().unwrap().contains(c))
                    && args["target"].as_str().is_none_or(|t| l["target"] == t)
                    && args["since_ms"]
                        .as_u64()
                        .is_none_or(|s| l["timestamp_ms"].as_u64().unwrap() > s)
            })
            .cloned()
            .collect();
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;
        let entries = matched[matched.len().saturating_sub(limit)..].to_vec();
        json!({ "count": entries.len(), "total_buffered": matched.len(), "newest_ms": newest, "entries": entries })
    }
}

impl ServerInvoker for Ring<'_> {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'b>(&'b self, name: &'b str, args: Value) -> BoxedOutcome<'b> {
        let out = match name {
            "server_log_tail" => ToolOutcome {
                ok: true,
                json: self.tail(&args),
                ..Default::default()
            },
            _ => ToolOutcome::err(format!("unknown tool {name}")),
        };
        Box::pin(async move { out })
    }
}

/// A server whose calls to `tool` after the first `after` never return.
pub(super) struct Stall<'a> {
    pub(super) inner: &'a FakeServer,
    pub(super) tool: &'static str,
    pub(super) after: usize,
    pub(super) n: Mutex<usize>,
}

impl ServerInvoker for Stall<'_> {
    fn url(&self) -> &str {
        self.inner.url()
    }

    fn call<'b>(&'b self, name: &'b str, args: Value) -> BoxedOutcome<'b> {
        if name == self.tool {
            let mut n = self.n.lock().unwrap();
            *n += 1;
            if *n > self.after {
                return Box::pin(std::future::pending());
            }
        }
        self.inner.call(name, args)
    }
}

/// Run the one row; also return how long it took on the paused clock.
pub(super) async fn run(fake: &Fake, server: &dyn ServerInvoker, rows: &str) -> (RowEvidence, u64) {
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

pub(super) fn clause<'r>(row: &'r RowEvidence, id: &str) -> &'r ClauseResult {
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
    // The first poll reads from a millisecond before the mark on.
    assert_eq!(calls[1].1, 300);
    assert_eq!(calls[1].0["since_ms"], 98);
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
    assert_eq!(timed.calls_to("server_log_tail")[1].0["since_ms"], 98);
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

/// A row with `setup`, then `steps`, then a chat clause.
fn row_with(setup: &str, steps: &str) -> String {
    format!(
        r#"
[[row]]
id = "SV-W"
title = "wait"
expected = "Waits."
setup = [{setup}]
step = [{steps}]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "x"
"#
    )
}

/// The ring's first line: something older than anything a wait is for.
fn boot() -> (When, Value) {
    (When::Always, line(100, "cell.boot", "boot", json!({})))
}

#[tokio::test(start_paused = true)]
async fn a_first_step_wait_sees_the_line_its_setup_caused() {
    let fake = client();
    // The NPC is logged while the setup's chat line is sent.
    let spawned = line(105, "cell.npc", "npc spawned", json!({ "id": 3 }));
    let server = Ring::new(&fake, vec![boot(), (When::Typed(".summon"), spawned)]);
    let rows = row_with(
        r#"{ chat = ".summon" }"#,
        r#"{ wait_server = { log = "spawned", timeout_ms = 1000 } }"#,
    );
    let (row, _) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    assert_eq!(a.result["matched"]["timestamp_ms"], 105);
    // The mark was read before the setup action, not after it.
    assert_eq!(a.calls[0]["mark_ms"], 100);
}

#[tokio::test(start_paused = true)]
async fn a_wait_finds_its_line_among_the_oldest_of_sixty() {
    let fake = client();
    let mut lines = vec![boot()];
    for n in 0..60u64 {
        let l = line(101 + n, "cell.burst", "burst", json!({ "n": n }));
        lines.push((When::At(100), l));
    }
    let server = Ring::new(&fake, lines);
    let rows = wait_row(
        "{ wait_ms = 300 }, ",
        r#"{ log = "burst", fields = { n = 2 }, timeout_ms = 1000 }"#,
    );
    let (row, ms) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    assert_eq!(a.result["matched"]["timestamp_ms"], 103);
    assert_eq!(ms, 300);
}

#[tokio::test(start_paused = true)]
async fn a_line_stamped_before_the_mark_but_pushed_after_it_is_seen() {
    let fake = client();
    // Stamped 99, a millisecond before the mark's newest line, but in the
    // ring only after the mark was read.
    let late = line(99, HOLD, "hold released", json!({}));
    let server = Ring::new(&fake, vec![boot(), (When::At(200), late)]);
    let rows = wait_row(
        "{ wait_ms = 300 }, ",
        r#"{ log = "hold released", timeout_ms = 1000 }"#,
    );
    let (row, _) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    assert_eq!(a.result["matched"]["timestamp_ms"], 99);
}

#[tokio::test(start_paused = true)]
async fn the_second_of_two_waits_waits_from_before_the_first() {
    let fake = client();
    // The second wait's line lands while the first one is still waiting.
    let server = Ring::new(
        &fake,
        vec![
            boot(),
            (When::At(100), line(105, "cell.t", "second done", json!({}))),
            (When::At(200), line(110, "cell.t", "first done", json!({}))),
        ],
    );
    let rows = row_with(
        "",
        r#"{ wait_server = { log = "first done", timeout_ms = 1000 } }, { wait_server = { log = "second done", timeout_ms = 1000 } }"#,
    );
    let (row, ms) = run(&fake, &server, &rows).await;
    let waits: Vec<&ActionRecord> = row
        .actions
        .iter()
        .filter(|a| a.kind == "wait_server")
        .collect();
    assert_eq!(waits.len(), 2);
    assert!(waits[0].ok, "{:?}", waits[0].error);
    assert!(waits[1].ok, "{:?}", waits[1].error);
    assert_eq!(waits[1].result["matched"]["timestamp_ms"], 105);
    assert_eq!(waits[1].calls[0]["mark_ms"], 100);
    assert_eq!(ms, 250);
}

#[tokio::test(start_paused = true)]
async fn a_text_field_that_parses_as_nan_still_matches() {
    let fake = client();
    let named = line(105, HOLD, "named", json!({ "player_name": "Nan" }));
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![tail(100, vec![]), tail(105, vec![named])],
    );
    let rows = wait_row(
        "",
        r#"{ log = "named", fields = { player_name = "Nan" }, timeout_ms = 1000 }"#,
    );
    let (row, _) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    assert_eq!(a.result["matched"]["fields"]["player_name"], "Nan");
}

#[tokio::test(start_paused = true)]
async fn a_failed_pre_action_mark_is_recorded() {
    let fake = client();
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![
            json!({ "__error": "lab-mcp: boom" }),
            tail(100, vec![]),
            tail(105, vec![line(105, HOLD, "hold released", json!({}))]),
        ],
    );
    let rows = wait_row(
        "{ wait_ms = 300 }, ",
        r#"{ log = "hold released", timeout_ms = 1000 }"#,
    );
    let (row, _) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert!(a.ok, "{:?}", a.error);
    assert_eq!(
        a.calls[0]["pre_action_mark"],
        "server_log_tail: lab-mcp: boom"
    );
    assert_eq!(a.calls[1]["mark_ms"], 100);
}

#[tokio::test(start_paused = true)]
async fn an_unset_var_in_the_log_text_fails_at_once() {
    let fake = client();
    let server = FakeServer::new(json!({})).answer("server_log_tail", vec![tail(100, vec![])]);
    let rows = wait_row("", r#"{ log = "spawned ${npc}", timeout_ms = 1000 }"#);
    let (row, ms) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert_eq!(a.error.as_deref(), Some("unresolved variable ${npc}"));
    assert!(!server.names().iter().any(|n| n == "server_log_tail"));
    assert_eq!(ms, 0);

    let rows = wait_row(
        "",
        r#"{ log = "spawned", target = "${tgt}", timeout_ms = 1000 }"#,
    );
    let (row, _) = run(&fake, &server, &rows).await;
    assert_eq!(
        wait_action(&row).error.as_deref(),
        Some("unresolved variable ${tgt}")
    );
}

#[tokio::test(start_paused = true)]
async fn a_failed_log_read_while_polling_fails_the_wait_at_once() {
    let fake = client();
    let server = FakeServer::new(json!({})).answer(
        "server_log_tail",
        vec![tail(100, vec![]), json!({ "__error": "lab-mcp: boom" })],
    );
    let rows = wait_row("", r#"{ log = "hold released", timeout_ms = 1000 }"#);
    let (row, ms) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert_eq!(a.error.as_deref(), Some("server_log_tail: lab-mcp: boom"));
    assert_eq!(a.calls.last().unwrap()["ok"], false);
    assert_eq!(ms, 0);
}

#[tokio::test(start_paused = true)]
async fn a_poll_that_never_returns_expires_at_the_deadline() {
    let fake = client();
    let inner = FakeServer::new(json!({})).answer("server_log_tail", vec![tail(100, vec![])]);
    // The mark is read; the first poll hangs.
    let server = Stall {
        inner: &inner,
        tool: "server_log_tail",
        after: 1,
        n: Mutex::new(0),
    };
    let rows = wait_row("", r#"{ log = "hold released", timeout_ms = 1000 }"#);
    let (row, ms) = run(&fake, &server, &rows).await;
    let a = wait_action(&row);
    assert_eq!(
        a.error.as_deref(),
        Some("no server log 'hold released' within 1000 ms")
    );
    assert_eq!(ms, 1000);
}
