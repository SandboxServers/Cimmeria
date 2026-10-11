//! Fixture parity runner tests (lab-fixtures FX-10), part two: when a
//! mismatch may fail a row (fixture mode `never`), the settle re-reads,
//! and the snapshot's optional parts (the file, the session list).

use std::path::Path;

use super::fixture_tests::{
    client, fixture_request, fx_listed, run, Endpoint, CHAIN_ROW, FIXTURE_ROW, SNAP_MISSIONS,
};
use super::snapshot_tests::{chain_missions, mission_reads, parity_reason};
use super::{FixtureMode, Runner};
use crate::uat::evidence::{RowResult, RunDir};
use crate::uat::invoke::ServerInvoker;

const STEP_MISMATCH: &str = "fixture parity: mission 622: chain has step 2113, fixture wants 80623";

/// `never` on a lone row: nothing says what the client held, so the
/// mismatch is a flag and the row keeps its result.
#[tokio::test]
async fn never_mode_without_a_passing_neighbour_only_flags_a_mismatch() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(SNAP_MISSIONS, vec![chain_missions(2113, &[])]);
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Never).await;
    let row = &rows[0];
    assert_eq!(row.vars["fixture_mode"], "chain");
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert!(parity_reason(row).is_none(), "{:?}", row.reasons);
    assert!(
        row.flags.iter().any(|f| f == STEP_MISMATCH),
        "{:?}",
        row.flags
    );
    let p = row.fixture_parity.as_ref().expect("parity recorded");
    assert!(p.advisory && p.mismatches.len() == 1, "{p:?}");
}

/// `never` after a passing spec neighbour: the chain is the route the
/// fixture describes, so the mismatch fails the row as in `auto`.
#[tokio::test]
async fn never_mode_after_a_passing_neighbour_fails_a_mismatch() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(SNAP_MISSIONS, vec![chain_missions(2113, &[])]);
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Never).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    let b = &rows[1];
    assert_eq!(b.result, RowResult::Fail, "{:?}", b.reasons);
    assert_eq!(parity_reason(b).map(String::as_str), Some(STEP_MISMATCH));
    assert!(!b.fixture_parity.as_ref().expect("parity").advisory);
}

/// The previous row's last write lands between the first read and the
/// second: the row passes on the settled state.
#[tokio::test]
async fn a_mismatch_that_settles_on_a_reread_passes() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(
        SNAP_MISSIONS,
        vec![chain_missions(2113, &[]), chain_missions(80623, &[])],
    );
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    let b = &rows[1];
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    let p = b.fixture_parity.as_ref().expect("parity recorded");
    assert!(p.mismatches.is_empty(), "{p:?}");
    assert!(!b.flags.iter().any(|f| f.starts_with("fixture parity")));
    assert_eq!(mission_reads(&server), 2);
    // Every re-read is on the record as fixture plumbing.
    let queries: Vec<_> = b
        .actions
        .iter()
        .filter(|a| a.requested == "server_db_query")
        .collect();
    assert_eq!(queries.len(), 6);
    assert!(queries.iter().all(|a| a.kind == "fixture"));
}

/// A snapshot file that cannot be written loses only the file.
#[tokio::test]
async fn an_unwritable_snapshot_file_keeps_the_parity() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(SNAP_MISSIONS, vec![chain_missions(2113, &[])]);
    let req = fixture_request(&format!("{CHAIN_ROW}{FIXTURE_ROW}"), FixtureMode::Auto);
    let runner = Runner::new(&fake, Some(&server as &dyn ServerInvoker), req).unwrap();
    // A directory where the file goes makes the write fail.
    std::fs::create_dir_all(runner.run.snapshot_json("gm-parity", "FS-B")).unwrap();
    let out = runner.run_all().await.unwrap();
    let rows = RunDir::open(Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap();
    let b = rows.iter().find(|r| r.row_id == "FS-B").expect("FS-B");
    assert_eq!(b.result, RowResult::Fail, "{:?}", b.reasons);
    assert_eq!(parity_reason(b).map(String::as_str), Some(STEP_MISMATCH));
    let p = b.fixture_parity.as_ref().expect("parity recorded");
    assert!(p.snapshot_error.is_none(), "{p:?}");
    assert!(!b.attachments.iter().any(|a| a.name == "snapshot"));
}

/// The session list only matters when the fixture names a world.
#[tokio::test]
async fn a_failed_session_list_matters_only_when_the_fixture_names_a_world() {
    let no_world = FIXTURE_ROW.replace("world = \"Castle_CellBlock\"\n", "");
    assert_ne!(no_world, FIXTURE_ROW);
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.sessions_error = Some("lab-mcp HTTP 502".into());
    let rows = run(
        &fake,
        Some(&server),
        &format!("{CHAIN_ROW}{no_world}"),
        FixtureMode::Auto,
    )
    .await;
    let b = &rows[1];
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    let p = b.fixture_parity.as_ref().expect("parity recorded");
    assert_eq!(p, &crate::uat::snapshot::Parity::default());
    assert!(b.attachments.iter().any(|a| a.name == "snapshot"));

    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.sessions_error = Some("lab-mcp HTTP 502".into());
    let rows = run(
        &fake,
        Some(&server),
        &format!("{CHAIN_ROW}{FIXTURE_ROW}"),
        FixtureMode::Auto,
    )
    .await;
    let b = &rows[1];
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    assert_eq!(
        b.fixture_parity
            .as_ref()
            .and_then(|p| p.snapshot_error.clone()),
        Some("sessions: lab-mcp HTTP 502".to_string())
    );
    assert_eq!(
        b.reasons,
        vec!["fixture parity not checked: sessions: lab-mcp HTTP 502".to_string()]
    );
}
