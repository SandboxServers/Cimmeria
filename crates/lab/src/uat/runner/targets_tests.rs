//! SV-07 target tests over the fake client and a fake `cimmeria-lab-mcp`:
//! a `pose` or `tag` argument resolves, in the lab player's own space, to
//! the one entity carrying the tag (filtered again when the server ignores
//! the tag), zero or several matches fail naming the tag and the space, the
//! client tool never sees a runner argument, and `@stand` types the
//! teleport, then polls until the server position arrives or gives up.

use serde_json::{json, Value};

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{ActionRecord, RowEvidence, RowResult, RunDir};
use crate::uat::invoke::ServerInvoker;

/// The Frost body's live server position (README F20).
const FROST: [f64; 3] = [-328.30, 73.472, -210.27];
const FROST_TAG: &str = "ArmYourself_FrostBody";
/// The player's entity id the fake client reports.
const PLAYER: u32 = 8;
/// The player's space; another space holds a twin of every tagged body.
const SPACE: u32 = 3;

fn body(id: u32, tag: Option<&str>) -> Value {
    json!({ "entity_id": id, "space_id": SPACE, "tag": tag, "position": FROST })
}

/// `client_entity_find { include_player }`: the player at `at`.
fn player_at(at: [f64; 3]) -> Value {
    json!({ "player": { "id": PLAYER, "position": {
        "server": { "x": at[0], "y": at[1], "z": at[2] },
    }}})
}

/// A server whose player 8 is in space 3, and whose query answers `entities`.
fn server(entities: Vec<Value>) -> FakeServer {
    let n = entities.len();
    FakeServer::new(json!({}))
        .answer(
            "server_entity_get",
            vec![json!({ "entity": { "entity_id": PLAYER, "space_id": SPACE } })],
        )
        .answer(
            "server_entity_query",
            vec![
                json!({ "entities": entities, "returned": n, "total_matched": n, "capped": false }),
            ],
        )
}

fn client(finds: Vec<Value>) -> Fake {
    Fake::new(&[])
        .with("client_entity_find")
        .with("client_world_click")
        .with("client_camera")
        .answer("client_entity_find", finds)
}

const FROST_POSE: &str = r#"pose = [
  { id = "frost", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0 },
  { id = "torso", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, aim = "point", aim_dy_m = 0.28 },
]"#;

/// One row with the two poses and `step`, graded by a chat line the fake
/// always shows.
fn row(step: &str) -> String {
    format!(
        r#"
[[row]]
id = "SV-T"
title = "targets"
expected = "Reaches the tagged body."
{FROST_POSE}
step = [{step}]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "Welcome"
"#
    )
}

async fn run(fake: &Fake, server: Option<&FakeServer>, step: &str) -> RowEvidence {
    let tmp = tempfile::tempdir().unwrap().keep();
    let server = server.map(|s| s as &dyn ServerInvoker);
    let out = Runner::new(fake, server, request(&tmp, &row(step)))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0)
}

fn step_action(row: &RowEvidence) -> &ActionRecord {
    row.actions
        .iter()
        .find(|a| a.label.as_deref() == Some("go"))
        .expect("the labelled step ran")
}

/// The arguments the fake client got for `tool` (its last call).
fn sent(fake: &Fake, tool: &str) -> Option<Value> {
    fake.log
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(n, _)| n == tool)
        .map(|(_, a)| a.clone())
}

#[tokio::test]
async fn a_pose_click_reaches_the_tagged_entity_in_the_players_space() {
    let fake = client(vec![player_at([0.0; 3])]);
    let srv = server(vec![
        body(100_751, Some(FROST_TAG)),
        body(100_752, Some("ArmYourself_GuardBody")),
    ]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@world_click", args = { pose = "frost", button = "right" }, label = "go" }"#,
    )
    .await;
    let act = step_action(&ev);
    assert!(act.ok, "{:?}", act.error);

    // The space came from the player's own entity, and scoped the query.
    let calls = srv.calls.lock().unwrap().clone();
    assert_eq!(
        calls[0],
        ("server_entity_get".into(), json!({ "entity_id": PLAYER }))
    );
    assert_eq!(
        calls[1],
        (
            "server_entity_query".into(),
            json!({ "space_id": SPACE, "tag": FROST_TAG })
        )
    );

    let args = sent(&fake, "client_world_click").expect("the click ran");
    assert_eq!(args["entity_id"], 100_751);
    assert_eq!(args["rotate_camera"], false);
    assert_eq!(args["button"], "right");
    for k in ["pose", "tag", "face_tag", "aim", "aim_dy_m"] {
        assert!(args.get(k).is_none(), "{k} reached the client: {args}");
    }
    // The evidence shows what the tool got, and the resolution for lab-golden.
    assert_eq!(act.args, args);
    assert!(act.calls.contains(&json!({ "resolve": {
        "tag": FROST_TAG, "entity_id": 100_751, "space_id": SPACE, "pose": "frost",
    }})));
}

