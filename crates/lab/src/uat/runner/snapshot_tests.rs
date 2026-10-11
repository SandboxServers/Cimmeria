//! Fixture parity runner tests (lab-fixtures FX-10): a row that continues
//! the chain is snapshotted before its setup and checked against its
//! fixture; a row that ran from its fixture records no parity.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::fixture_tests::{
    client, fixture_request, fx_listed, run, run_request, tools, Endpoint, CHAIN_ROW, FIXTURE_ROW,
    SNAP_ITEMS, SNAP_MISSIONS, SNAP_TUTORIALS,
};
use super::snapshot::SETTLE_READS;
use super::FixtureMode;
use crate::uat::attest::{attest, AttestRequest};
use crate::uat::evidence::{RowEvidence, RowResult, RunDir, Verdict};
use crate::uat::snapshot::RowSnapshot;

/// A `server_db_query` reply holding `rows`.
pub(super) fn reply(rows: Value) -> Value {
    let n = rows.as_array().map_or(0, Vec::len);
    json!({ "row_count": n, "truncated": false, "row_cap": 500, "rows": rows })
}

/// The chain's mission rows: 622 on `step`, plus `extra`.
pub(super) fn chain_missions(step: i64, extra: &[Value]) -> Value {
    let mut rows = vec![json!({ "id": 622, "status": 1, "step": step })];
    rows.extend_from_slice(extra);
    reply(json!(rows))
}

/// How many snapshot mission reads the endpoint served.
pub(super) fn mission_reads(server: &Endpoint<'_>) -> usize {
    server
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, a)| a["sql"].as_str().is_some_and(|s| s.contains(SNAP_MISSIONS)))
        .count()
}

pub(super) fn parity_reason(row: &RowEvidence) -> Option<&String> {
    row.reasons
        .iter()
        .find(|r| r.starts_with("fixture parity: "))
}

/// The snapshot attachment's path, relative to the run directory.
fn snapshot_rel(row: &RowEvidence) -> &str {
    &row.attachments
        .iter()
        .find(|a| a.name == "snapshot")
        .expect("snapshot attachment")
        .path
}

/// The one run directory a test run created under `root`.
pub(super) fn run_dir_under(root: &Path) -> PathBuf {
    std::fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.join("run.json").is_file())
        .expect("run dir")
}

#[tokio::test]
async fn a_chained_row_with_a_parity_mismatch_fails() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(SNAP_MISSIONS, vec![chain_missions(2113, &[])]);
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    // A chained row with no fixture has nothing to compare.
    assert!(rows[0].fixture_parity.is_none());
    let b = &rows[1];
    assert_eq!(b.vars["fixture_mode"], "chain");
    // A mismatch that never settles is read once plus SETTLE_READS times.
    assert_eq!(mission_reads(&server), 1 + SETTLE_READS);
    // Every clause and step held: only parity turns the row.
    assert!(b.clauses.iter().all(|c| c.verdict == Verdict::Pass));
    assert_eq!(b.result, RowResult::Fail, "{:?}", b.reasons);
    assert_eq!(
        parity_reason(b).map(String::as_str),
        Some("fixture parity: mission 622: chain has step 2113, fixture wants 80623"),
        "{:?}",
        b.reasons
    );
    let p = b.fixture_parity.as_ref().expect("parity recorded");
    assert_eq!(p.mismatches.len(), 1, "{p:?}");
    assert_eq!(server.count("server_stage_fixture"), 0);
}

