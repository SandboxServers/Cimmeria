//! Fixture-row runner tests (lab-fixtures FX-06), part two: the session a
//! fixture row runs in. It ends any earlier session (and drains it),
//! stages at character select, plays once and ends its own session; a new
//! fixture character gets one bootstrap session; a session that does not
//! end cleanly is flagged and drained by the next one. The fake endpoint
//! and helpers are in `fixture_tests.rs`.

use serde_json::json;

use super::fixture::{FIRST_LOGIN_HOLD_LABEL, FIXTURE_KIND, SESSION_DRAIN_MS, SESSION_POLL_MS};
use super::fixture_tests::{
    client, fixture_row_c, fx_listed, in_order, listing, ran, run, tools, Endpoint, CHAIN_ROW,
    FIXTURE_ROW, FX,
};
use super::FixtureMode;
use crate::uat::evidence::{RowEvidence, RowResult};

/// The polls one drain makes before it gives up.
const POLLS: usize = (SESSION_DRAIN_MS / SESSION_POLL_MS) as usize;

/// The drain-timeout text for `name`.
fn still_lists(name: &str) -> String {
    format!(
        "server still lists {name} after {} s",
        SESSION_DRAIN_MS / 1000
    )
}

fn has_flag(row: &RowEvidence, want: &str) -> bool {
    row.flags.iter().any(|f| f == want)
}

#[tokio::test]
async fn a_fixture_row_runs_in_its_own_session_in_order() {
    let full = json!({ "characters": [{ "name": format!("Uat {FX}"),
        "archetype": "soldier", "level": 3, "alignment": "good" }] });
    let fake = client(vec![full]);
    let server = Endpoint::new(&fake);
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    let seen = tools(row);
    // The run found the client in world as the lab character: that session
    // is stopped and drained (`server_sessions`) before the next start.
    assert!(
        in_order(
            &seen,
            &[
                "lab_client_stop",
                "server_sessions",
                "lab_client_start",
                "lab_login",
                "lab_characters",
                "server_stage_fixture",
                "lab_play_character",
                "lab_logout",
                "lab_client_stop",
                "server_sessions",
            ]
        ),
        "{seen:?}"
    );
    // Staged for the fixture character, and the entity resolved after Play.
    let calls = server.calls.lock().unwrap().clone();
    let stage = calls
        .iter()
        .find(|c| c.0 == "server_stage_fixture")
        .unwrap();
    assert_eq!(stage.1["character"], FX);
    assert_eq!(stage.1["missions"][0]["step"], 80623);
    let vars = &row.vars;
    assert_eq!(vars["fixture_mode"], "fixture");
    assert_eq!(vars["fixture_character"], FX);
    assert_eq!(vars["character"], FX, "${{character}} is the one in world");
    assert_eq!(vars["player_entity_id"], 42);
    assert_eq!(row.character["name"], FX);
    assert_eq!(row.character["archetype"], "soldier");
    assert_eq!(row.character["level"], 3);
    // Plumbing is marked for lab-golden; the row's own actions are not.
    const PLUMBING: [&str; 9] = [
        "lab_client_status",
        "lab_client_stop",
        "lab_client_start",
        "lab_login",
        "lab_logout",
        "lab_characters",
        "lab_play_character",
        "server_stage_fixture",
        "server_sessions",
    ];
    for a in &row.actions {
        if a.tool.as_deref().is_some_and(|t| PLUMBING.contains(&t)) {
            assert_eq!(a.kind, FIXTURE_KIND, "{} {:?}", a.index, a.tool);
        }
    }
    assert!(row
        .actions
        .iter()
        .any(|a| a.requested == ".help" && a.kind == "chat"));
    // The session ended with the row: the client is stopped.
    assert!(!*fake.running.lock().unwrap());
    assert!(row.flags.is_empty(), "{:?}", row.flags);
}

