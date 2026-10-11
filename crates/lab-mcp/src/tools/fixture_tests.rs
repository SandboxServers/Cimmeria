//! FX-04: `server_stage_fixture` stages only for an offline character on a
//! GM account, and refuses what the base apply would refuse later.
//!
//! Sentinel block `0x7046_0400`: each live-DB test takes a `0x10` slot,
//! account at the slot base and player one above.

use serde_json::{json, Value};
use sqlx::PgPool;

use cimmeria_services::base::lab_fixture::{
    fixture_pending, take_fixture_for, StagedItem, StagedMission, StagedMissionState,
};
use cimmeria_services::base::OnlinePlayer;

use super::{check_shape, parse_args, refuse_if_online, stage_with, Character, REFUSED};
use crate::test_support::require_db_or_skip;

const PLAYER_ACCOUNT: i32 = 0x7046_0410;
const OTHER_STEP: i32 = 0x7046_0420;
const STAGES: i32 = 0x7046_0430;
const ONLINE: i32 = 0x7046_0440;
const BAD_SEED: i32 = 0x7046_0450;
const AMBIGUOUS_A: i32 = 0x7046_0460;
const AMBIGUOUS_B: i32 = 0x7046_0470;
/// Never inserted: an item design id no seed row has.
const NO_SUCH_ITEM: i32 = 0x7046_0401;

/// The Cellblock weapon mission and its middle step.
const MISSION: i32 = 622;
const MISSION_STEP: i32 = 80623;
const OTHER_MISSION: i32 = 1360;

/// The runner's arguments for `name` (`crate::uat::fixture::stage_args` in
/// cimmeria-lab): a f32 position as f64 JSON, snake_case states, nulls
/// dropped.
fn runner_args(name: &str) -> Value {
    json!({
        "character": name,
        "world": "Castle_CellBlock",
        "position": [-319.0, 73.5999984741211, -212.5],
        "missions": [
            { "id": MISSION, "step": MISSION_STEP },
            { "id": OTHER_MISSION, "state": "not_active" }
        ],
        "items": [],
        "tutorials_unseen": [5882]
    })
}

fn character(player_id: i32) -> Character {
    Character {
        player_id,
        player_name: "Uat Fxpraxisdefa".to_string(),
        account_id: player_id - 1,
        account_name: "lab".to_string(),
        accesslevel: 2,
    }
}

fn session(player_id: Option<i32>, name: &str) -> OnlinePlayer {
    OnlinePlayer {
        id: 100,
        name: name.to_string(),
        player_id,
        account_id: 10,
        account_name: Some("lab".to_string()),
        archetype: "Soldier",
        level: 1,
        zone: "Castle_CellBlock".to_string(),
        ping: None,
        status: "in_world",
        session: "127.0.0.1:50000".to_string(),
    }
}

fn refusal(r: Result<impl std::fmt::Debug, String>) -> String {
    let e = r.expect_err("must be refused");
    assert!(e.starts_with(REFUSED), "refusal without the prefix: {e}");
    e
}

// ── Unit ──────────────────────────────────────────────────────────────

/// **Guard: an online character is refused.** The base reads a fixture
/// only at Play, so one staged for a character in the world would land on
/// a later login. A session counts by player id, or by name before
/// `playCharacter` set the id.
#[test]
fn an_online_character_is_refused() {
    let ch = character(77);
    let e = refusal(refuse_if_online(
        "Fxpraxisdefa",
        &ch,
        &[
            session(Some(5), "Someone"),
            session(Some(77), "Uat Fxpraxisdefa"),
        ],
    ));
    assert_eq!(e, "stage refused: character Fxpraxisdefa is online");
    refusal(refuse_if_online(
        "Fxpraxisdefa",
        &ch,
        &[session(None, "uat fxpraxisdefa")],
    ));
    assert!(refuse_if_online("Fxpraxisdefa", &ch, &[session(Some(5), "Someone")]).is_ok());
    assert!(
        refuse_if_online("Fxpraxisdefa", &ch, &[session(None, "")]).is_ok(),
        "a nameless character-select session is no character's"
    );
}

#[test]
fn the_runner_arguments_parse_into_the_staged_shape() {
    let args = parse_args(runner_args("Fxpraxisdefa")).expect("runner args parse");
    assert_eq!(args.position, Some([-319.0, 73.6, -212.5]));
    let (missions, items) = check_shape(&args).expect("shape");
    assert_eq!(
        missions,
        vec![
            StagedMission {
                mission_id: MISSION,
                state: StagedMissionState::Active {
                    step_id: MISSION_STEP
                }
            },
            StagedMission {
                mission_id: OTHER_MISSION,
                state: StagedMissionState::NotActive
            },
        ]
    );
    assert!(items.is_empty());

    let args = parse_args(json!({
        "character": "X",
        "missions": [{ "id": 1, "state": "completed" }, { "id": 2, "step": 3, "state": "active" }],
        "items": [{ "id": 55, "count": 0 }, { "id": 3730, "count": 10, "container": "mission" }]
    }))
    .expect("parse");
    let (missions, items) = check_shape(&args).expect("shape");
    assert_eq!(missions[0].state, StagedMissionState::Completed);
    assert_eq!(missions[1].state, StagedMissionState::Active { step_id: 3 });
    assert_eq!(
        items,
        vec![
            StagedItem {
                type_id: 55,
                count: 0,
                container_id: 1
            },
            StagedItem {
                type_id: 3730,
                count: 10,
                container_id: 2
            },
        ]
    );
}

