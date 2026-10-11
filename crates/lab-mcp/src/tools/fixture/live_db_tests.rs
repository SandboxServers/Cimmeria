//! FX-04 live-DB tests: `server_stage_fixture` stages only for an offline
//! lab fixture character on a GM account, and refuses what the base apply
//! would refuse later. Every test runs the tool's own path,
//! [`stage_from`], with a [`FakeContext`] holding the pool and the online
//! roster, the two things `LabState` hands it.
//!
//! Sentinel block `0x7046_0400`: each test takes a `0x10` slot, account at
//! the slot base and player one above. Character names are fixture names
//! (`Fx` then letters), so the slot is spelled in letters.

use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::Level;

use cimmeria_services::base::lab_fixture::{fixture_pending, take_fixture_for};
use cimmeria_services::base::OnlinePlayer;

use super::tests::{
    refusal, runner_args, session, FakeContext, MISSION, MISSION_STEP, OTHER_MISSION,
};
use super::{stage_from, Refusal};
use crate::test_support::{require_db_or_skip, LogCapture};

const PLAYER_ACCOUNT: i32 = 0x7046_0410;
const OTHER_STEP: i32 = 0x7046_0420;
const STAGES: i32 = 0x7046_0430;
const ONLINE: i32 = 0x7046_0440;
const BAD_SEED: i32 = 0x7046_0450;
const AMBIGUOUS_A: i32 = 0x7046_0460;
const AMBIGUOUS_B: i32 = 0x7046_0470;
const NOT_FIXTURE: i32 = 0x7046_0480;
const COMPLETED: i32 = 0x7046_0490;
/// Never inserted: no character has this slot's name.
const NO_MATCH: i32 = 0x7046_04a0;
/// Never inserted: an item design id no seed row has.
const NO_SUCH_ITEM: i32 = 0x7046_0401;
/// A mission the test inserts with no steps, so it cannot be completed.
const STEPLESS_MISSION: i32 = 0x7046_0402;

/// `id` in hex, each digit as a letter (`0` is `a`, `f` is `p`).
fn letters(id: i32) -> String {
    format!("{id:x}")
        .chars()
        .map(|c| (b'a' + c.to_digit(16).unwrap_or(0) as u8) as char)
        .collect()
}

fn player_name(player_id: i32) -> String {
    format!("Uat Fx{}", letters(player_id))
}

/// The name the runner would send: the last word, in another case.
fn requested(player_id: i32) -> String {
    format!("FX{}", letters(player_id).to_uppercase())
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

/// The tool's path with this pool and roster.
async fn stage(pool: &PgPool, roster: &[OnlinePlayer], v: Value) -> Result<Value, Refusal> {
    let ctx = FakeContext {
        pool: Some(pool.clone()),
        roster: roster.to_vec(),
    };
    stage_from(&ctx, v).await
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
    assert_eq!(e.reason, "not_gm");
    assert!(e.message.contains("is not on a GM account"), "{e:?}");
    assert!(!pending, "nothing is staged for a player account");
}

/// **Guard: only a lab fixture character is staged for.** A GM's own
/// character ("Uat Smithers...") is refused, so a token holder cannot
/// stage onto a real GM's main; the refusal logs its reason and the
/// character.
#[tokio::test]
async fn live_db_stage_fixture_refuses_a_gm_character_that_is_not_a_fixture() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let last = format!("Smithers{}", letters(NOT_FIXTURE + 1));
    let name = format!("Uat {last}");
    let player_id = seed(&pool, NOT_FIXTURE, 2, &name).await;
    let result = stage(&pool, &[], runner_args(&last)).await;
    let pending = fixture_pending(player_id);
    cleanup(&pool, NOT_FIXTURE).await;
    let e = refusal(result);
    assert_eq!(e.reason, "not_fixture_character");
    assert_eq!(
        e.message,
        format!(
            "stage refused: character {name} is not a lab fixture character (last name must be Fx...)"
        )
    );
    assert!(!pending, "nothing is staged for a real character");
    let logged = capture
        .find_event(
            Level::WARN,
            "lab fixture stage refused",
            "not_fixture_character",
        )
        .unwrap_or_else(|| panic!("{:#?}", capture.all()));
    assert!(
        logged.has_field("player_id", &player_id.to_string()),
        "{logged:?}"
    );
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
    assert_eq!(e.reason, "step");
    assert!(
        e.message.contains(&format!(
            "missions[0].step: step {MISSION_STEP} belongs to mission {MISSION}, not mission {OTHER_MISSION}"
        )),
        "{e:?}"
    );
    assert!(!pending);
}

/// Stages for an offline GM fixture character, matched by its name's last
/// word in another case, with the owner's account id: the account the
/// base's take compares against the playing session's.
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

