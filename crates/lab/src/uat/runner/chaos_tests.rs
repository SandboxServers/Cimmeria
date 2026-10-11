//! Chaos around rows (lab-chaos CH-08) over the fake client and the fake
//! lab MCP: `net`/`flush` are set on the row's player entity before its
//! setup and cleared after its teardown on every path, a run without chaos
//! sends no chaos call, and `burst` is refused before anything runs.

use std::time::Duration;

use serde_json::json;
use tokio::sync::watch;

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::{RunOutcome, Runner};
use crate::uat::evidence::{RowEvidence, RowResult, RunDir, RunManifest};
use crate::uat::invoke::ServerInvoker;

const ROWS: &str = r#"
[[row]]
id = "C1"
title = "first"
expected = "Answers."
step = [{ chat = ".help" }]
teardown = [{ chat = ".who" }]
[[row.expect]]
id = "help"
text = ".help answers"
source = "chat"
contains = "Available commands"

[[row]]
id = "C2"
title = "second"
expected = "Answers."
step = [{ chat = ".help" }]
teardown = [{ chat = ".who" }]
[[row.expect]]
id = "help"
text = ".help answers"
source = "chat"
contains = "Available commands"
"#;

fn fake() -> Fake {
    Fake::new(&[
        (".help", &["Available commands: .help"]),
        (".who", &["Players online: 1"]),
    ])
    .with("lab_fail")
}

async fn run_chaos(
    fake: &Fake,
    server: &FakeServer,
    rows: &str,
    chaos: &[&str],
) -> (RunOutcome, Vec<RowEvidence>, RunManifest) {
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, rows);
    req.chaos = chaos.iter().map(|s| s.to_string()).collect();
    let out = Runner::new(fake, Some(server as &dyn ServerInvoker), req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let run = RunDir::open(std::path::Path::new(&out.run_dir)).unwrap();
    let manifest: RunManifest = run.read_json(&run.run_json()).unwrap();
    (out, run.rows().unwrap(), manifest)
}

/// Where the first action named `what` sits in the row, if anywhere.
fn at(row: &RowEvidence, what: &str) -> Option<usize> {
    row.actions.iter().position(|a| a.requested == what)
}

/// Regression guard: each row sets its flush chaos on its own player
/// entity before the steps and clears it after the teardown. Fails if the
/// set moves after the steps or the clear is dropped.
#[tokio::test]
async fn flush_chaos_is_set_before_and_cleared_after_each_row() {
    let fake = fake();
    let server = FakeServer::new(json!({}));
    let (out, rows, manifest) = run_chaos(&fake, &server, ROWS, &["flush:pack"]).await;

    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert_eq!(
            row.result,
            RowResult::Pass,
            "{} {:?}",
            row.row_id,
            row.reasons
        );
        let set = at(row, "server_chaos_set").expect("chaos set");
        let step = at(row, ".help").expect("step");
        let teardown = at(row, ".who").expect("teardown");
        let clear = at(row, "server_chaos_clear").expect("chaos cleared");
        assert!(
            set < step && step < teardown && teardown < clear,
            "{}",
            row.row_id
        );
        let args = &row.actions[set].args;
        assert_eq!(args["entity_id"], 7, "scoped to the row's player entity");
        assert_eq!(args["ttl_s"], 360, "the row's 300 s budget plus a minute");
        assert_eq!(args["flush"], json!({ "mode": "pack", "records": 24 }));
        assert!(args.get("seed").is_none(), "pack reads no seed");
        assert_eq!(row.actions[clear].args, json!({ "entity_id": 7 }));
    }
    let chaos_calls: Vec<String> = server
        .names()
        .into_iter()
        .filter(|n| n.starts_with("server_chaos"))
        .collect();
    assert_eq!(
        chaos_calls,
        [
            "server_chaos_set",
            "server_chaos_clear",
            "server_chaos_set",
            "server_chaos_clear"
        ]
    );
    // The run reports and records the runner's own header, seed resolved.
    assert_eq!(out.chaos.len(), 1);
    assert!(
        out.chaos[0].starts_with("chaos=flush:pack seed="),
        "{:?}",
        out.chaos
    );
    assert_eq!(manifest.chaos.as_deref(), Some(out.chaos[0].as_str()));
}

/// Regression guard: a row whose step fails, and one whose setup fails
/// (so its teardown never runs), both still clear their chaos.
#[tokio::test]
async fn failed_row_still_clears_its_chaos() {
    let rows = r#"
[[row]]
id = "F1"
title = "step fails"
expected = "Fails."
step = [{ tool = "lab_fail", tier = "N1" }, { chat = ".help" }]
teardown = [{ chat = ".who" }]
[[row.expect]]
id = "help"
text = ".help answers"
source = "chat"
contains = "Available commands"

[[row]]
id = "F2"
title = "setup fails"
expected = "Blocked."
setup = [{ tool = "lab_fail", tier = "N1" }]
step = [{ chat = ".help" }]
[[row.expect]]
id = "help"
text = ".help answers"
source = "chat"
contains = "Available commands"
"#;
    let fake = fake();
    let server = FakeServer::new(json!({}));
    let (_, rows, _) = run_chaos(&fake, &server, rows, &["net:break"]).await;

    let step_failed = &rows[0];
    assert_ne!(step_failed.result, RowResult::Pass);
    let fail = at(step_failed, "lab_fail").expect("failed step");
    let clear = at(step_failed, "server_chaos_clear").expect("cleared after a failed step");
    assert!(fail < clear && at(step_failed, ".who").unwrap() < clear);
    let set = at(step_failed, "server_chaos_set").unwrap();
    assert!(
        step_failed.actions[set].args["seed"].as_u64().is_some(),
        "net reads the seed"
    );

    let setup_failed = &rows[1];
    assert_eq!(
        setup_failed.result,
        RowResult::Blocked,
        "{:?}",
        setup_failed.reasons
    );
    assert!(at(setup_failed, "server_chaos_set").is_some());
    assert!(
        at(setup_failed, "server_chaos_clear").is_some(),
        "cleared after a failed setup"
    );
}

