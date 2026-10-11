//! Fingerprint-mode runner tests (lab-golden GD-06) over the fake client
//! and the fake `cimmeria-lab-mcp`: every row is tapped, a row that enters
//! the world mid-way binds its tap late, windows and entities are captured,
//! and the grade is the same with the mode on or off.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{RowEvidence, RowResult, RunDir, Verdict};
use crate::uat::invoke::{ServerInvoker, ToolInvoker, ToolOutcome};
use crate::uat::tier::Role;

/// A tool-only row with no packet clause: setup, a labelled step and an
/// unlabelled one, and a Lua clause that passes (the fake reads "40").
const TOOL_ROW: &str = r#"
[[row]]
id = "FP-1"
title = "tool row"
expected = "Range is 40."
anchor = false
setup = [{ tool = "client_wait_for", tier = "N1" }]
step = [
  { tool = "client_wait_for", tier = "N1", label = "frost" },
  { tool = "client_wait_for", tier = "N1" },
]
[[row.expect]]
id = "range"
text = "range is 40"
source = "lua"
chunk = "return 40"
op = "approx"
value = 40
tolerance = 1
"#;

/// A row whose character enters the world at the step labelled `play`.
const LATE_ROW: &str = r#"
[[row]]
id = "FP-2"
title = "enters the world"
expected = "Range is 40."
anchor = false
step = [
  { tool = "client_wait_for", tier = "N1", label = "login" },
  { tool = "lab_play_character", tier = "N1", label = "play" },
  { tool = "client_wait_for", tier = "N1" },
]
[[row.expect]]
id = "range"
text = "range is 40"
source = "lua"
chunk = "return 40"
op = "approx"
value = 40
tolerance = 1
"#;

/// The client every test drives: windows come back from every UI read.
fn client() -> Fake {
    Fake::new(&[])
        .with("lab_play_character")
        .with("lab_fail")
        .answer(
            "client_ui_state",
            vec![json!({ "windows": ["SelfStatusWin"], "chat_tail": [] })],
        )
}

/// The fake client, opening `flag` (the character is in the world) when
/// `lab_play_character` runs.
struct Entering<'a> {
    fake: &'a Fake,
    flag: Arc<AtomicBool>,
}

impl ToolInvoker for Entering<'_> {
    fn has_tool(&self, name: &str) -> bool {
        self.fake.has_tool(name)
    }

    fn tool_names(&self) -> Vec<String> {
        self.fake.tool_names()
    }

    async fn call(&self, name: &str, args: Value) -> ToolOutcome {
        let out = self.fake.call(name, args).await;
        if name == "lab_play_character" {
            self.flag.store(true, Ordering::SeqCst);
        }
        out
    }
}

