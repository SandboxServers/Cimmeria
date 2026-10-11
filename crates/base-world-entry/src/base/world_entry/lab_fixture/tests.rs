//! Live-DB guards for the staged lab fixture apply (lab-fixtures FX-02),
//! and the fixtures they share with `play_character_live_db_tests`.
//!
//! Sentinel block `0x7046_0200` (the store's in-memory ids sit at
//! `0x7046_0201..=0x7046_0203`); each test takes a `0x10` slot, account at
//! the slot base and player one above.

use sqlx::PgPool;

use super::{apply_staged_fixture, FixtureApplied};

use cimmeria_base_session::base::world_entry::lab_fixture::{
    StagedFixture, StagedMission, StagedMissionState,
};

pub(super) const COMPLETED_BACK: i32 = 0x7046_0210;
pub(super) const NOT_ACTIVE: i32 = 0x7046_0220;
pub(super) const BAD_STEP: i32 = 0x7046_0230;
pub(super) const TUTORIALS: i32 = 0x7046_0240;
pub(super) const MOVE: i32 = 0x7046_0250;
pub(super) const ITEMS: i32 = 0x7046_0258;
pub(super) const OTHER_ACCOUNT: i32 = 0x7046_0260;
pub(super) const STRANGER_ACCOUNT: i32 = 0x7046_0268;
pub(super) const PLAY_GM: i32 = 0x7046_0270;
pub(super) const PLAY_PLAYER: i32 = 0x7046_0280;
pub(super) const COMPLETED: i32 = 0x7046_0290;
pub(super) const UNKNOWN_WORLD: i32 = 0x7046_02A0;
pub(super) const REFUSED_WRITE: i32 = 0x7046_02B0;
pub(super) const LOG_ROWS: i32 = 0x7046_02C0;
pub(super) const LOG_ROWS_STRANGER: i32 = 0x7046_02C8;
pub(super) const PLAY_VICTIM: i32 = 0x7046_02D0;
pub(super) const PLAY_THIEF: i32 = 0x7046_02E0;

/// The Cellblock weapon mission: three steps, enough for a middle one.
pub(super) const MISSION: i32 = 622;
/// A second seeded mission, for the "step from another mission" case.
pub(super) const OTHER_MISSION: i32 = 1360;
pub(super) const TUTORIAL: i32 = 5882;
pub(super) const OTHER_TUTORIAL: i32 = 5883;

pub(super) const START: [f32; 3] = [-334.231, 73.472, -228.026];

pub(super) async fn cleanup(pool: &PgPool, account_id: i32) {
    let player_id = account_id + 1;
    for sql in [
        "DELETE FROM sgw_mission WHERE player_id = $1",
        "DELETE FROM sgw_player_tutorials WHERE player_id = $1",
        "DELETE FROM sgw_player WHERE player_id = $1",
    ] {
        let _ = sqlx::query(sql).bind(player_id).execute(pool).await;
    }
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// An account and one character in Castle_CellBlock at [`START`].
pub(super) async fn seed_player(pool: &PgPool, account_id: i32) -> i32 {
    cleanup(pool, account_id).await;
    let player_id = account_id + 1;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("fx02-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'Castle_CellBlock', \
                   'BS_HumanMale.BS_HumanMale', $4, $5, $6, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("fx02-{player_id}"))
    .bind(START[0])
    .bind(START[1])
    .bind(START[2])
    .execute(pool)
    .await
    .expect("insert player");
    player_id
}

/// A saved mission row, the way the cell's persist leaves one.
pub(super) async fn seed_mission(
    pool: &PgPool,
    player_id: i32,
    mission_id: i32,
    status: i32,
    repeats: i32,
) {
    let done = sqlx::query(
        "INSERT INTO sgw_mission (player_id, mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, \
         failed_objective_ids, repeats) \
         VALUES ($1, $2, $3, NULL, '{}', '{}', '{}', '{}', $4)",
    )
    .bind(player_id)
    .bind(mission_id)
    .bind(status)
    .bind(repeats)
    .execute(pool)
    .await
    .expect("insert mission");
    assert_eq!(done.rows_affected(), 1);
}

#[derive(Debug, PartialEq, sqlx::FromRow)]
pub(super) struct MissionRow {
    pub(super) status: i32,
    pub(super) current_step_id: Option<i32>,
    pub(super) completed_step_ids: Vec<i32>,
    pub(super) completed_objective_ids: Vec<i32>,
    pub(super) active_objective_ids: Vec<i32>,
    pub(super) repeats: i32,
}

pub(super) async fn mission_row(
    pool: &PgPool,
    player_id: i32,
    mission_id: i32,
) -> Option<MissionRow> {
    sqlx::query_as(
        "SELECT status, current_step_id, completed_step_ids, completed_objective_ids, \
         active_objective_ids, repeats FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
    )
    .bind(player_id)
    .bind(mission_id)
    .fetch_optional(pool)
    .await
    .expect("read mission")
}

/// The mission's steps in play order, read from the seed so a renumbered
/// step does not break the test.
pub(super) async fn steps_of(pool: &PgPool, mission_id: i32) -> Vec<i32> {
    let steps: Vec<i32> = sqlx::query_scalar(
        "SELECT step_id FROM resources.mission_steps WHERE mission_id = $1 \
         ORDER BY index ASC NULLS LAST, step_id ASC",
    )
    .bind(mission_id)
    .fetch_all(pool)
    .await
    .expect("read steps");
    assert!(
        steps.len() >= 3,
        "fixture sanity: mission {mission_id} needs three steps in the seed, has {steps:?}"
    );
    steps
}

pub(super) async fn objectives_of_step(pool: &PgPool, step_id: i32) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT objective_id FROM resources.mission_objectives WHERE step_id = $1 \
         ORDER BY objective_id",
    )
    .bind(step_id)
    .fetch_all(pool)
    .await
    .expect("read objectives")
}