/// Regression guard: a lease takeover cuts the row off mid-wait, and the
/// row's chaos is still cleared (the server call is not lease-gated).
#[tokio::test]
async fn a_takeover_mid_row_still_clears_the_chaos() {
    let rows = r#"
[[row]]
id = "W1"
title = "long"
expected = "Waits."
anchor = false
step = [{ wait_ms = 60000 }]
[[row.expect]]
id = "waited"
text = "the wait is met"
source = "wait"
lua_condition = "true"
"#;
    let fake = fake();
    let server = FakeServer::new(json!({}));
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, rows);
    req.chaos = vec!["flush:delay".into()];
    let (tx, rx) = watch::channel(None::<String>);
    let runner = Runner::new(&fake, Some(&server as &dyn ServerInvoker), req)
        .unwrap()
        .with_revocation(rx);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let _ = tx.send(Some("lease taken over by session-b".into()));
    });
    tokio::time::timeout(Duration::from_secs(10), runner.run_all())
        .await
        .expect("the run stopped at the takeover")
        .unwrap();
    let names = server.names();
    assert_eq!(
        names.last().map(String::as_str),
        Some("server_chaos_clear"),
        "{names:?}"
    );
}

/// A `{refused}` set (a server without DEVELOPER_MODE) blocks the row: it
/// must not grade as chaos-tested. Nothing was set, so nothing is cleared.
#[tokio::test]
async fn a_refused_set_blocks_the_row_and_drives_nothing() {
    let fake = fake();
    let server = FakeServer::new(json!({})).answer(
        "server_chaos_set",
        vec![json!({ "refused": "lab chaos is off (DEVELOPER_MODE)" })],
    );
    let (_, rows, _) = run_chaos(&fake, &server, ROWS, &["flush:pack"]).await;
    for row in &rows {
        assert_eq!(row.result, RowResult::Blocked, "{:?}", row.reasons);
        assert!(
            row.reasons.iter().any(|r| r.contains("DEVELOPER_MODE")),
            "{:?}",
            row.reasons
        );
        assert!(
            at(row, ".help").is_none(),
            "a row without its chaos drove a step"
        );
    }
    assert!(!server.names().iter().any(|n| n == "server_chaos_clear"));
}

/// No chaos, or cpu alone, sends no server chaos call and writes no
/// `chaos` key into run.json.
#[tokio::test]
async fn no_chaos_sends_no_chaos_calls() {
    for chaos in [&[][..], &["cpu:cores=1,duty=10"][..]] {
        let fake = fake();
        let server = FakeServer::new(json!({}));
        let (out, rows, manifest) = run_chaos(&fake, &server, ROWS, chaos).await;
        assert!(rows.iter().all(|r| r.result == RowResult::Pass));
        assert!(
            !server.names().iter().any(|n| n.starts_with("server_chaos")),
            "{chaos:?}: {:?}",
            server.names()
        );
        assert_eq!(out.chaos.is_empty(), chaos.is_empty());
        assert_eq!(manifest.chaos.is_none(), chaos.is_empty());
    }
}

/// `burst` is refused before the run starts until CH-13 lands (CH-08
/// worknote, "Burst: deferred"): no client call, no server call.
#[tokio::test]
async fn burst_is_refused_before_the_run() {
    let fake = fake();
    let server = FakeServer::new(json!({}));
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, ROWS);
    req.chaos = vec!["burst".into()];
    let err = Runner::new(&fake, Some(&server as &dyn ServerInvoker), req)
        .err()
        .expect("burst refused");
    assert!(
        err.contains("burst is not wired yet; it waits on CH-13"),
        "{err}"
    );
    assert!(fake.names().is_empty() && server.names().is_empty());
}

/// Without the lab MCP, net/flush chaos cannot be set, so the plan reports
/// the row BLOCKED (the CLI then exits 3) instead of SKIPPED "ready".
#[tokio::test]
async fn server_chaos_without_the_lab_mcp_blocks_the_plan() {
    let fake = fake();
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, ROWS);
    req.chaos = vec!["net".into()];
    req.plan_only = true;
    let out = Runner::new(&fake, None, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert!(
        out.rows.iter().all(|r| r.result == "BLOCKED"),
        "{:?}",
        out.rows
    );
    assert!(out.rows[0].reasons[0].contains("server lab MCP"));
}