async fn run_fp<I: ToolInvoker>(
    inv: &I,
    server: Option<&FakeServer>,
    rows: &str,
    fingerprint: bool,
) -> (RowEvidence, RunDir) {
    let tmp = tempfile::tempdir().unwrap().keep();
    let server = server.map(|s| s as &dyn ServerInvoker);
    let mut req = request(&tmp, rows);
    req.fingerprint = fingerprint;
    let out = Runner::new(inv, server, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let run = RunDir::open(std::path::Path::new(&out.run_dir)).unwrap();
    let row = run.rows().unwrap().remove(0);
    (row, run)
}

/// The attachment `name`'s JSON, if the row has one.
fn attachment(row: &RowEvidence, run: &RunDir, name: &str) -> Option<Value> {
    let a = row.attachments.iter().find(|a| a.name == name)?;
    let text = std::fs::read_to_string(run.root.join(&a.path)).unwrap();
    Some(serde_json::from_str(&text).unwrap())
}

fn recorded(row: &RowEvidence, tool: &str) -> usize {
    row.actions.iter().filter(|a| a.requested == tool).count()
}

fn server_calls(server: &FakeServer, tool: &str) -> usize {
    server.names().iter().filter(|n| *n == tool).count()
}

/// `TOOL_ROW` plus a packet clause the canned tap read satisfies.
fn packet_row() -> String {
    format!(
        "{TOOL_ROW}[[row.expect]]\nid = \"timer\"\ntext = \"a timer\"\nsource = \"packet\"\n\
         message = \"onTimerUpdate\"\ndirection = \"to_client\"\nmin_rows = 1\n"
    )
}

/// One `onTimerUpdate` to the player, entity 7.
fn timer_tap() -> Value {
    json!({ "count": 1, "dropped": 0, "messages": [
        { "ts_ms": 1, "dir": "out", "msg_id": null, "method_index": 40,
          "msg_name": "onTimerUpdate", "target_entity_id": 7, "args_len": 0,
          "args_hex": "", "decoded": {} },
    ]})
}

/// Run `rows` with the character entering the world at `lab_play_character`
/// (or in the world from the start when the row never plays).
async fn run_entering(
    rows: &str,
    server: impl FnOnce(Arc<AtomicBool>) -> FakeServer,
    fingerprint: bool,
) -> (RowEvidence, RunDir, FakeServer) {
    let fake = client();
    let flag = Arc::new(AtomicBool::new(!rows.contains("lab_play_character")));
    let inv = Entering {
        fake: &fake,
        flag: flag.clone(),
    };
    let server = server(flag);
    let (row, run) = run_fp(&inv, Some(&server), rows, fingerprint).await;
    (row, run, server)
}

#[tokio::test]
async fn fingerprint_mode_taps_a_row_without_packet_clauses() {
    let fake = client();
    let server = FakeServer::new(json!({ "messages": [], "count": 0, "dropped": 0 }));
    let (row, run) = run_fp(&fake, Some(&server), TOOL_ROW, true).await;
    let tap = attachment(&row, &run, "packet_tap").expect("a packet_tap attachment");
    assert_eq!(tap["coverage"], "full");
    assert_eq!(tap["replaced_existing"], false);
    assert_eq!(tap["entity_id"], 7);
    assert_eq!(tap["stopped"], true);
    for tool in [
        "server_packet_tap_start",
        "server_packet_tap_read",
        "server_packet_tap_stop",
    ] {
        assert!(server.names().contains(&tool.to_string()), "{tool}");
    }
    assert_eq!(server_calls(&server, "server_packet_tap_start"), 1);
    assert_eq!(recorded(&row, "server_packet_tap_start"), 1);

    let server = FakeServer::new(json!({}));
    let (row, run) = run_fp(&fake, Some(&server), TOOL_ROW, false).await;
    assert!(attachment(&row, &run, "packet_tap").is_none());
    assert!(attachment(&row, &run, "windows").is_none());
    assert!(server.names().is_empty(), "{:?}", server.names());
}

#[tokio::test]
async fn a_row_that_enters_the_world_binds_its_tap_late() {
    let fake = client();
    let flag = Arc::new(AtomicBool::new(false));
    let inv = Entering {
        fake: &fake,
        flag: flag.clone(),
    };
    let server = FakeServer::new(json!({ "messages": [] })).in_world_when(flag);
    let (row, run) = run_fp(&inv, Some(&server), LATE_ROW, true).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);

    let tap = attachment(&row, &run, "packet_tap").unwrap();
    assert_eq!(tap["coverage"], "late:play");
    // Looked up at the start, after `login` and after `play`, silently.
    let lookups = server
        .names()
        .iter()
        .filter(|n| *n == "server_sessions")
        .count();
    assert_eq!(lookups, 3);
    assert_eq!(recorded(&row, "server_sessions"), 0);
    assert_eq!(recorded(&row, "server_packet_tap_start"), 1);
    assert!(
        row.actions
            .iter()
            .filter(|a| a.role != Role::Step)
            .all(|a| a.ok),
        "no recorded failed attempts: {:?}",
        row.actions
    );
    // The late-bound tap is still read and stopped.
    assert_eq!(
        server.names().last().map(String::as_str),
        Some("server_packet_tap_stop")
    );
}