#[derive(Debug, PartialEq, sqlx::FromRow)]
pub(super) struct Place {
    pub(super) world_location: String,
    pub(super) world_id: Option<i32>,
    pub(super) pos_x: f32,
    pub(super) pos_y: f32,
    pub(super) pos_z: f32,
}

pub(super) async fn place(pool: &PgPool, player_id: i32) -> Place {
    sqlx::query_as(
        "SELECT world_location, world_id, pos_x, pos_y, pos_z FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await
    .expect("read place")
}

/// A fixture for `player_id`, staged for its owner (the account one below,
/// as [`seed_player`] lays them out).
pub(super) fn fixture(player_id: i32, missions: Vec<StagedMission>) -> StagedFixture {
    StagedFixture {
        player_id,
        account_id: player_id - 1,
        player_name: format!("fx02-{player_id}"),
        world: None,
        position: None,
        missions,
        items: Vec::new(),
        tutorials_unseen: Vec::new(),
    }
}

/// [pply_staged_fixture] with the error as its message, for the
/// assertions that read the cause.
pub(super) async fn apply(
    pool: &PgPool,
    account_id: i32,
    f: &StagedFixture,
) -> Result<FixtureApplied, String> {
    apply_staged_fixture(pool, account_id, f)
        .await
        .map_err(|e| e.message().to_string())
}

pub(super) fn active(mission_id: i32, step_id: i32) -> StagedMission {
    StagedMission {
        mission_id,
        state: StagedMissionState::Active { step_id },
    }
}

mod lab_fixture_live_db_tests {
    use cimmeria_base_session::base::world_entry::lab_fixture::StagedItem;

    use super::*;
    use crate::base::world_entry::shown_tutorials::{record_tutorial_shown, TutorialInsert};
    use crate::test_support::require_db_or_skip;

    /// **Guard: a fixture puts a completed mission back on a middle step**,
    /// the move no GM command can make (the route forward refuses a
    /// completed mission at its repeat cap). The earlier step and its
    /// objectives are completed, so chains reading them see the route's
    /// state; `repeats` goes back to 0. Drop the completed-steps fill or the
    /// repeats reset and an assertion fails.
    #[tokio::test]
    async fn live_db_fixture_moves_a_completed_mission_back_to_a_middle_step() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, COMPLETED_BACK).await;
        seed_mission(&pool, player_id, MISSION, 2, 1).await;
        let steps = steps_of(&pool, MISSION).await;
        let first_objectives = objectives_of_step(&pool, steps[0]).await;

        let applied = apply(
            &pool,
            COMPLETED_BACK,
            &fixture(player_id, vec![active(MISSION, steps[1])]),
        )
        .await;
        let row = mission_row(&pool, player_id, MISSION).await;
        cleanup(&pool, COMPLETED_BACK).await;

        assert_eq!(applied.expect("applied").missions, 1);
        assert_eq!(
            row.expect("the mission row is kept"),
            MissionRow {
                status: 1,
                current_step_id: Some(steps[1]),
                completed_step_ids: vec![steps[0]],
                completed_objective_ids: first_objectives,
                active_objective_ids: Vec::new(),
                repeats: 0,
            }
        );
    }

    /// **Guard: not active means never taken**, even over a row with
    /// repeats: the fixture writes `repeats = 0`, so the persist's abandon
    /// rule deletes the row instead of keeping a not-active one.
    #[tokio::test]
    async fn live_db_fixture_not_active_deletes_the_row_even_with_repeats() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, NOT_ACTIVE).await;
        seed_mission(&pool, player_id, MISSION, 2, 3).await;

        let applied = apply(
            &pool,
            NOT_ACTIVE,
            &fixture(
                player_id,
                vec![StagedMission {
                    mission_id: MISSION,
                    state: StagedMissionState::NotActive,
                }],
            ),
        )
        .await;
        let row = mission_row(&pool, player_id, MISSION).await;
        cleanup(&pool, NOT_ACTIVE).await;

        assert_eq!(applied.expect("applied").missions, 1);
        assert_eq!(row, None, "a not-active fixture mission leaves no row");
    }

    /// **Guard: every check runs before the first write.** The first
    /// mission is valid, the second names a step of another mission; the
    /// apply fails and the first mission's row is as it was. Checking and
    /// writing mission by mission would rewrite the first one.
    #[tokio::test]
    async fn live_db_fixture_step_from_another_mission_writes_nothing() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, BAD_STEP).await;
        seed_mission(&pool, player_id, MISSION, 2, 1).await;
        let steps = steps_of(&pool, MISSION).await;
        let before = mission_row(&pool, player_id, MISSION).await;

        let applied = apply(
            &pool,
            BAD_STEP,
            &fixture(
                player_id,
                vec![active(MISSION, steps[1]), active(OTHER_MISSION, steps[0])],
            ),
        )
        .await;
        let after = mission_row(&pool, player_id, MISSION).await;
        let other = mission_row(&pool, player_id, OTHER_MISSION).await;
        cleanup(&pool, BAD_STEP).await;

        let error = applied.expect_err("a step of another mission is refused");
        assert!(
            error.contains(&steps[0].to_string()) && error.contains(&OTHER_MISSION.to_string()),
            "the error names the step and the mission: {error}"
        );
        assert!(before.is_some());
        assert_eq!(after, before, "the valid mission's row is unchanged");
        assert_eq!(other, None);
    }

    /// **Guard: only the named tutorials are forgotten, and a forgotten one
    /// shows again**: the next record of it is a first insert, the outcome
    /// on which the cell displays the dialog.
    #[tokio::test]
    async fn live_db_fixture_forgets_only_the_named_tutorials() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, TUTORIALS).await;
        for id in [TUTORIAL, OTHER_TUTORIAL] {
            let first = record_tutorial_shown(&pool, player_id, TUTORIALS, id).await;
            assert_eq!(first.expect("record"), TutorialInsert::Inserted);
        }

        let mut f = fixture(player_id, Vec::new());
        f.tutorials_unseen = vec![TUTORIAL];
        let applied = apply(&pool, TUTORIALS, &f).await;
        let left: Vec<i32> = sqlx::query_scalar(
            "SELECT tutorial_id FROM sgw_player_tutorials WHERE player_id = $1 ORDER BY tutorial_id",
        )
        .bind(player_id)
        .fetch_all(&pool)
        .await
        .expect("read tutorials");
        let again = record_tutorial_shown(&pool, player_id, TUTORIALS, TUTORIAL).await;
        cleanup(&pool, TUTORIALS).await;

        assert_eq!(applied.expect("applied").tutorials_forgotten, 1);
        assert_eq!(left, vec![OTHER_TUTORIAL]);
        assert_eq!(again.expect("record again"), TutorialInsert::Inserted);
    }

    /// **Guard: world and position land together**, with `world_id`
    /// resolved from `resources.worlds` the way the logout persist does.
    #[tokio::test]
    async fn live_db_fixture_moves_world_and_position() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, MOVE).await;
        let castle_id: Option<i32> =
            sqlx::query_scalar("SELECT world_id FROM resources.worlds WHERE world = 'Castle'")
                .fetch_optional(&pool)
                .await
                .expect("read world");
        assert!(castle_id.is_some(), "fixture sanity: the seed has Castle");

        let mut f = fixture(player_id, Vec::new());
        f.world = Some("Castle".to_string());
        f.position = Some([411.5, 70.25, 987.75]);
        let applied = apply(&pool, MOVE, &f).await;
        let stored = place(&pool, player_id).await;
        cleanup(&pool, MOVE).await;

        assert!(applied.expect("applied").moved);
        assert_eq!(
            stored,
            Place {
                world_location: "Castle".to_string(),
                world_id: castle_id,
                pos_x: 411.5,
                pos_y: 70.25,
                pos_z: 987.75,
            }
        );
    }

    /// **Guard: another account's character is refused, and nothing is
    /// written.** The store is keyed by player only; the session's account
    /// is what proves the character is the one being played.
    #[tokio::test]
    async fn live_db_fixture_refuses_another_accounts_character() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, OTHER_ACCOUNT).await;
        seed_player(&pool, STRANGER_ACCOUNT).await;
        seed_mission(&pool, player_id, MISSION, 2, 1).await;
        let steps = steps_of(&pool, MISSION).await;
        let mission_before = mission_row(&pool, player_id, MISSION).await;
        let place_before = place(&pool, player_id).await;

        let mut f = fixture(player_id, vec![active(MISSION, steps[1])]);
        f.world = Some("Castle".to_string());
        f.position = Some([1.0, 2.0, 3.0]);
        let applied = apply(&pool, STRANGER_ACCOUNT, &f).await;
        let mission_after = mission_row(&pool, player_id, MISSION).await;
        let place_after = place(&pool, player_id).await;
        cleanup(&pool, OTHER_ACCOUNT).await;
        cleanup(&pool, STRANGER_ACCOUNT).await;

        let error = applied.expect_err("another account's character is refused");
        assert!(error.contains("belongs to account"), "{error}");
        assert_eq!(mission_after, mission_before);
        assert_eq!(place_after, place_before);
    }

    /// FX-02 leaves item counts to FX-03: an item fixture is refused, not
    /// silently skipped, and refused in the precheck, so the fixture's
    /// mission is not written either. Move the refusal back into the items
    /// part (after the missions write) and the row assertion fails.
    #[tokio::test]
    async fn live_db_fixture_items_are_refused_until_fx03() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, ITEMS).await;
        seed_mission(&pool, player_id, MISSION, 2, 1).await;
        let steps = steps_of(&pool, MISSION).await;
        let before = mission_row(&pool, player_id, MISSION).await;
        let mut f = fixture(player_id, vec![active(MISSION, steps[1])]);
        f.items = vec![StagedItem {
            type_id: 55,
            count: 1,
            container_id: 1,
        }];
        let applied = apply(&pool, ITEMS, &f).await;
        let after = mission_row(&pool, player_id, MISSION).await;
        cleanup(&pool, ITEMS).await;
        assert_eq!(applied, Err("item fixtures land in FX-03".to_string()));
        assert!(before.is_some());
        assert_eq!(after, before, "the mission row is untouched");
    }

    /// **Guard: the completed row shape.** Over an active mid-step row, a
    /// `Completed` fixture writes status 2, no current step, every step in
    /// index order, every one of their objectives, and `repeats = 1` (what
    /// `MissionInstance::complete` leaves after a first completion). Change
    /// the repeats or drop the objective fill and an assertion fails.
    #[tokio::test]
    async fn live_db_fixture_completed_writes_every_step_and_repeats_one() {
        let pool = require_db_or_skip!();
        let player_id = seed_player(&pool, COMPLETED).await;
        seed_mission(&pool, player_id, MISSION, 1, 0).await;
        let steps = steps_of(&pool, MISSION).await;
        let mut objectives = Vec::new();
        for step in &steps {
            objectives.extend(objectives_of_step(&pool, *step).await);
        }
        assert!(
            !objectives.is_empty(),
            "fixture sanity: mission {MISSION} has objectives in the seed"
        );

        let applied = apply(
            &pool,
            COMPLETED,
            &fixture(
                player_id,
                vec![StagedMission {
                    mission_id: MISSION,
                    state: StagedMissionState::Completed,
                }],
            ),
        )
        .await;
        let row = mission_row(&pool, player_id, MISSION).await;
        cleanup(&pool, COMPLETED).await;

        assert_eq!(applied.expect("applied").missions, 1);
        assert_eq!(
            row.expect("the completed row is kept"),
            MissionRow {
                status: 2,
                current_step_id: None,
                completed_step_ids: steps,
                completed_objective_ids: objectives,
                active_objective_ids: Vec::new(),
                repeats: 1,
            }
        );
    }
}
