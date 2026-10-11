//! SV-07 target tests over the fake client and a fake `cimmeria-lab-mcp`:
//! a `pose` or `tag` argument resolves, in the lab player's own space, to
//! the one entity carrying the tag (filtered again when the server ignores
//! the tag), zero or several matches fail naming the tag and the space, the
//! client tool never sees a runner argument, and `@stand` types the
//! teleport, then polls until the server position arrives or gives up.

use serde_json::{json, Value};

use super::targets::{STAND_POLL, STAND_TIMEOUT};
use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::{HealHint, RunOutcome, Runner};
use crate::uat::evidence::{ActionRecord, RowEvidence, RowResult, RunDir};
use crate::uat::invoke::ServerInvoker;
use crate::uat::tier::Role;

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

/// The frost click as a step: the pose resolved and the click typed.
const CLICK_FROST: &str = r#"{ tool = "@world_click", args = { pose = "frost" }, label = "go" }"#;

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

/// The run's summaries for the one row `text` holds.
async fn outcome(fake: &Fake, server: Option<&FakeServer>, text: &str) -> RunOutcome {
    let tmp = tempfile::tempdir().unwrap().keep();
    let server = server.map(|s| s as &dyn ServerInvoker);
    Runner::new(fake, server, request(&tmp, text))
        .unwrap()
        .run_all()
        .await
        .unwrap()
}

fn step_action(row: &RowEvidence) -> &ActionRecord {
    row.actions
        .iter()
        .find(|a| a.label.as_deref() == Some("go"))
        .expect("the labelled step ran")
}

/// Whether the fake client typed a `/gmgotoxyz` (setup types its own lines).
fn teleported(fake: &Fake) -> bool {
    fake.log.lock().unwrap().iter().any(|(n, a)| {
        n == "client_type_text"
            && a["text"]
                .as_str()
                .is_some_and(|t| t.starts_with("/gmgotoxyz"))
    })
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
    // The resolve reads live in the step's `calls`, not in records of their own.
    let steps = ev.actions.iter().filter(|a| a.role == Role::Step).count();
    assert_eq!(steps, 1, "{:?}", ev.actions);
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
        // The same tag in another space is not the player's.
        json!({ "entity_id": 100_760, "space_id": 4, "tag": FROST_TAG, "position": FROST }),
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
    // An explicit pitch beats the pose's, the rest come from the pose.
    let fake = client(vec![player_at([0.0; 3])]);
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@camera", args = { pose = "frost", pitch_deg = -10.0 }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_camera").unwrap();
    assert_eq!(args["face_entity_id"], 100_751);
    assert_eq!(args["pitch_deg"], -10.0);
    assert_eq!(args["yaw_offset_deg"], 25.3);
    assert_eq!(args["zoom"], 250.0);

    // `zoom_to` is the same setting: the pose's zoom stays out.
    let fake = client(vec![player_at([0.0; 3])]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@camera", args = { pose = "frost", zoom_to = 100 }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_camera").unwrap();
    assert_eq!(args["zoom_to"], 100);
    assert!(args.get("zoom").is_none(), "{args}");
}

#[test]
fn a_pose_next_to_the_target_it_replaces_is_refused() {
    // The validator refuses it in a spec; the runner again for built args.
    let m = |v: Value| v.as_object().unwrap().clone();
    let clash = super::targets::refuse_pose_clash;
    assert_eq!(
        clash(
            "client_world_click",
            &m(json!({ "pose": "frost", "entity_id": 5 }))
        ),
        Err("pose with entity_id: give one".into())
    );
    assert_eq!(
        clash(
            "client_camera",
            &m(json!({ "pose": "frost", "face_point": [0, 0, 0] }))
        ),
        Err("pose with face_point: give one".into())
    );
    assert_eq!(
        clash(
            "client_world_click",
            &m(json!({ "pose": "frost", "rotate_camera": true }))
        ),
        Ok(())
    );
}

#[tokio::test]
async fn explicit_args_next_to_a_click_pose_win() {
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    // An explicit rotate_camera survives the pose's `false`.
    let fake = client(vec![player_at([0.0; 3])]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@world_click", args = { pose = "frost", rotate_camera = true }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_world_click").unwrap();
    assert_eq!(args["rotate_camera"], true);
    assert_eq!(args["entity_id"], 100_751);

    // An explicit point aim turns an entity pose into a point click.
    let fake = client(vec![player_at([0.0; 3])]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@world_click", args = { pose = "frost", aim = "point", aim_dy_m = 0.5 }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_world_click").unwrap();
    let y = args["point"]["y"].as_f64().unwrap();
    assert!((y - (FROST[1] + 0.5)).abs() < 1e-6, "{args}");
    assert!(args.get("entity_id").is_none(), "{args}");

    // aim_dy_m with an entity aim says nothing: refused, nothing clicked.
    let fake = client(vec![player_at([0.0; 3])]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@world_click", args = { pose = "frost", aim_dy_m = 0.5 }, label = "go" }"#,
    )
    .await;
    let act = step_action(&ev);
    assert_eq!(act.error.as_deref(), Some("aim_dy_m needs aim = \"point\""));
    assert!(!fake.names().iter().any(|n| n == "client_world_click"));
}