#[tokio::test]
async fn windows_are_sampled_after_every_action() {
    let fake = client().answer(
        "client_ui_state",
        vec![
            json!({ "windows": ["SelfStatusWin"] }),
            json!({ "windows": ["DialogWin", "SelfStatusWin"] }),
            json!({ "__error": "lua timeout" }),
            json!({ "windows": [] }),
        ],
    );
    let (row, run) = run_fp(&fake, None, TOOL_ROW, true).await;
    let w = attachment(&row, &run, "windows").unwrap();
    let samples = w["samples"].as_array().unwrap();
    let after: Vec<&str> = samples
        .iter()
        .map(|s| s["after"].as_str().unwrap())
        .collect();
    assert_eq!(after, ["start", "setup#0", "frost", "step#1"]);
    assert_eq!(samples[0]["windows"], json!(["SelfStatusWin"]));
    assert_eq!(samples[1]["windows"], json!(["DialogWin", "SelfStatusWin"]));
    assert_eq!(samples[2]["error"], "lua timeout");
    assert_eq!(samples[3]["windows"], json!([]));
    // Samples are reads, never recorded actions.
    assert_eq!(recorded(&row, "client_ui_state"), 0);
}

#[tokio::test]
async fn entities_are_snapshotted_at_bind_and_finish() {
    let fake = client();
    let server = FakeServer::new(json!({ "messages": [] }));
    let (row, run) = run_fp(&fake, Some(&server), TOOL_ROW, true).await;
    let e = attachment(&row, &run, "entities").unwrap();
    assert_eq!(e["player"], 7);
    assert_eq!(e["space_id"], 3);
    let snaps = e["snapshots"].as_array().unwrap();
    let at: Vec<&str> = snaps.iter().map(|s| s["at"].as_str().unwrap()).collect();
    assert_eq!(at, ["bind", "finish"]);
    assert_eq!(snaps[0]["capped"], false);
    assert_eq!(
        snaps[0]["entities"][1],
        json!({ "entity_id": 100751, "is_player": false, "tag": "ArmYourself_FrostBody",
                "spawn_id": null, "template_id": 14 })
    );
    assert_eq!(snaps[0]["entities"][2]["spawn_id"], 412);
    // Each snapshot is one recorded get and query, as setup.
    assert_eq!(recorded(&row, "server_entity_get"), 2);
    assert_eq!(recorded(&row, "server_entity_query"), 2);
    // The deferred records are re-indexed: indices stay positions.
    for (i, a) in row.actions.iter().enumerate() {
        assert_eq!(a.index, i, "{:?}", a.requested);
    }
    // The finish snapshot is taken before the tap is read.
    let pos = |tool: &str| server.names().iter().rposition(|n| n == tool).unwrap();
    assert!(pos("server_entity_query") < pos("server_packet_tap_read"));
}

#[tokio::test]
async fn without_a_server_the_coverage_is_unavailable() {
    let fake = client();
    let (row, run) = run_fp(&fake, None, TOOL_ROW, true).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    let tap = attachment(&row, &run, "packet_tap").unwrap();
    assert_eq!(tap["coverage"], "unavailable");
    assert!(tap["error"].as_str().unwrap().contains("not configured"));
    let e = attachment(&row, &run, "entities").unwrap();
    assert_eq!(e["snapshots"], json!([]));
    assert!(e.get("player").is_none());

    // A server that never lists the character: `none`, with the reason.
    let server = FakeServer::new(json!({})).in_world_when(Arc::new(AtomicBool::new(false)));
    let (row, run) = run_fp(&fake, Some(&server), TOOL_ROW, true).await;
    let tap = attachment(&row, &run, "packet_tap").unwrap();
    assert_eq!(tap["coverage"], "none");
    assert!(tap["error"]
        .as_str()
        .unwrap()
        .contains("no in-world session"));
    assert_eq!(recorded(&row, "server_packet_tap_start"), 0);
}

/// `[row.golden] skip` leaves a row out of fingerprint capture.
#[tokio::test]
async fn a_skipped_row_is_not_captured() {
    let fake = client();
    let server = FakeServer::new(json!({}));
    let rows = format!("{TOOL_ROW}[row.golden]\nskip = true\n");
    let (row, run) = run_fp(&fake, Some(&server), &rows, true).await;
    assert!(attachment(&row, &run, "windows").is_none());
    assert!(server.names().is_empty());
}