#[test]
fn a_malformed_call_is_a_refusal() {
    let e = refusal(parse_args(json!({ "character": "X", "rewind": true })));
    assert!(e.contains("rewind"), "names the unknown field: {e}");
    refusal(parse_args(
        json!({ "character": "X", "missions": [{ "id": 1, "state": "done" }] }),
    ));
    refusal(parse_args(json!({ "world": "Castle" })));
}

#[test]
fn shape_refusals_name_the_field() {
    let shape = |v: Value| check_shape(&parse_args(v).expect("parse"));
    let cases = [
        (
            json!({ "character": "X", "items": [{ "id": 5, "count": 11 }] }),
            "items[0]: count 11",
        ),
        (
            json!({ "character": "X", "items": [{ "id": 5, "count": 1, "container": "bank" }] }),
            "items[0]: container \"bank\"",
        ),
        (
            json!({ "character": "X", "items": [{ "id": 5, "count": 1 }, { "id": 5, "count": 0, "container": "mission" }] }),
            "items[1]: item 5 is listed twice",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1, "state": "active" }] }),
            "missions[0]: mission 1 is active but has no step",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1, "step": 2, "state": "not_active" }] }),
            "missions[0]: mission 1 has a step",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1 }] }),
            "missions[0]: mission 1 needs a step or a state",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1, "state": "completed" }, { "id": 1, "state": "not_active" }] }),
            "missions[1]: mission 1 is listed twice",
        ),
        (
            json!({ "character": "X", "tutorials_unseen": [5882, 0] }),
            "tutorials_unseen: 0",
        ),
        (
            json!({ "character": "X", "position": [0.0, 1e39, 0.0] }),
            "position",
        ),
        (json!({ "character": "X", "world": " " }), "world is empty"),
    ];
    for (args, want) in cases {
        let e = refusal(shape(args.clone()));
        assert!(e.contains(want), "{args}: want {want:?} in {e:?}");
    }
}

// ── Live-DB ───────────────────────────────────────────────────────────

fn player_name(player_id: i32) -> String {
    format!("Uat Fx04-{player_id:x}")
}

/// The name the runner would send: the last word, in another case.
fn requested(player_id: i32) -> String {
    format!("FX04-{player_id:X}")
}

async fn cleanup(pool: &PgPool, account_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(account_id + 1)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// An account at `accesslevel` with one character named `name`.
async fn seed(pool: &PgPool, account_id: i32, accesslevel: i32, name: &str) -> i32 {
    cleanup(pool, account_id).await;
    let player_id = account_id + 1;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, accesslevel) \
         VALUES ($1, $2, '', $3)",
    )
    .bind(account_id)
    .bind(format!("fx04-{account_id}"))
    .bind(accesslevel)
    .execute(pool)
    .await
    .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'Castle_CellBlock', \
                   'BS_HumanMale.BS_HumanMale', 0, 0, 0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert player");
    player_id
}

async fn stage(pool: &PgPool, online: &[OnlinePlayer], v: Value) -> Result<Value, String> {
    stage_with(pool, online, parse_args(v).expect("parse")).await
}

/// **Guard: no fixture for a player account (gate 3).** The base would
/// discard it at Play, but a fixture must never sit staged for a
/// character no lab GM owns.
#[tokio::test]
async fn live_db_stage_fixture_refuses_a_player_account() {
    let pool = require_db_or_skip!();
    let player_id = seed(&pool, PLAYER_ACCOUNT, 0, &player_name(PLAYER_ACCOUNT + 1)).await;
    let result = stage(&pool, &[], runner_args(&requested(player_id))).await;
    let pending = fixture_pending(player_id);
    cleanup(&pool, PLAYER_ACCOUNT).await;
    let e = refusal(result);
    assert!(e.contains("is not on a GM account"), "{e}");
    assert!(!pending, "nothing is staged for a player account");
}