#[tokio::test]
async fn an_old_server_that_ignores_tag_is_filtered() {
    // A server older than SV-01 answers with the whole space.
    let fake = client(vec![player_at([0.0; 3])]);
    let srv = server(vec![
        body(100_750, None),
        body(100_752, Some("ArmYourself_GuardBody")),
        body(100_751, Some(FROST_TAG)),
    ]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@world_click", args = { tag = "ArmYourself_FrostBody" }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_world_click").unwrap();
    assert_eq!(args["entity_id"], 100_751);
    // A bare tag set no camera: the tool's own rotate default stays.
    assert!(args.get("rotate_camera").is_none(), "{args}");
    assert!(args.get("tag").is_none(), "{args}");
}

#[tokio::test]
async fn zero_or_two_matches_fail_naming_the_tag() {
    let step = r#"{ tool = "@world_click", args = { pose = "frost" }, label = "go" }"#;
    for (entities, want) in [
        (
            vec![body(100_752, Some("ArmYourself_GuardBody"))],
            format!("no entity with tag {FROST_TAG} in space {SPACE}"),
        ),
        (
            vec![
                body(100_751, Some(FROST_TAG)),
                body(100_753, Some(FROST_TAG)),
            ],
            format!("tag {FROST_TAG} matches entities 100751, 100753 in space {SPACE}"),
        ),
    ] {
        let fake = client(vec![player_at([0.0; 3])]);
        let ev = run(&fake, Some(&server(entities)), step).await;
        assert_eq!(ev.result, RowResult::Fail, "{:?}", ev.reasons);
        let act = step_action(&ev);
        assert!(!act.ok);
        assert_eq!(act.error.as_deref(), Some(want.as_str()));
        assert!(!fake.names().iter().any(|n| n == "client_world_click"));
    }
}

#[tokio::test]
async fn a_point_pose_clicks_above_the_origin() {
    let fake = client(vec![player_at([0.0; 3])]);
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@world_click", args = { pose = "torso" }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_world_click").unwrap();
    let y = args["point"]["y"].as_f64().unwrap();
    assert!((y - (FROST[1] + 0.28)).abs() < 1e-6, "{args}");
    assert!((args["point"]["x"].as_f64().unwrap() - FROST[0]).abs() < 1e-6);
    assert!(args.get("entity_id").is_none(), "{args}");
}

#[tokio::test]
async fn a_camera_pose_faces_the_entity_and_explicit_args_win() {
    // The pose's target replaces an explicit face_name (one target only);
    // an explicit pitch beats the pose's, the rest come from the pose.
    let fake = client(vec![player_at([0.0; 3])]);
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@camera", args = { pose = "frost", face_name = "Frost", pitch_deg = -10.0 }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_camera").unwrap();
    assert_eq!(args["face_entity_id"], 100_751);
    assert!(args.get("face_name").is_none(), "{args}");
    assert_eq!(args["pitch_deg"], -10.0);
    assert_eq!(args["yaw_offset_deg"], 25.3);
    assert_eq!(args["zoom"], 250.0);
}

/// Where the frost pose stands (README F20): `/gmgotoxyz -325.00 73.60 -212.80`.
const STAND_AT: [f64; 3] = [-325.0, 73.6, -212.8];

#[tokio::test(start_paused = true)]
async fn stand_types_the_teleport_and_waits_for_arrival() {
    let far = player_at([0.0, 73.6, 0.0]);
    let near = player_at([STAND_AT[0] + 0.2, 73.6, STAND_AT[2] - 0.2]);
    let step = r#"{ tool = "@stand", args = { pose = "frost" }, label = "go" }"#;

    // The first find names the player; then two polls far away, then there.
    let fake = client(vec![far.clone(), far.clone(), far.clone(), near]);
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let ev = run(&fake, Some(&srv), step).await;
    let act = step_action(&ev);
    assert!(act.ok, "{:?}", act.error);
    let typed = sent(&fake, "client_type_text").unwrap();
    assert_eq!(typed["text"], "/gmgotoxyz -325.00 73.60 -212.80");
    let finds = fake
        .names()
        .iter()
        .filter(|n| *n == "client_entity_find")
        .count();
    assert_eq!(finds, 4, "the player lookup and three polls");
    // Only the last poll is kept: the lookup's entry and one poll.
    let recorded = act
        .calls
        .iter()
        .filter(|c| c["tool"] == "client_entity_find")
        .count();
    assert_eq!(recorded, 2, "{:?}", act.calls);

    // Never arriving gives up after the timeout and says how far it is.
    let fake = client(vec![far]);
    let ev = run(&fake, Some(&srv), step).await;
    let act = step_action(&ev);
    assert!(!act.ok);
    let err = act.error.as_deref().unwrap_or_default();
    assert!(
        err.starts_with("stand: still ")
            && err.ends_with(" m from (-325.00, 73.60, -212.80) after 5 s"),
        "{err}"
    );
}

#[tokio::test]
async fn no_server_blocks_a_tag_row() {
    let fake = client(vec![player_at([0.0; 3])]);
    let ev = run(
        &fake,
        None,
        r#"{ tool = "@world_click", args = { pose = "frost" }, label = "go" }"#,
    )
    .await;
    assert_eq!(ev.result, RowResult::Blocked, "{:?}", ev.reasons);
    assert!(
        ev.reasons
            .iter()
            .any(|r| r.contains("client_world_click by tag needs the server lab MCP")),
        "{:?}",
        ev.reasons
    );
    assert!(!fake.names().iter().any(|n| n == "client_world_click"));
}
