//! Live-DB guards for staging mission 1360 (Frost's Letter), whose persist
//! accepts only two exact active snapshots (see `missions.rs`).
//!
//! Sentinel slots `0x7046_02F0`, `02F8`, `02FC` from `tests.rs`'s `0x7046_0200` range.

use super::tests::{
    active, apply, cleanup, fixture, mission_row, seed_mission, seed_player, MissionRow,
};
use crate::test_support::require_db_or_skip;

const LETTER: i32 = 1360;
const STEP_A: i32 = 4037;
const STEP_B: i32 = 4038;
const AT_A: i32 = 0x7046_02F0;
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

    let applied = apply(&pool, AT_A, &fixture(player_id, vec![active(LETTER, STEP_A)])).await;
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

    let applied = apply(&pool, AT_B, &fixture(player_id, vec![active(LETTER, STEP_B)])).await;
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
    assert_eq!((row.status, row.current_step_id, row.repeats), (1, Some(STEP_A), 0));
}
