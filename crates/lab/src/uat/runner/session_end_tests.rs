//! TG-PIPE-08 (S1): a run ends with `lab_logout` then `lab_disconnect`
//! after its last row, each checked against the server line it causes,
//! recorded as row `_session_end`; a plan drives neither.

use serde_json::{json, Value};

use super::session::SESSION_END_ROW;
use super::tests::{request, Fake};
use super::{RunOutcome, Runner};
use crate::uat::evidence::{RowEvidence, RunDir};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};

const ROW: &str = r#"
[[row]]
id = "M1-1"
title = "help"
expected = "Each answers in chat."
step = [{ chat = ".help" }]
[[row.expect]]
id = "help-answers"
text = ".help lists commands"
source = "chat"
contains = "Available commands"
"#;

const ACCOUNT: &str = "lab1";

/// A server log ring that fills the way the base logs a session ending:
/// `session.end` once the client logged out (unless `no_session_end`), the
/// account teardown once it disconnected.
struct EndRing<'a> {
    fake: &'a Fake,
    no_session_end: bool,
}

impl EndRing<'_> {
    fn called(&self, tool: &str) -> bool {
        self.fake.names().iter().any(|n| n == tool)
    }

    fn lines(&self) -> Vec<Value> {
        let mut v = vec![json!({ "timestamp_ms": 1000, "target": "cimmeria_base",
            "message": "Client connected", "fields": {} })];
        if self.called("lab_logout") && !self.no_session_end {
            v.push(json!({ "timestamp_ms": 2000, "target": "session.end",
                "message": "player session ended",
                "fields": { "disconnect_reason": "logoff_character_select",
                            "account_name": ACCOUNT, "player_id": 71 } }));
        }
        if self.called("lab_disconnect") {
            v.push(json!({ "timestamp_ms": 3000,
                "target": "cimmeria_base_session::base::helpers::session_teardown",
                "message": "Client entities cleaned up",
                "fields": { "disconnect_reason": "logoff", "account_name": ACCOUNT,
                            "account_id": 5 } }));
        }
        v
    }

    /// `server_log_tail`: `newest_ms` over the ring, then the filters.
    fn tail(&self, args: &Value) -> Value {
        let ring = self.lines();
        let newest = ring.iter().filter_map(|l| l["timestamp_ms"].as_u64()).max();
        let entries: Vec<Value> = ring
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
            .collect();
        json!({ "count": entries.len(), "total_buffered": entries.len(),
                "newest_ms": newest, "entries": entries })
    }
}

impl ServerInvoker for EndRing<'_> {
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

async fn run(fake: &Fake, ring: &EndRing<'_>, plan_only: bool) -> (RunOutcome, Vec<RowEvidence>) {
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, ROW);
    req.end_session = true;
    req.plan_only = plan_only;
    req.account_name = Some(ACCOUNT.into());
    let out = Runner::new(fake, Some(ring as &dyn ServerInvoker), req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let rows = RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap();
    (out, rows)
}

/// The session-ending flows the client saw, in order.
fn ends(fake: &Fake) -> Vec<String> {
    fake.names()
        .into_iter()
        .filter(|n| n == "lab_logout" || n == "lab_disconnect")
        .collect()
}

/// Regression guard (TG-PIPE-08): after the last row the run logs out,
/// waits for `session.end disconnect_reason=logoff_character_select`,
/// disconnects, and waits for `Client entities cleaned up
/// disconnect_reason=logoff`, both pinned to the run's account. Without
/// the end-of-run hook neither flow runs and no `_session_end` row exists.
#[tokio::test(start_paused = true)]
async fn run_all_ends_the_session() {
    let fake = Fake::new(&[(".help", &["Available commands: .help"])]).with("lab_disconnect");
    let ring = EndRing {
        fake: &fake,
        no_session_end: false,
    };
    let (out, rows) = run(&fake, &ring, false).await;

    assert_eq!(ends(&fake), ["lab_logout", "lab_disconnect"]);
    // Both come after the last row's own action (its chat key).
    let names = fake.names();
    let row_done = names.iter().rposition(|n| n == "client_input_key").unwrap();
    let logout = names.iter().position(|n| n == "lab_logout").unwrap();
    assert!(
        logout > row_done,
        "the logout must follow the last row: {names:?}"
    );

    let summary = out.rows.last().unwrap();
    assert_eq!(summary.row, SESSION_END_ROW);
    assert_eq!(summary.result, "PASS", "{:?}", summary.reasons);
    let end = rows.iter().find(|r| r.row_id == SESSION_END_ROW).unwrap();
    let waits: Vec<_> = end
        .actions
        .iter()
        .filter(|a| a.kind == "wait_server")
        .collect();
    assert_eq!(waits.len(), 2, "{:?}", end.actions);
    let reasons: Vec<&Value> = waits
        .iter()
        .map(|a| &a.result["matched"]["fields"]["disconnect_reason"])
        .collect();
    assert_eq!(
        reasons,
        [&json!("logoff_character_select"), &json!("logoff")]
    );
    for w in &waits {
        assert_eq!(w.args["fields"]["account_name"], ACCOUNT, "{:?}", w.args);
    }
}

/// A plan drives nothing: no logout, no disconnect, no `_session_end` row.
#[tokio::test(start_paused = true)]
async fn a_plan_does_not_end_the_session() {
    let fake = Fake::new(&[]).with("lab_disconnect");
    let ring = EndRing {
        fake: &fake,
        no_session_end: false,
    };
    let (out, _) = run(&fake, &ring, true).await;
    assert!(ends(&fake).is_empty(), "{:?}", fake.names());
    assert!(out.rows.iter().all(|r| r.row != SESSION_END_ROW));
}

/// The server clause is real: no `session.end` after the logout fails the
/// row, and the disconnect still runs so the account does not stay online.
#[tokio::test(start_paused = true)]
async fn a_missing_session_end_line_fails_the_row_but_still_disconnects() {
    let fake = Fake::new(&[(".help", &["Available commands: .help"])]).with("lab_disconnect");
    let ring = EndRing {
        fake: &fake,
        no_session_end: true,
    };
    let (out, _) = run(&fake, &ring, false).await;
    assert_eq!(ends(&fake), ["lab_logout", "lab_disconnect"]);
    let summary = out.rows.last().unwrap();
    assert_eq!(summary.row, SESSION_END_ROW);
    assert_eq!(summary.result, "FAIL");
    assert!(
        summary
            .reasons
            .iter()
            .any(|r| r.contains("player session ended")),
        "{:?}",
        summary.reasons
    );
}

/// A client already stopped at the run's end has nothing to end.
#[tokio::test(start_paused = true)]
async fn a_stopped_client_is_left_alone() {
    let rows = r#"
[[row]]
id = "S1"
title = "stopped"
expected = "nothing"
state = "client_stopped"
step = [{ tool = "lab_client_status" }]
[[row.expect]]
id = "stopped"
text = "the client is stopped"
source = "tool"
tool = "lab_client_status"
pointer = "/running"
op = "eq"
value = false
"#;
    let fake = Fake::new(&[])
        .with("lab_disconnect")
        .with("lab_client_stop");
    let ring = EndRing {
        fake: &fake,
        no_session_end: false,
    };
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, rows);
    req.end_session = true;
    let out = Runner::new(&fake, Some(&ring as &dyn ServerInvoker), req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert!(ends(&fake).is_empty(), "{:?}", fake.names());
    assert!(out.rows.iter().all(|r| r.row != SESSION_END_ROW));
}
