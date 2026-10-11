//! Run-level chaos (lab-chaos CH-08 review): the `server_chaos_status`
//! probe, a clear that fails twice stopping the run, which row states can
//! take net/flush, the TTL cap, the plan's header and the cpu load held for
//! the whole run.

use std::time::Duration;

use serde_json::json;

use super::chaos::LOGIN_ROW;
use super::chaos_tests::{at, fake, run_chaos, run_with, ROWS};
use super::test_server::FakeServer;
use crate::uat::evidence::RowResult;

/// Regression guard (coordinator decision): a clear that fails twice keeps
/// the row's own grade plus a flag, and every later row is BLOCKED without
/// being driven, since chaos may still be live. Fails if the run goes on.
#[tokio::test]
async fn a_clear_that_fails_twice_stops_the_run() {
    let fake = fake();
    let server = FakeServer::new(json!({})).answer(
        "server_chaos_clear",
        vec![json!({ "__error": "lab-mcp HTTP 502" })],
    );
    let (_, rows, _) = run_chaos(&fake, &server, ROWS, &["flush:pack"]).await;

    let first = &rows[0];
    assert_eq!(first.result, RowResult::Pass, "{:?}", first.reasons);
    assert!(
        first.flags.iter().any(
            |f| f.contains("chaos clear failed twice (lab-mcp HTTP 502)")
                && f.contains("may still be live on entity 7")
        ),
        "{:?}",
        first.flags
    );
    let second = &rows[1];
    assert_eq!(second.result, RowResult::Blocked, "{:?}", second.reasons);
    assert!(
        second
            .reasons
            .iter()
            .any(|r| r == "chaos may still be live on entity 7; run stopped"),
        "{:?}",
        second.reasons
    );
    assert!(at(second, "server_chaos_set").is_none() && at(second, ".help").is_none());
    let sets = server
        .names()
        .iter()
        .filter(|n| *n == "server_chaos_set")
        .count();
    assert_eq!(sets, 1);
}

/// A set that failed in transport and a clear that failed too: the flag
/// says the set was never confirmed (not "set but not cleared"), and the
/// run still stops, since the set may have landed.
#[tokio::test]
async fn an_unconfirmed_set_with_a_failed_clear_says_so_and_stops() {
    let fake = fake();
    let server = FakeServer::new(json!({}))
        .answer(
            "server_chaos_set",
            vec![json!({ "__error": "lab-mcp: operation timed out" })],
        )
        .answer(
            "server_chaos_clear",
            vec![json!({ "__error": "lab-mcp: operation timed out" })],
        );
    let (_, rows, _) = run_chaos(&fake, &server, ROWS, &["net"]).await;
    assert_eq!(rows[0].result, RowResult::Blocked);
    assert!(
        rows[0]
            .flags
            .iter()
            .any(|f| f.starts_with("chaos set was never confirmed (lab-mcp: operation timed out)")),
        "{:?}",
        rows[0].flags
    );
    assert!(rows[1].reasons.iter().any(|r| r.contains("run stopped")));
}

/// A server without lab chaos (the tools absent, or DEVELOPER_MODE off)
/// blocks every net/flush row before a set: the plan exits 3, and a real
/// run never arms a guard or sends a clear for chaos it never set.
#[tokio::test]
async fn a_server_without_lab_chaos_blocks_every_row() {
    for status in [
        json!({ "__error": "unknown tool server_chaos_status" }),
        json!({ "refused": "lab chaos is off (DEVELOPER_MODE)" }),
    ] {
        for plan in [true, false] {
            let fake = fake();
            let server =
                FakeServer::new(json!({})).answer("server_chaos_status", vec![status.clone()]);
            let (_, rows, _) =
                run_with(&fake, &server, ROWS, &["net"], |r| r.plan_only = plan).await;
            for row in &rows {
                assert_eq!(row.result, RowResult::Blocked, "{status} {plan}");
                assert!(
                    row.reasons
                        .iter()
                        .any(|r| r.contains("server_chaos_status")),
                    "{:?}",
                    row.reasons
                );
            }
            let names = server.names();
            assert!(
                !names
                    .iter()
                    .any(|n| n == "server_chaos_set" || n == "server_chaos_clear"),
                "{names:?}"
            );
        }
    }
}

const STATE_ROWS: &str = r#"
[[row]]
id = "L1"
title = "stopped"
expected = "x"
state = "client_stopped"
step = [{ chat = ".help" }]
[[row.expect]]
id = "help"
text = ".help answers"
source = "chat"
contains = "Available commands"