#[tokio::test]
async fn a_new_fixture_character_gets_one_bootstrap_session() {
    let fake = client(vec![listing(&[]), listing(&[&format!("Uat {FX}")])]);
    let server = Endpoint::new(&fake);
    // A second profile, whose character the bootstrap must not delete.
    let castle = "[section.profiles.castle]\nalignment = \"good\"\n\
                  archetype = \"scientist\"\ngender = \"female\"\n";
    let rows = format!("{castle}{FIXTURE_ROW}{}", fixture_row_c());
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    for r in &rows {
        assert_eq!(r.result, RowResult::Pass, "{} {:?}", r.row_id, r.reasons);
    }
    let names = fake.names();
    let creates = names
        .iter()
        .filter(|n| *n == "lab_create_character")
        .count();
    assert_eq!(creates, 1, "{names:?}");
    let log = fake.log.lock().unwrap().clone();
    let arg = |tool: &str| log.iter().find(|(t, _)| t == tool).unwrap().1.clone();
    let create = arg("lab_create_character");
    assert_eq!(create["last"], FX);
    assert_eq!(create["archetype"], "soldier");
    // Protects the lab character and every profile's fixture character.
    assert_eq!(
        arg("lab_ensure_character_slot")["protect"],
        json!(["Labone", "Fxcastledefa", FX])
    );
    let holds: Vec<_> = rows
        .iter()
        .flat_map(|r| r.actions.iter())
        .filter(|a| a.label.as_deref() == Some(FIRST_LOGIN_HOLD_LABEL))
        .collect();
    assert_eq!(holds.len(), 1);
    assert_eq!(holds[0].requested, "wait 17000 ms");
    assert_eq!(holds[0].kind, FIXTURE_KIND);
    assert_eq!(holds[0].result["skipped"], true, "no_settle skips the wait");
    // The bootstrap belongs to the first row and ended before the stage.
    let first = tools(&rows[0]);
    assert!(
        in_order(
            &first,
            &[
                "lab_create_character",
                "lab_play_character",
                "lab_client_stop",
                "lab_client_start",
                "server_stage_fixture",
                "lab_play_character",
            ]
        ),
        "{first:?}"
    );
    assert_eq!(server.count("server_stage_fixture"), 2);
}

/// The bootstrap session is drained like any other: a character the
/// server keeps listing blocks the row before anything is staged.
#[tokio::test]
async fn the_bootstrap_session_is_drained() {
    let fake = client(vec![listing(&[]), listing(&[&format!("Uat {FX}")])]);
    let mut server = Endpoint::new(&fake);
    server.stuck = Some(format!("Uat {FX}"));
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Blocked);
    let want = format!("fixture not established: {}", still_lists(FX));
    assert!(row.reasons.contains(&want), "{:?}", row.reasons);
    assert_eq!(server.count("server_stage_fixture"), 0);
}

#[tokio::test]
async fn a_refused_stage_blocks_the_row_and_runs_no_setup() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.refuse = Some(format!("stage refused: character {FX} is online"));
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Blocked);
    let want = format!("fixture not established: stage: stage refused: character {FX} is online");
    assert!(row.reasons.iter().any(|r| r == &want), "{:?}", row.reasons);
    assert!(!ran(row, ".cleareffects") && !ran(row, ".help"));
    assert!(row.actions.iter().all(|a| a.kind == FIXTURE_KIND));
    // Never played; the session was ended after the refusal.
    let seen = tools(row);
    assert!(!seen.contains(&"lab_play_character"), "{seen:?}");
    assert!(
        in_order(&seen, &["server_stage_fixture", "lab_client_stop"]),
        "{seen:?}"
    );
    assert!(!*fake.running.lock().unwrap());
}

#[tokio::test]
async fn the_session_drain_times_out_with_the_name() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.stuck = Some("Uat Labone".into());
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Blocked);
    let want = format!("fixture not established: {}", still_lists("Labone"));
    assert!(row.reasons.contains(&want), "{:?}", row.reasons);
    // Polled for the whole window, then gave up before logging in again;
    // the row's own end_session drained the leftover once more, and says
    // so.
    assert_eq!(server.count("server_sessions"), 2 * POLLS);
    assert!(!fake.names().contains(&"lab_login".to_string()));
    let flag = format!("session end: {}", still_lists("Labone"));
    assert!(has_flag(row, &flag), "{:?}", row.flags);
}

