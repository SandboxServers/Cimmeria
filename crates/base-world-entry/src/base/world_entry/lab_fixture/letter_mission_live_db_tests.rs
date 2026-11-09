//! Live-DB guards for staging mission 1360 (Frost's Letter), whose persist
//! accepts only two exact active snapshots (see `missions.rs`).
//!
//! Sentinel slots `0x7046_02F0`, `02F2`, `02F4`, `02F8`, `02FC` from `tests.rs`'s `0x7046_0200` range.

use cimmeria_base_session::base::world_entry::lab_fixture::{StagedMission, StagedMissionState};

use super::tests::{
    active, apply, cleanup, fixture, mission_row, seed_mission, seed_player, MissionRow,
};
use crate::base::world_entry::methods::missions::state_lock;
use crate::test_support::require_db_or_skip;

const LETTER: i32 = 1360;
const STEP_A: i32 = 4037;
const STEP_B: i32 = 4038;
const AT_A: i32 = 0x7046_02F0;
const AT_COMPLETED: i32 = 0x7046_02F2;
const AT_LOCKED: i32 = 0x7046_02F4;
const AT_B: i32 = 0x7046_02F8;
const OVER_STALE: i32 = 0x7046_02FC;

/// **Guard: 1360 stages at step A with its objective 4650.** The generic row
/// carries no active objective, which the letter persist refuses
/// (`invalid_or_stale_snapshot`), so the apply failed with "was not written".
/// Drop the objective fill in `build_row` and this fails the same way.
#[tokio::test]
async fn live_db_fixture_stages_the_letter_mission_at_step_a() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, AT_A).await;

    let applied = apply(
        &pool,
        AT_A,
        &fixture(player_id, vec![active(LETTER, STEP_A)]),
    )
    .await;
    let row = mission_row(&pool, player_id, LETTER).await;
    cleanup(&pool, AT_A).await;

    assert_eq!(applied.expect("applied").missions, 1);
    assert_eq!(
        row.expect("the letter row"),
        MissionRow {
            status: 1,
            current_step_id: Some(STEP_A),
            completed_step_ids: Vec::new(),
            completed_objective_ids: Vec::new(),
            active_objective_ids: vec![4650],
            repeats: 0,
        }
    );
}

/// **Guard: 1360 stages at step B from a clean character**, where the
/// persist only moves B from a stored A: the fixture writes A first. Drop
/// the A-then-B sequence in `write_letter_mission` and the apply is refused.
#[tokio::test]
async fn live_db_fixture_stages_the_letter_mission_at_step_b() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, AT_B).await;

    let applied = apply(
        &pool,
        AT_B,
        &fixture(player_id, vec![active(LETTER, STEP_B)]),
    )
    .await;
    let row = mission_row(&pool, player_id, LETTER).await;
    cleanup(&pool, AT_B).await;

    assert_eq!(applied.expect("applied").missions, 1);
    assert_eq!(
        row.expect("the letter row"),
        MissionRow {
            status: 1,
            current_step_id: Some(STEP_B),
            completed_step_ids: vec![STEP_A],
            completed_objective_ids: vec![4650],
            active_objective_ids: vec![4651],
            repeats: 0,
        }
    );
}

/// **Guard: a stale 1360 row does not block staging.** A completed run
/// (status 2, repeats 1) is cleared first, so the character can be put
/// back at step A.
#[tokio::test]
async fn live_db_fixture_restages_the_letter_mission_over_a_stale_row() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, OVER_STALE).await;
    seed_mission(&pool, player_id, LETTER, 2, 1).await;

    let applied = apply(
        &pool,
        OVER_STALE,
        &fixture(player_id, vec![active(LETTER, STEP_A)]),
    )
    .await;
    let row = mission_row(&pool, player_id, LETTER).await;
    cleanup(&pool, OVER_STALE).await;

    assert_eq!(applied.expect("applied").missions, 1);
    let row = row.expect("the letter row");
    assert_eq!(
        (row.status, row.current_step_id, row.repeats),
        (1, Some(STEP_A), 0)
    );
}

/// **Guard: a locked player's saved 1360 row survives a fixture.** The reset
/// is a raw DELETE, so the lock check in `write_letter_mission` runs before
/// it. Remove that check and the DELETE drops the row, then the A write is
/// refused: the row is gone.
#[tokio::test]
async fn live_db_fixture_keeps_a_locked_letter_row() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, AT_LOCKED).await;
    seed_mission(&pool, player_id, LETTER, 1, 0).await;

    state_lock::lock(player_id, 1);
    let applied = apply(
        &pool,
        AT_LOCKED,
        &fixture(player_id, vec![active(LETTER, STEP_A)]),
    )
    .await;
    state_lock::clear(player_id);
    let row = mission_row(&pool, player_id, LETTER).await;
    cleanup(&pool, AT_LOCKED).await;

    let error = applied.expect_err("a locked player's letter write is refused");
    assert!(error.contains("was not written"), "{error}");
    assert_eq!(row.expect("the saved row survives").status, 1);
}

/// **Guard: 1360 cannot be staged as completed.** The refusal comes before
/// the letter reset, so the character's own 1360 row is left as it was.
/// Without it the row is deleted and the read-back fails.
#[tokio::test]
async fn live_db_fixture_refuses_a_completed_letter_mission() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, AT_COMPLETED).await;
    seed_mission(&pool, player_id, LETTER, 1, 0).await;
    let before = mission_row(&pool, player_id, LETTER).await;

    let completed = StagedMission {
        mission_id: LETTER,
        state: StagedMissionState::Completed,
    };
    let applied = apply(&pool, AT_COMPLETED, &fixture(player_id, vec![completed])).await;
    let row = mission_row(&pool, player_id, LETTER).await;
    cleanup(&pool, AT_COMPLETED).await;

    let error = applied.expect_err("a completed letter mission is refused");
    assert!(error.contains("cannot be staged as completed"), "{error}");
    assert_eq!(row, before, "the character's 1360 row is unchanged");
}