[[row]]
id = "L2"
title = "char select"
expected = "x"
state = "char_select"
step = [{ chat = ".help" }]
[[row.expect]]
id = "help"
text = ".help answers"
source = "chat"
contains = "Available commands"

[[row]]
id = "A1"
title = "any"
expected = "x"
state = "any"
step = [{ chat = ".help" }]
[[row.expect]]
id = "help"
text = ".help answers"
source = "chat"
contains = "Available commands"

[[row]]
id = "T1"
title = "too long"
expected = "x"
anchor = false
step = [{ wait_ms = 1600000 }]
[[row.expect]]
id = "waited"
text = "the wait is met"
source = "wait"
lua_condition = "true"
"#;

/// Regression guard: login rows are BLOCKED naming CH-15, a row over the
/// TTL cap is BLOCKED, and an `any` row is left to run time (the plan says
/// it is ready). Fails if `any` is blocked statically again.
#[tokio::test]
async fn the_plan_blocks_login_rows_and_long_rows_but_not_any() {
    let fake = fake();
    let server = FakeServer::new(json!({}));
    let (out, rows, _) = run_with(&fake, &server, STATE_ROWS, &["flush:pack"], |r| {
        r.plan_only = true
    })
    .await;
    let by_id = |id: &str| rows.iter().find(|r| r.row_id == id).unwrap();
    for id in ["L1", "L2"] {
        let row = by_id(id);
        assert_eq!(row.result, RowResult::Blocked, "{id}");
        assert!(
            row.reasons.iter().any(|r| r.starts_with(LOGIN_ROW)),
            "{:?}",
            row.reasons
        );
    }
    let any = by_id("A1");
    assert_eq!(any.result, RowResult::Skipped, "{:?}", any.reasons);
    let long = by_id("T1");
    assert_eq!(long.result, RowResult::Blocked);
    assert!(long
        .reasons
        .iter()
        .any(|r| r.contains("over the server's 1800 s")));
    // The plan does not know the run's seed.
    assert_eq!(out.chaos, ["chaos=flush:pack seed=<per run>"]);
}

/// An `any` row is checked when its chaos is to be set: with no player in
/// world it is BLOCKED with a message saying so, and nothing is set.
#[tokio::test]
async fn an_any_row_with_no_player_in_world_is_blocked_at_run_time() {
    let fake = fake();
    let nobody = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let server = FakeServer::new(json!({})).in_world_when(nobody);
    let (_, rows, _) = run_with(&fake, &server, STATE_ROWS, &["net"], |r| {
        r.rows = Some(vec!["A1".into()])
    })
    .await;
    assert_eq!(rows[0].result, RowResult::Blocked);
    assert!(
        rows[0]
            .reasons
            .iter()
            .any(|r| r.contains("this any row had no player in world")),
        "{:?}",
        rows[0].reasons
    );
    assert!(!server.names().iter().any(|n| n == "server_chaos_set"));

    // In world, the same row runs under its chaos.
    let client = super::chaos_tests::fake();
    let server = FakeServer::new(json!({}));
    let (_, rows, _) = run_with(&client, &server, STATE_ROWS, &["net"], |r| {
        r.rows = Some(vec!["A1".into()])
    })
    .await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    assert!(at(&rows[0], "server_chaos_set").is_some());
}

/// Regression guard: the cpu load runs while the rows do. Fails if
/// `run_all` drops its guard at once (`let _ = ...` in place of `let _cpu`).
/// The thread count is process-wide; nextest runs each test in its own
/// process, so only this run's threads count.
#[tokio::test]
async fn the_cpu_load_is_held_while_rows_run() {
    let rows = r#"
[[row]]
id = "W1"
title = "waits"
expected = "x"
anchor = false
step = [{ wait_ms = 800 }]
[[row.expect]]
id = "waited"
text = "the wait is met"
source = "wait"
lua_condition = "true"
"#;
    let fake = fake();
    let server = FakeServer::new(json!({}));
    let sampler = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(400)).await;
        crate::chaos::cpu::live_threads()
    });
    let (_, evidence, _) = run_chaos(&fake, &server, rows, &["cpu:cores=1,duty=5"]).await;
    assert!(!evidence[0].actions.is_empty(), "the row drove nothing");
    assert!(sampler.await.unwrap() >= 1, "no cpu load while the row ran");
}