/// The base precheck would refuse a step of another mission after the
/// character pressed Play; the tool refuses it at stage time.
#[tokio::test]
async fn live_db_stage_fixture_refuses_a_step_from_another_mission() {
    let pool = require_db_or_skip!();
    let player_id = seed(&pool, OTHER_STEP, 2, &player_name(OTHER_STEP + 1)).await;
    let mut args = runner_args(&requested(player_id));
    args["missions"] = json!([{ "id": OTHER_MISSION, "step": MISSION_STEP }]);
    let result = stage(&pool, &[], args).await;
    let pending = fixture_pending(player_id);
    cleanup(&pool, OTHER_STEP).await;
    let e = refusal(result);
    assert!(
        e.contains(&format!(
            "missions[0].step: step {MISSION_STEP} belongs to mission {MISSION}, not mission {OTHER_MISSION}"
        )),
        "{e}"
    );
    assert!(!pending);
}

/// Stages for an offline GM character, matched by its name's last word in
/// another case, with the owner's account id: the account the base's
/// take compares against the playing session's.
#[tokio::test]
async fn live_db_stage_fixture_stages_for_an_offline_gm_character() {
    let pool = require_db_or_skip!();
    let name = player_name(STAGES + 1);
    let player_id = seed(&pool, STAGES, 2, &name).await;
    let other_online = [session(Some(5), "Someone")];
    let first = stage(&pool, &other_online, runner_args(&requested(player_id))).await;
    let pending = fixture_pending(player_id);
    let second = stage(&pool, &[], runner_args(&name.to_lowercase())).await;
    let taken = take_fixture_for(player_id, STAGES);
    cleanup(&pool, STAGES).await;

    assert_eq!(
        first.expect("staged"),
        json!({ "staged": true, "player_id": player_id, "player_name": name, "replaced": false })
    );
    assert!(pending, "the fixture waits for the character's Play");
    assert_eq!(
        second.expect("staged again, by full name")["replaced"],
        true
    );
    let f = taken.expect("the owning account takes it");
    assert_eq!(f.account_id, STAGES);
    assert_eq!(f.player_name, name);
    assert_eq!(f.world.as_deref(), Some("Castle_CellBlock"));
    assert_eq!(f.position, Some([-319.0, 73.6, -212.5]));
    assert_eq!(f.tutorials_unseen, vec![5882]);
    assert_eq!(f.missions.len(), 2);
}

/// Gate 4 on the live path: the character's own session refuses it.
#[tokio::test]
async fn live_db_stage_fixture_refuses_an_online_character() {
    let pool = require_db_or_skip!();
    let name = player_name(ONLINE + 1);
    let player_id = seed(&pool, ONLINE, 2, &name).await;
    let online = [session(Some(player_id), &name)];
    let result = stage(&pool, &online, runner_args(&requested(player_id))).await;
    let pending = fixture_pending(player_id);
    cleanup(&pool, ONLINE).await;
    let e = refusal(result);
    assert_eq!(
        e,
        format!(
            "stage refused: character {} is online",
            requested(player_id)
        )
    );
    assert!(!pending);
}

/// An unknown world, mission or item design is refused before staging.
#[tokio::test]
async fn live_db_stage_fixture_refuses_what_the_seed_lacks() {
    let pool = require_db_or_skip!();
    let player_id = seed(&pool, BAD_SEED, 2, &player_name(BAD_SEED + 1)).await;
    let name = requested(player_id);
    let mut world = runner_args(&name);
    world["world"] = json!("Fx04_No_Such_World");
    let mut mission = runner_args(&name);
    mission["missions"] = json!([{ "id": NO_SUCH_ITEM, "state": "not_active" }]);
    let mut item = runner_args(&name);
    item["items"] = json!([{ "id": NO_SUCH_ITEM, "count": 1 }]);
    let results = [
        stage(&pool, &[], world).await,
        stage(&pool, &[], mission).await,
        stage(&pool, &[], item).await,
    ];
    let pending = fixture_pending(player_id);
    cleanup(&pool, BAD_SEED).await;
    let [world, mission, item] = results.map(refusal);
    assert!(
        world.contains("world Fx04_No_Such_World is not in resources.worlds"),
        "{world}"
    );
    assert!(
        mission.contains("is not in resources.missions"),
        "{mission}"
    );
    assert!(item.contains("is not in resources.items"), "{item}");
    assert!(!pending);
}

/// Two characters whose names end in the same word: neither is guessed.
#[tokio::test]
async fn live_db_stage_fixture_refuses_an_ambiguous_name() {
    let pool = require_db_or_skip!();
    let word = format!("Fx04amb{AMBIGUOUS_A:x}");
    let a = seed(&pool, AMBIGUOUS_A, 2, &format!("Uat {word}")).await;
    let b = seed(&pool, AMBIGUOUS_B, 2, &format!("Other {word}")).await;
    let result = stage(&pool, &[], runner_args(&word)).await;
    let pending = fixture_pending(a) || fixture_pending(b);
    cleanup(&pool, AMBIGUOUS_A).await;
    cleanup(&pool, AMBIGUOUS_B).await;
    let e = refusal(result);
    assert!(e.contains("matches 2 characters"), "{e}");
    assert!(!pending);
}
