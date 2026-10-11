//! Fixture verify-step tests (lab-fixtures FX-07), part two: what each
//! check compares and how it fails: the session it picks, an item's own
//! container, tutorials, the position on every axis and at the tolerance,
//! value types, and tool errors.

use serde_json::json;

use super::fixture_tests::{client, fx_listed, run, Endpoint, FIXTURE_ROW};
use super::fixture_verify_tests::{blocked_reason, positioned_row};
use super::FixtureMode;
use crate::uat::evidence::RowResult;

const MISSIONS: &str = "missions = [{ id = 622, step = 80623 }]\n";

/// `FIXTURE_ROW` with `extra` added to its fixture after the missions.
fn with_fixture(extra: &str) -> String {
    FIXTURE_ROW.replace(MISSIONS, &format!("{MISSIONS}{extra}\n"))
}

/// Sessions of other players, listed ahead of the fixture character's, in
/// another world. Their names hold neither the lab nor the fixture
/// character, so the session drains and the stage do not see them.
fn others() -> Vec<serde_json::Value> {
    vec![
        json!({ "entity_id": 50, "name": "Uat Otherone", "world": "Castle" }),
        json!({ "entity_id": 51, "name": "Uat Othertwo", "world": "Castle" }),
    ]
}

/// The session is found by the fixture character's name, not by position
/// in the list.
#[tokio::test]
async fn the_session_world_is_the_fixture_characters() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.others = others();
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
}

#[tokio::test]
async fn an_unlisted_fixture_session_blocks_with_null() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.omit_in_world = true;
    server.others = others();
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert!(
        reason.contains(r#"session world: got null, want "Castle_CellBlock""#),
        "{reason}"
    );
}

/// The count carried in total holds, but not in the item's own container.
#[tokio::test]
async fn an_item_outside_its_container_blocks() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies("i.container_id IN", vec![json!({ "rows": [{ "n": 1 }] })]);
    server.db_replies("i.container_id = 2", vec![json!({ "rows": [{ "n": 0 }] })]);
    let row = with_fixture(r#"items = [{ id = 3730, count = 1, container = "mission" }]"#);
    let rows = run(&fake, Some(&server), &row, FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert!(
        reason.contains("item 3730 in container 2: got 0, want 1"),
        "{reason}"
    );
    assert!(!reason.contains("item 3730 count"), "{reason}");
}

#[tokio::test]
async fn a_seen_tutorial_blocks() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(
        "t.tutorial_id = 5882",
        vec![json!({ "rows": [{ "n": 1 }] })],
    );
    let rows = run(
        &fake,
        Some(&server),
        &with_fixture("tutorials_unseen = [5882]"),
        FixtureMode::Auto,
    )
    .await;
    let reason = blocked_reason(&rows[0]);
    assert!(
        reason.contains("tutorial 5882 unseen: got 1, want 0"),
        "{reason}"
    );
}

#[tokio::test]
async fn a_position_off_on_z_only_blocks() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.position = [-319.0, 73.6, -210.0];
    let rows = run(&fake, Some(&server), &positioned_row(), FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert!(reason.contains("position within 2 m: got"), "{reason}");
}

/// Exactly the tolerance away still passes. 73.5 is exact in f32, so the
/// distance is exactly 2.0.
#[tokio::test]
async fn a_position_at_the_tolerance_passes() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.position = [-317.0, 73.5, -212.5];
    let row = positioned_row().replace("73.6", "73.5");
    let rows = run(&fake, Some(&server), &row, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
}

#[tokio::test]
async fn a_string_is_not_a_number() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    server.db_replies(
        "m.mission_id = 622",
        vec![json!({ "rows": [{ "n_rows": 1, "status": 1, "step": "80623" }] })],
    );
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert!(reason.contains(r#"got "80623", want 80623"#), "{reason}");
}

/// No `world` in the fixture: whatever world the session is in passes, and
/// the verify step makes no session-world call. The control run with the
/// world set (and matching) makes exactly one more `server_sessions` call.
#[tokio::test]
async fn no_world_means_no_session_world_check() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.world = "Castle".into();
    let row = FIXTURE_ROW.replace("world = \"Castle_CellBlock\"\n", "");
    let rows = run(&fake, Some(&server), &row, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    let without = server.count("server_sessions");

    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    assert_eq!(server.count("server_sessions"), without + 1);
}

#[tokio::test]
async fn a_failing_query_blocks_with_its_error() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.db_error = Some("relation does not exist".into());
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert!(
        reason.contains(r#"got "error: relation does not exist""#),
        "{reason}"
    );
}

#[tokio::test]
async fn an_entity_the_server_does_not_have_blocks() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.no_entity = true;
    let rows = run(&fake, Some(&server), &positioned_row(), FixtureMode::Auto).await;
    let reason = blocked_reason(&rows[0]);
    assert!(reason.contains("position within 2 m: got null"), "{reason}");
}