#[tokio::test]
async fn a_chained_row_with_parity_passes_and_writes_its_snapshot() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(
        SNAP_MISSIONS,
        vec![chain_missions(
            80623,
            &[json!({ "id": 1360, "status": 2, "step": null })],
        )],
    );
    server.db_replies(
        SNAP_ITEMS,
        vec![reply(json!([{ "id": 55, "container": 1, "n": 1 }]))],
    );
    server.db_replies(SNAP_TUTORIALS, vec![reply(json!([{ "id": 5882 }]))]);
    let req = fixture_request(&format!("{CHAIN_ROW}{FIXTURE_ROW}"), FixtureMode::Auto);
    let root = req.root.clone();
    let rows = run_request(&fake, Some(&server), req).await;
    let b = &rows[1];
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    let p = b.fixture_parity.as_ref().expect("parity recorded");
    assert!(p.mismatches.is_empty(), "{p:?}");
    // A chain mission the fixture does not name is reported, not failed.
    assert_eq!(
        p.unspecified,
        vec!["mission 1360: chain has it completed, fixture does not name it".to_string()]
    );
    assert_eq!(
        b.flags,
        vec![
            "fixture parity: mission 1360: chain has it completed, fixture does not name it"
                .to_string()
        ]
    );
    // The snapshot sits beside the row JSON, and `RunDir::rows` (which
    // `run` used to read these rows back) does not take it for a row.
    let rel = snapshot_rel(b);
    assert_eq!(rel, "rows/gm-parity/FS-B.snapshot.json");
    let run = RunDir::open(&run_dir_under(&root)).unwrap();
    assert_eq!(run.root.join(rel), run.snapshot_json("gm-parity", "FS-B"));
    let snap: RowSnapshot = run.read_json(&run.root.join(rel)).unwrap();
    assert_eq!(snap.character, "Labone");
    assert_eq!(snap.world.as_deref(), Some("Castle_CellBlock"));
    assert_eq!(snap.missions.len(), 2);
    assert_eq!(snap.items[0].count, 1);
    assert_eq!(snap.tutorials_seen, vec![5882]);
    // The snapshot's SQL is end-anchored on the chain character.
    let calls = server.calls.lock().unwrap();
    let sql: Vec<&str> = calls
        .iter()
        .filter(|(n, _)| n == "server_db_query")
        .filter_map(|(_, a)| a["sql"].as_str())
        .collect();
    assert_eq!(sql.len(), 3, "{sql:?}");
    assert!(
        sql.iter().all(|s| s.contains("ILIKE '% labone'")),
        "{sql:?}"
    );
}

/// A packet clause, so the row also starts a tap.
const PACKET_CLAUSE: &str = r#"
[[row.expect]]
id = "p"
text = "a timer reaches the client"
source = "packet"
message = "onTimerUpdate"
direction = "to_client"
entity = "${player_entity_id}"
min_rows = 1
"#;

#[tokio::test]
async fn the_snapshot_runs_before_the_anchor_tap_and_setup_as_fixture_plumbing() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}{PACKET_CLAUSE}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    let b = &rows[1];
    let at = |what: &str| {
        b.actions
            .iter()
            .position(|a| a.requested == what)
            .unwrap_or_else(|| panic!("{what} not run: {:?}", tools(b)))
    };
    let snap_at = at("server_db_query");
    let anchor_at = at(".bug uat FS-B");
    let tap_at = at("server_packet_tap_start");
    let setup_at = at(".cleareffects");
    assert!(
        snap_at < anchor_at && anchor_at < tap_at && tap_at < setup_at,
        "{:?}",
        tools(b)
    );
    // Everything before the anchor is the snapshot.
    let snap_calls: Vec<_> = b.actions[..anchor_at]
        .iter()
        .filter(|a| a.requested == "server_db_query" || a.requested == "server_sessions")
        .collect();
    assert_eq!(snap_calls.len(), 4, "{:?}", tools(b));
    assert!(
        snap_calls.iter().all(|a| a.kind == "fixture"),
        "{snap_calls:?}"
    );
}

#[tokio::test]
async fn a_fixture_row_records_no_parity() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    // The chain would mismatch; the fixture path never looks at it.
    server.db_replies(SNAP_MISSIONS, vec![chain_missions(2113, &[])]);
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.vars["fixture_mode"], "fixture");
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert!(row.fixture_parity.is_none());
    assert!(!row.attachments.iter().any(|a| a.name == "snapshot"));
    let calls = server.calls.lock().unwrap();
    assert!(
        !calls
            .iter()
            .any(|(_, a)| a["sql"].as_str().is_some_and(|s| s.contains(SNAP_MISSIONS))),
        "a fixture row ran the snapshot"
    );
}

#[tokio::test]
async fn a_snapshot_error_is_a_reason_not_a_failure() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.db_error = Some("lab-mcp HTTP 403 Forbidden".into());
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    let b = &rows[1];
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    assert!(
        b.reasons
            .iter()
            .any(|r| r == "fixture parity not checked: missions: lab-mcp HTTP 403 Forbidden"),
        "{:?}",
        b.reasons
    );
    let p = b.fixture_parity.as_ref().expect("parity recorded");
    assert!(
        p.mismatches.is_empty() && p.snapshot_error.is_some(),
        "{p:?}"
    );
    assert!(!b.attachments.iter().any(|a| a.name == "snapshot"));
}