/// Play reached the world and then errored: the session that is ended is
/// the fixture character's, not the lab-character fallback.
#[tokio::test]
async fn a_play_that_errors_in_world_drains_the_fixture_character() {
    let fake = client(fx_listed()).fail_after("lab_play_character", "no HUD after 60 s");
    let mut server = Endpoint::new(&fake);
    server.stuck = Some(format!("Uat {FX}"));
    let rows = run(&fake, Some(&server), FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Blocked);
    let want = "fixture not established: lab_play_character: no HUD after 60 s";
    assert!(row.reasons.iter().any(|r| r == want), "{:?}", row.reasons);
    let flag = format!("session end: {}", still_lists(FX));
    assert!(has_flag(row, &flag), "{:?}", row.flags);
    assert!(!*fake.running.lock().unwrap());
}

/// A drain that times out at the end of a passing row leaves the row
/// passing with a flag, and the next fixture row drains that character
/// before it logs in.
#[tokio::test]
async fn a_failed_drain_is_flagged_and_drained_by_the_next_row() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.stuck = Some(format!("Uat {FX}"));
    let rows = format!("{FIXTURE_ROW}{}", fixture_row_c());
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    let (b, c) = (&rows[0], &rows[1]);
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    let flag = format!("session end: {}", still_lists(FX));
    assert!(has_flag(b, &flag), "{:?}", b.flags);
    assert_eq!(c.result, RowResult::Blocked);
    let want = format!("fixture not established: {}", still_lists(FX));
    assert!(c.reasons.contains(&want), "{:?}", c.reasons);
    assert!(!tools(c).contains(&"lab_login"), "{:?}", tools(c));
}

/// A failure after Play (no session to resolve the entity from) still
/// ends the session it started. The row has no `world`, so the verify step
/// right after Play has no session to check and the entity lookup fails.
#[tokio::test]
async fn a_failure_after_play_ends_the_session() {
    let fake = client(fx_listed());
    let mut server = Endpoint::new(&fake);
    server.omit_in_world = true;
    let row = FIXTURE_ROW.replace("world = \"Castle_CellBlock\"\n", "");
    let rows = run(&fake, Some(&server), &row, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Blocked);
    let want = format!(
        "fixture not established: player entity: no in-world session for character \
         \"{FX}\" in server_sessions"
    );
    assert!(row.reasons.contains(&want), "{:?}", row.reasons);
    let seen = tools(row);
    assert!(
        in_order(
            &seen,
            &[
                "lab_play_character",
                "lab_logout",
                "lab_client_stop",
                "server_sessions",
            ]
        ),
        "{seen:?}"
    );
    assert!(!*fake.running.lock().unwrap());
}

/// A failed setup BLOCKs the row and the session still ends with it.
#[tokio::test]
async fn a_failed_setup_still_ends_the_fixture_session() {
    let fake = client(fx_listed()).with("lab_fail");
    let server = Endpoint::new(&fake);
    let row = FIXTURE_ROW.replace(
        "setup = [{ chat = \".cleareffects\" }]",
        "setup = [{ tool = \"lab_fail\", tier = \"N1\" }]",
    );
    let rows = run(&fake, Some(&server), &row, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Blocked, "{:?}", row.reasons);
    assert_eq!(row.vars["fixture_mode"], "fixture");
    assert!(!ran(row, ".help"));
    let seen = tools(row);
    assert!(
        in_order(
            &seen,
            &["lab_play_character", "lab_fail", "lab_client_stop"]
        ),
        "{seen:?}"
    );
    assert!(!*fake.running.lock().unwrap());
}

/// A chain row then a fixture row under `always`: the chain row's session
/// is drained by name before the fixture session starts.
#[tokio::test]
async fn always_mode_drains_the_chain_rows_session_first() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Always).await;
    let b = tools(&rows[1]);
    assert!(
        in_order(
            &b,
            &["lab_client_stop", "server_sessions", "lab_client_start"]
        ),
        "{b:?}"
    );
}