/// Gate 4 on the live path: the character's own session, in the roster
/// the context hands the tool, refuses it.
#[tokio::test]
async fn live_db_stage_fixture_refuses_an_online_character() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let name = player_name(ONLINE + 1);
    let player_id = seed(&pool, ONLINE, 2, &name).await;
    let online = [session(Some(player_id), &name)];
    let result = stage(&pool, &online, runner_args(&requested(player_id))).await;
    let pending = fixture_pending(player_id);
    cleanup(&pool, ONLINE).await;
    let e = refusal(result);
    assert_eq!(
        e.message,
        format!(
            "stage refused: character {} is online",
            requested(player_id)
        )
    );
    assert!(!pending);
    let logged = capture
        .find_event(Level::WARN, "lab fixture stage refused", "online")
        .unwrap_or_else(|| panic!("{:#?}", capture.all()));
    assert!(logged.has_field("player_name", &name), "{logged:?}");
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
    assert_eq!(
        (world.reason, mission.reason, item.reason),
        ("world", "mission", "item")
    );
    assert!(
        world
            .message
            .contains("world Fx04_No_Such_World is not in resources.worlds"),
        "{world:?}"
    );
    assert!(
        mission.message.contains("is not in resources.missions"),
        "{mission:?}"
    );
    assert!(
        item.message.contains("is not in resources.items"),
        "{item:?}"
    );
    assert!(!pending);
}

/// **Guard: a mission with no steps cannot be staged completed** (the base
/// `build_row` would refuse it at Play); one with steps can.
#[tokio::test]
async fn live_db_stage_fixture_completes_only_a_mission_with_steps() {
    let pool = require_db_or_skip!();
    let player_id = seed(&pool, COMPLETED, 2, &player_name(COMPLETED + 1)).await;
    let _ = sqlx::query("DELETE FROM resources.missions WHERE mission_id = $1")
        .bind(STEPLESS_MISSION)
        .execute(&pool)
        .await;
    sqlx::query(
        "INSERT INTO resources.missions (\
                mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, \
                difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, \
                is_shareable, level, mission_defn, mission_label, num_repeats, \
                show_faction_change_icon, show_instance_icon, show_pvp_icon\
             ) VALUES ($1, '', false, false, false, false, 1, false, true, false, false, \
                       false, 1, 'Fx04Stepless', 'test', 0, false, false, false)",
    )
    .bind(STEPLESS_MISSION)
    .execute(&pool)
    .await
    .expect("insert stepless mission");

    let name = requested(player_id);
    let mut stepless = runner_args(&name);
    stepless["missions"] = json!([{ "id": STEPLESS_MISSION, "state": "completed" }]);
    let refused = stage(&pool, &[], stepless).await;
    let pending_after_refusal = fixture_pending(player_id);
    let mut with_steps = runner_args(&name);
    with_steps["missions"] = json!([{ "id": MISSION, "state": "completed" }]);
    let staged = stage(&pool, &[], with_steps).await;
    let taken = take_fixture_for(player_id, COMPLETED);

    let _ = sqlx::query("DELETE FROM resources.missions WHERE mission_id = $1")
        .bind(STEPLESS_MISSION)
        .execute(&pool)
        .await;
    cleanup(&pool, COMPLETED).await;

    let e = refusal(refused);
    assert_eq!(e.reason, "mission");
    assert_eq!(
        e.message,
        format!("stage refused: missions[0]: mission {STEPLESS_MISSION} has no steps to complete")
    );
    assert!(!pending_after_refusal);
    staged.expect("a mission with steps can be completed");
    let f = taken.expect("staged");
    assert_eq!(f.missions.len(), 1);
    assert_eq!(f.missions[0].mission_id, MISSION);
    assert_eq!(
        f.missions[0].state,
        cimmeria_services::base::lab_fixture::StagedMissionState::Completed
    );
}

/// A name no character has is refused, not guessed.
#[tokio::test]
async fn live_db_stage_fixture_refuses_a_name_that_matches_no_character() {
    let pool = require_db_or_skip!();
    let name = requested(NO_MATCH + 1);
    let e = refusal(stage(&pool, &[], runner_args(&name)).await);
    assert_eq!(e.reason, "no_character");
    assert_eq!(
        e.message,
        format!("stage refused: character {name} matches no character")
    );
}

/// Two characters whose names end in the same word: neither is guessed.
#[tokio::test]
async fn live_db_stage_fixture_refuses_an_ambiguous_name() {
    let pool = require_db_or_skip!();
    let word = format!("Fxamb{}", letters(AMBIGUOUS_A));
    let a = seed(&pool, AMBIGUOUS_A, 2, &format!("Uat {word}")).await;
    let b = seed(&pool, AMBIGUOUS_B, 2, &format!("Other {word}")).await;
    let result = stage(&pool, &[], runner_args(&word)).await;
    let pending = fixture_pending(a) || fixture_pending(b);
    cleanup(&pool, AMBIGUOUS_A).await;
    cleanup(&pool, AMBIGUOUS_B).await;
    let e = refusal(result);
    assert_eq!(e.reason, "ambiguous");
    assert!(e.message.contains("matches 2 characters"), "{e:?}");
    assert!(!pending);
}