#[tokio::test]
async fn no_lab_endpoint_leaves_a_chained_row_unchecked() {
    let fake = client(fx_listed());
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, None, &rows, FixtureMode::Auto).await;
    let b = &rows[1];
    assert_eq!(b.vars["fixture_mode"], "chain");
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    assert!(
        b.reasons
            .iter()
            .any(|r| r.starts_with("fixture parity not checked: no server lab MCP")),
        "{:?}",
        b.reasons
    );
}

/// `lab_uat_attest` re-grades a row from its record: an attestation that
/// makes it PASS must still meet the parity mismatch.
#[tokio::test]
async fn an_attestation_keeps_a_parity_failure() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(SNAP_MISSIONS, vec![chain_missions(2113, &[])]);
    let human = format!(
        "{FIXTURE_ROW}[[row.expect]]\nid = \"h\"\ntext = \"looks right\"\n\
         source = \"human\"\nquestion = \"Does it look right?\"\n"
    );
    let req = fixture_request(&format!("{CHAIN_ROW}{human}"), FixtureMode::Auto);
    let root = req.root.clone();
    let rows = run_request(&fake, Some(&server), req).await;
    let b = &rows[1];
    // Waiting on a person: parity is a flag until the row would pass.
    assert_eq!(b.result, RowResult::NeedsHuman, "{:?}", b.reasons);
    assert!(b.flags.iter().any(|f| f.starts_with("fixture parity: ")));
    let out = attest(&AttestRequest {
        run_dir: run_dir_under(&root),
        row: Some("FS-B".into()),
        clause: Some("h".into()),
        verdict: Some(Verdict::Pass),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(out["result"], "FAIL", "{out}");
    assert!(
        out["reasons"].as_array().unwrap().iter().any(|r| r
            .as_str()
            .is_some_and(|r| r.starts_with("fixture parity: mission 622"))),
        "{out}"
    );
}

/// An attestation that makes the row PASS keeps the "not checked" reason.
#[tokio::test]
async fn an_attestation_keeps_a_snapshot_error() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.db_error = Some("lab-mcp HTTP 403 Forbidden".into());
    let human = format!(
        "{FIXTURE_ROW}[[row.expect]]\nid = \"h\"\ntext = \"looks right\"\n\
         source = \"human\"\nquestion = \"Does it look right?\"\n"
    );
    let req = fixture_request(&format!("{CHAIN_ROW}{human}"), FixtureMode::Auto);
    let root = req.root.clone();
    let rows = run_request(&fake, Some(&server), req).await;
    assert_eq!(
        rows[1].result,
        RowResult::NeedsHuman,
        "{:?}",
        rows[1].reasons
    );
    let out = attest(&AttestRequest {
        run_dir: run_dir_under(&root),
        row: Some("FS-B".into()),
        clause: Some("h".into()),
        verdict: Some(Verdict::Pass),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(out["result"], "PASS", "{out}");
    assert_eq!(
        out["reasons"],
        json!(["fixture parity not checked: missions: lab-mcp HTTP 403 Forbidden"]),
        "{out}"
    );
}

/// `RunDir::rows` reads `R.json` and skips `R.snapshot.json` beside it; a
/// row written before `fixture_parity` existed still reads.
#[tokio::test]
async fn run_dir_rows_skips_the_snapshot_and_reads_an_old_row() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let rows = run(
        &fake,
        Some(&server),
        &format!("{CHAIN_ROW}{FIXTURE_ROW}"),
        FixtureMode::Auto,
    )
    .await;
    let tmp = tempfile::tempdir().unwrap();
    let dir = RunDir::create(tmp.path(), "2026-10-11", None, "r1").unwrap();
    dir.write_json(&dir.row_json("s", "R"), &rows[1]).unwrap();
    // Not a row: reading it as one would fail the whole listing.
    dir.write_json(&dir.snapshot_json("s", "R"), &json!({ "schema": 1 }))
        .unwrap();
    let back = dir.rows().unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].fixture_parity, rows[1].fixture_parity);
    let mut old = serde_json::to_value(&rows[1]).unwrap();
    old.as_object_mut().unwrap().remove("fixture_parity");
    let old: RowEvidence = serde_json::from_value(old).unwrap();
    assert!(old.fixture_parity.is_none());
}