#[tokio::test]
async fn target_and_entity_find_take_the_resolved_entity() {
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let fake = client(vec![player_at([0.0; 3])]).with("client_target");
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@target", args = { pose = "frost" }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_target").unwrap();
    assert_eq!(args["entity_id"], 100_751);
    assert_eq!(args["rotate_camera"], false);

    let fake = client(vec![player_at([0.0; 3])]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@entity_find", args = { tag = "ArmYourself_FrostBody" }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let args = sent(&fake, "client_entity_find").unwrap();
    assert_eq!(args["entity_id"], 100_751);
    assert!(args.get("tag").is_none(), "{args}");
}

#[tokio::test]
async fn a_capped_reply_without_the_tag_says_so() {
    let fake = client(vec![player_at([0.0; 3])]);
    let srv = FakeServer::new(json!({}))
        .answer(
            "server_entity_get",
            vec![json!({ "entity": { "entity_id": PLAYER, "space_id": SPACE } })],
        )
        .answer(
            "server_entity_query",
            vec![json!({
                "entities": [body(100_750, None)],
                "returned": 1, "total_matched": 900, "capped": true,
            })],
        );
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@world_click", args = { pose = "frost" }, label = "go" }"#,
    )
    .await;
    let want = format!("no entity with tag {FROST_TAG} in space {SPACE} (the reply was capped)");
    assert_eq!(step_action(&ev).error.as_deref(), Some(want.as_str()));
}

#[tokio::test]
async fn a_pose_click_miss_puts_the_heal_hint_in_the_summary() {
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let fake = client(vec![player_at([0.0; 3])]).answer(
        "client_world_click",
        vec![json!({ "__error": "pose missed", "__data": { "step": "hover" } })],
    );
    let out = outcome(&fake, Some(&srv), &row(CLICK_FROST)).await;
    let summary = &out.rows[0];
    assert_ne!(summary.result, "PASS", "{:?}", summary.reasons);
    assert_eq!(
        summary.heal,
        Some(HealHint {
            pose: "frost".into(),
            step: "hover".into(),
        })
    );
    let json = serde_json::to_value(summary).unwrap();
    assert_eq!(json["heal"], json!({ "pose": "frost", "step": "hover" }));
}

#[tokio::test]
async fn a_passing_row_summary_has_no_heal_key() {
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let fake = client(vec![player_at([0.0; 3])]);
    // A lua clause the fake answers (its `results` hold "40"), since the
    // chat clause of `row` needs a line this fake never shows.
    let text = format!(
        "[[row]]\nid = \"SV-T\"\ntitle = \"targets\"\nexpected = \"Reaches the tagged body.\"\n{FROST_POSE}\nstep = [{CLICK_FROST}]\n\
         [[row.expect]]\nid = \"c\"\ntext = \"t\"\nsource = \"lua\"\nchunk = \"return 40\"\nop = \"gte\"\nvalue = 1\n"
    );
    let out = outcome(&fake, Some(&srv), &text).await;
    let summary = &out.rows[0];
    assert_eq!(summary.result, "PASS", "{:?}", summary.reasons);
    assert!(summary.heal.is_none());
    let json = serde_json::to_value(summary).unwrap();
    assert!(json.get("heal").is_none(), "{json}");

    // The same click optional and missing: the evidence marks it optional,
    // so it gives no hint (the grader still fails the row on it).
    let text = text.replace(r#"label = "go" }"#, r#"label = "go", optional = true }"#);
    let fake = client(vec![player_at([0.0; 3])]).answer(
        "client_world_click",
        vec![json!({ "__error": "pose missed", "__data": { "step": "hover" } })],
    );
    let out = outcome(&fake, Some(&srv), &text).await;
    let summary = &out.rows[0];
    assert_ne!(summary.result, "PASS", "{:?}", summary.reasons);
    let json = serde_json::to_value(summary).unwrap();
    assert!(json.get("heal").is_none(), "{json}");
    let ev = RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0);
    let act = step_action(&ev);
    assert!(!act.ok && act.optional, "{act:?}");
}

/// Where the frost pose stands (README F20): `/gmgotoxyz -325.00 73.60 -212.80`.
const STAND_AT: [f64; 3] = [-325.0, 73.6, -212.8];

