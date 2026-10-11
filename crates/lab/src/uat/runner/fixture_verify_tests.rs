//! Fixture verify-step tests (lab-fixtures FX-07, D-FX6): the staged state
//! is checked with read-only SQL, the session's world and the entity's
//! position after Play, and a fixture that does not hold BLOCKS the row
//! before its setup or steps run.

use serde_json::json;

use super::fixture_tests::{client, fx_listed, ran, run, Endpoint, FIXTURE_ROW};
use super::fixture_verify::{REASON_CAP, VERIFY_POLL_MS, VERIFY_TIMEOUT_MS};
use super::FixtureMode;
use crate::uat::evidence::{RowEvidence, RowResult};

const PREFIX: &str = "fixture not established: ";

/// The row's one `fixture not established` reason.
fn blocked_reason(row: &RowEvidence) -> &str {
    assert_eq!(row.result, RowResult::Blocked, "{:?}", row.reasons);
    row.reasons
        .iter()
        .find_map(|r| r.strip_prefix(PREFIX))
        .unwrap_or_else(|| panic!("no fixture reason in {:?}", row.reasons))
}

/// Mission 622's verify reply with `step` and status active.
fn on_step(step: i64) -> serde_json::Value {
    json!({ "row_count": 1, "rows": [{ "n_rows": 1, "status": 1, "step": step }] })
}

#[tokio::test]
async fn a_mission_on_the_wrong_step_blocks_with_got_and_want() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies("m.mission_id = 622", vec![on_step(2113)]);
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    let reason = blocked_reason(row);
    assert!(
        reason.contains("mission 622 on step 80623: got 2113, want 80623"),
        "{reason}"
    );
    // Only the failing check is named: status 1 holds.
    assert!(!reason.contains("active"), "{reason}");
    assert!(!ran(row, ".cleareffects") && !ran(row, ".help"));
    // A state that never holds is tried a bounded number of times even
    // with the pause skipped: two checks per attempt.
    let attempts = (VERIFY_TIMEOUT_MS / VERIFY_POLL_MS) as usize;
    assert_eq!(server.count("server_db_query"), 2 * attempts);
    // The verify calls are fixture plumbing on the row's record.
    let verify: Vec<_> = row
        .actions
        .iter()
        .filter(|a| a.tool.as_deref() == Some("server_db_query"))
        .collect();
    assert_eq!(verify.len(), 2 * attempts);
    assert!(verify.iter().all(|a| a.kind == "fixture"), "{verify:?}");
}

#[tokio::test]
async fn verify_retries_until_the_state_matches() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    // Each attempt runs the 622 query twice (step, then status): the base
    // applies the step on the third attempt.
    server.db_replies(
        "m.mission_id = 622",
        vec![
            on_step(2113),
            on_step(2113),
            on_step(2113),
            on_step(2113),
            on_step(80623),
        ],
    );
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert_eq!(server.count("server_db_query"), 6, "{:?}", server.names());
    assert!(ran(row, ".help"));
}

#[tokio::test]
async fn numbers_compare_by_value() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(
        "m.mission_id = 622",
        vec![json!({ "rows": [{ "n_rows": 1, "status": 1.0, "step": 80623.0 }] })],
    );
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
}

#[tokio::test]
async fn a_wrong_world_blocks() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.world = "Castle".into();
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    let reason = blocked_reason(row);
    assert!(
        reason.contains(r#"session world: got "Castle", want "Castle_CellBlock""#),
        "{reason}"
    );
    assert!(!ran(row, ".help"));
}

/// `FIXTURE_ROW` with a `position` in its fixture.
fn positioned_row() -> String {
    FIXTURE_ROW.replace(
        "world = \"Castle_CellBlock\"\n",
        "world = \"Castle_CellBlock\"\nposition = [-319.0, 73.6, -212.5]\n",
    )
}

#[tokio::test]
async fn a_position_outside_tolerance_blocks() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    // 2.5 m off on x: outside the 2 m tolerance.
    server.position = [-316.5, 73.6, -212.5];
    let rows = run(&fake, Some(&server), &positioned_row(), FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert!(
        reason.contains("position within 2 m: got [-316.5"),
        "{reason}"
    );
    // The entity checked is the row's own player entity.
    let calls = server.calls.lock().unwrap();
    let (_, args) = calls
        .iter()
        .find(|(n, _)| n == "server_entity_get")
        .expect("server_entity_get called");
    assert_eq!(args["entity_id"], 42);
}

#[tokio::test]
async fn a_position_inside_tolerance_passes() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    // 1.5 m off: within tolerance.
    server.position = [-317.5, 73.6, -212.5];
    let rows = run(&fake, Some(&server), &positioned_row(), FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    assert!(server.count("server_entity_get") >= 1);
}

#[tokio::test]
async fn the_blocked_reason_is_capped() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    // Twelve active missions the server never staged: every one fails two
    // checks, far more than the cap holds.
    let missions: Vec<String> = (700..712)
        .map(|id| format!("{{ id = {id}, step = {} }}", id * 100))
        .collect();
    let row = FIXTURE_ROW.replace(
        "missions = [{ id = 622, step = 80623 }]",
        &format!("missions = [{}]", missions.join(", ")),
    );
    let rows = run(&fake, Some(&server), &row, FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert_eq!(reason.chars().count(), REASON_CAP, "{reason}");
    assert!(reason.ends_with("..."), "{reason}");
    assert!(
        reason.starts_with("mission 700 on step 70000: got null, want 70000; "),
        "{reason}"
    );
}