/// What the grade is made of, per row: result, reasons, step actions,
/// clauses (timestamps stripped), blocks and flags.
fn graded(row: &RowEvidence) -> (RowResult, Vec<String>, Vec<Value>, Vec<Value>, Vec<String>) {
    let steps = row
        .actions
        .iter()
        .filter(|a| a.role == Role::Step)
        .map(|a| json!([a.index, a.requested, a.ok, a.error, a.label]))
        .collect();
    let clauses = row
        .clauses
        .iter()
        .map(|c| {
            let mut v = serde_json::to_value(c).unwrap();
            v.as_object_mut().unwrap().remove("evaluated_ms");
            v
        })
        .collect();
    let mut marks = row.blocked.clone();
    marks.extend(row.flags.iter().map(|f| format!("flag:{f}")));
    (row.result, row.reasons.clone(), steps, clauses, marks)
}

/// Regression guard: fingerprint mode never changes a grade. The failing
/// row binds late before its failing step, so a capture that recorded its
/// calls in line would move that step's index in the FAIL reason.
#[tokio::test]
async fn fingerprint_mode_does_not_change_the_grade() {
    let failing = LATE_ROW
        .replace(
            "{ tool = \"client_wait_for\", tier = \"N1\" },\n]",
            "{ tool = \"lab_fail\", tier = \"N1\" },\n]",
        )
        .replace("value = 40", "value = 10");
    assert!(failing.contains("lab_fail"), "the failing row was built");
    let packet_row = packet_row();
    let mut seen = Vec::new();
    for rows in [TOOL_ROW, LATE_ROW, failing.as_str(), packet_row.as_str()] {
        let mut grades = Vec::new();
        for fingerprint in [false, true] {
            let server = |flag| FakeServer::new(timer_tap()).in_world_when(flag);
            let (row, _, _) = run_entering(rows, server, fingerprint).await;
            grades.push(graded(&row));
        }
        assert_eq!(grades[0], grades[1], "{rows}");
        seen.push(grades[0].0);
    }
    // The cases cover a pass and a fail, so the guard is not vacuous, and
    // the packet row's clause really graded the tap.
    assert_eq!(seen[3], RowResult::Pass, "packet row: {seen:?}");
    assert!(seen.contains(&RowResult::Fail), "{seen:?}");
}

/// A row with a packet clause keeps the tap `tap_start` gives it: when its
/// character is not in the world at the row's start, fingerprint mode
/// does not start one later, so its clause grades the same on or off.
#[tokio::test]
async fn a_packet_row_is_never_late_bound() {
    let rows = format!(
        "{LATE_ROW}[[row.expect]]\nid = \"timer\"\ntext = \"a timer\"\nsource = \"packet\"\n\
         message = \"onTimerUpdate\"\ndirection = \"to_client\"\nmin_rows = 1\n"
    );
    let mut runs = Vec::new();
    for fingerprint in [false, true] {
        let server = |flag| FakeServer::new(timer_tap()).in_world_when(flag);
        let (row, _, server) = run_entering(&rows, server, fingerprint).await;
        let timer = row.clauses.iter().find(|c| c.id == "timer").unwrap();
        assert_ne!(timer.verdict, Verdict::Pass, "fingerprint={fingerprint}");
        runs.push((
            graded(&row),
            server_calls(&server, "server_packet_tap_start"),
        ));
    }
    assert_eq!(runs[0], runs[1]);
}