#[tokio::test(start_paused = true)]
async fn stand_types_the_teleport_and_waits_for_arrival() {
    let far = player_at([0.0, 73.6, 0.0]);
    // Arrival is horizontal: 2 m off in height, 0.28 m off across, is there.
    let near = player_at([STAND_AT[0] + 0.2, STAND_AT[1] + 2.0, STAND_AT[2] - 0.2]);
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
    // Each poll asks for the player by id: no scan of nearby entities.
    let poll = sent(&fake, "client_entity_find").unwrap();
    assert_eq!(poll["entity_id"], PLAYER, "{poll}");

    // Never arriving gives up at the deadline and says how far it is.
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
    // The lookup, then one poll per STAND_POLL until STAND_TIMEOUT, no more.
    let polls = (STAND_TIMEOUT.as_millis() / STAND_POLL.as_millis()) as usize;
    let finds = fake
        .names()
        .iter()
        .filter(|n| *n == "client_entity_find")
        .count();
    assert_eq!(finds, 1 + polls);

    // 0.6 m off horizontally is not there, whatever the height.
    let off = player_at([STAND_AT[0] + 0.6, STAND_AT[1], STAND_AT[2]]);
    let fake = client(vec![off]);
    let ev = run(&fake, Some(&srv), step).await;
    let err = step_action(&ev).error.clone().unwrap_or_default();
    assert_eq!(
        err, "stand: still 0.60 m from (-325.00, 73.60, -212.80) after 5 s",
        "{err}"
    );
}

#[tokio::test(start_paused = true)]
async fn stand_by_tag_or_point_types_the_right_teleport() {
    let srv = server(vec![body(100_751, Some(FROST_TAG))]);
    let there = player_at(STAND_AT);
    let fake = client(vec![there]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@stand", args = { tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13 }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let typed = sent(&fake, "client_type_text").unwrap();
    assert_eq!(typed["text"], "/gmgotoxyz -325.00 73.60 -212.80");

    // A tag alone has no stand point.
    let fake = client(vec![player_at(STAND_AT)]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@stand", args = { tag = "ArmYourself_FrostBody" }, label = "go" }"#,
    )
    .await;
    let want = format!("@stand by tag {FROST_TAG} needs radius_m and bearing_deg");
    assert_eq!(step_action(&ev).error.as_deref(), Some(want.as_str()));
    assert!(!teleported(&fake));

    // A point stands as it is, with no server read.
    let srv = server(vec![]);
    let fake = client(vec![player_at([1.0, 2.0, 3.0])]);
    let ev = run(
        &fake,
        Some(&srv),
        r#"{ tool = "@stand", args = { point = { x = 1.0, y = 2.0, z = 3.0 } }, label = "go" }"#,
    )
    .await;
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
    let typed = sent(&fake, "client_type_text").unwrap();
    assert_eq!(typed["text"], "/gmgotoxyz 1.00 2.00 3.00");
    assert!(
        !srv.names().iter().any(|n| n.starts_with("server_entity_")),
        "{:?}",
        srv.names()
    );
}

#[tokio::test(start_paused = true)]
async fn stand_with_no_player_position_says_so() {
    let fake = client(vec![
        json!({ "player": { "id": PLAYER, "position": null } }),
    ]);
    let ev = run(
        &fake,
        None,
        r#"{ tool = "@stand", args = { point = { x = 1.0, y = 2.0, z = 3.0 } }, label = "go" }"#,
    )
    .await;
    let err = step_action(&ev).error.clone().unwrap_or_default();
    assert!(
        err.starts_with("stand: no player position after 5 s"),
        "{err}"
    );
}

#[tokio::test]
async fn no_server_blocks_a_tag_stand_but_not_a_point_stand() {
    let fake = client(vec![player_at(STAND_AT)]);
    let ev = run(
        &fake,
        None,
        r#"{ tool = "@stand", args = { pose = "frost" }, label = "go" }"#,
    )
    .await;
    assert_eq!(ev.result, RowResult::Blocked, "{:?}", ev.reasons);
    assert!(
        ev.reasons
            .iter()
            .any(|r| r.contains("by tag needs the server lab MCP")),
        "{:?}",
        ev.reasons
    );
    assert!(!teleported(&fake));

    let fake = client(vec![player_at([1.0, 2.0, 3.0])]);
    let ev = run(
        &fake,
        None,
        r#"{ tool = "@stand", args = { point = { x = 1.0, y = 2.0, z = 3.0 } }, label = "go" }"#,
    )
    .await;
    assert_ne!(ev.result, RowResult::Blocked, "{:?}", ev.reasons);
    assert!(step_action(&ev).ok, "{:?}", step_action(&ev).error);
}

#[tokio::test]
async fn the_blocks_follow_the_action_exec_would_choose() {
    // The primary is not routed; its fallback clicks by pose: blocked.
    let fake = Fake::new(&[])
        .with("client_entity_find")
        .with("client_world_click");
    let ev = run(
        &fake,
        None,
        r#"{ tool = "@target", args = { entity_id = 5 }, label = "go", fallback = [
            { tool = "@world_click", args = { pose = "frost" } },
        ] }"#,
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

    // A pose primary that is not routed does not block a fallback that runs.
    let fake = Fake::new(&[])
        .with("client_entity_find")
        .with("client_target");
    let ev = run(
        &fake,
        None,
        r#"{ tool = "@world_click", args = { pose = "frost" }, label = "go", fallback = [
            { tool = "@target", args = { entity_id = 5 } },
        ] }"#,
    )
    .await;
    assert_ne!(ev.result, RowResult::Blocked, "{:?}", ev.reasons);
    assert_eq!(sent(&fake, "client_target").unwrap()["entity_id"], 5);
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