/// Fingerprint mode's late bind finds the session itself and never sets
/// `${player_entity_id}`, which a clause could read.
#[tokio::test]
async fn fingerprint_mode_leaves_player_entity_id_unset() {
    let rows = format!(
        "{LATE_ROW}[[row.expect]]\nid = \"entity\"\ntext = \"reads the entity var\"\n\
         source = \"lua\"\nchunk = \"return '${{player_entity_id}}'\"\nop = \"eq\"\n\
         value = \"${{player_entity_id}}\"\n"
    );
    let mut runs = Vec::new();
    for fingerprint in [false, true] {
        let server = |flag| FakeServer::new(json!({ "messages": [] })).in_world_when(flag);
        let (row, run, _) = run_entering(&rows, server, fingerprint).await;
        if fingerprint {
            let tap = attachment(&row, &run, "packet_tap").unwrap();
            assert_eq!(tap["coverage"], "late:play", "the late bind ran");
        }
        assert!(row.vars.get("player_entity_id").is_none(), "{:?}", row.vars);
        let c = row.clauses.iter().find(|c| c.id == "entity").unwrap();
        runs.push((
            row.vars.clone(),
            c.expected.clone(),
            c.observed.clone(),
            c.detail.clone(),
            graded(&row),
        ));
    }
    assert_eq!(runs[0], runs[1]);
}

/// A start that replaced another tap on the character says so, whoever
/// started it: fingerprint mode's bind or a packet clause's `tap_start`.
#[tokio::test]
async fn a_tap_that_replaced_an_observer_reports_it() {
    for rows in [TOOL_ROW.to_string(), packet_row()] {
        let fake = client();
        let server = FakeServer::new(timer_tap()).answer(
            "server_packet_tap_start",
            vec![json!({ "entity_id": 7, "replaced_existing": true })],
        );
        let (row, run) = run_fp(&fake, Some(&server), &rows, true).await;
        let tap = attachment(&row, &run, "packet_tap").unwrap();
        assert_eq!(tap["replaced_existing"], true, "{rows}");
        assert_eq!(tap["coverage"], "full", "{rows}");
    }
}

/// A snapshot that fails keeps its error in `entities.json`; the row is
/// graded as if fingerprint mode were off.
#[tokio::test]
async fn a_failed_snapshot_does_not_change_the_grade() {
    let fake = client();
    let server = FakeServer::new(json!({ "messages": [] })).answer(
        "server_entity_get",
        vec![json!({ "__error": "entity gone" })],
    );
    let (row, run) = run_fp(&fake, Some(&server), TOOL_ROW, true).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert!(row.reasons.is_empty(), "{:?}", row.reasons);
    let e = attachment(&row, &run, "entities").unwrap();
    let snaps = e["snapshots"].as_array().unwrap();
    assert_eq!(snaps.len(), 2);
    for s in snaps {
        assert!(s["error"].as_str().unwrap().contains("entity gone"), "{s}");
        assert!(s.get("entities").is_none(), "{s}");
    }
    assert_eq!(server_calls(&server, "server_entity_query"), 0);
}

/// A late start the server refuses is tried once, recorded once, and the
/// tap's coverage is `none` with the refusal.
#[tokio::test]
async fn a_failed_late_start_is_recorded_once() {
    let server = |flag| {
        FakeServer::new(json!({ "messages": [] }))
            .in_world_when(flag)
            .answer(
                "server_packet_tap_start",
                vec![json!({ "__error": "tap refused" })],
            )
    };
    let (row, run, server) = run_entering(LATE_ROW, server, true).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert_eq!(server_calls(&server, "server_packet_tap_start"), 1);
    assert_eq!(recorded(&row, "server_packet_tap_start"), 1);
    let tap = attachment(&row, &run, "packet_tap").unwrap();
    assert_eq!(tap["coverage"], "none");
    assert!(
        tap["error"].as_str().unwrap().contains("tap refused"),
        "{tap}"
    );
    // Nothing to read or stop.
    assert_eq!(server_calls(&server, "server_packet_tap_read"), 0);
}

/// A windows section that failed in Lua still answers `windows: []`: the
/// sample is the error, not "no windows open".
#[tokio::test]
async fn a_failed_window_read_is_an_error_not_no_windows() {
    let fake = client().answer(
        "client_ui_state",
        vec![json!({ "windows": [], "errors": ["windows: boom"] })],
    );
    let (row, run) = run_fp(&fake, None, TOOL_ROW, true).await;
    let w = attachment(&row, &run, "windows").unwrap();
    let first = &w["samples"][0];
    assert_eq!(first["error"], "windows: boom", "{first}");
    assert!(first.get("windows").is_none(), "{first}");
}
