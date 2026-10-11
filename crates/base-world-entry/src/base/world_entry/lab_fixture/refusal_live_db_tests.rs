//! Live-DB guards for the fixture apply's refusals (lab-fixtures FX-02
//! review): the precheck writes nothing, a refused mission write is
//! reported, and every "not applied" row carries a stable `reason`.
//!
//! Sentinels from `tests.rs`'s `0x7046_0200` block.

use std::sync::Arc;

use cimmeria_base_session::base::world_entry::lab_fixture::{
    fixture_pending, stage_fixture, GM_ACCESS_LEVEL,
};
use tracing::Level;

use super::apply_at_play;
use super::tests::{
    active, apply, cleanup, fixture, mission_row, place, seed_mission, seed_player, steps_of,
    LOG_ROWS, LOG_ROWS_STRANGER, MISSION, REFUSED_WRITE, UNKNOWN_WORLD,
};
use crate::base::world_entry::methods::missions::state_lock;
use crate::test_support::{require_db_or_skip, LogCapture};

/// A world name no seed will ever carry.
const NO_SUCH_WORLD: &str = "Fx02_No_Such_World";

/// **Guard: the world and position checks run before the first write.**
/// A valid mission with an unknown world, then with a NaN position: both
/// are refused and neither the mission row nor the character's place
/// changes. Without the precheck the mission is written before the
/// position part fails (unknown world) or the NaN lands in `pos_x`.
#[tokio::test]
async fn live_db_fixture_unknown_world_writes_nothing() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, UNKNOWN_WORLD).await;
    seed_mission(&pool, player_id, MISSION, 2, 1).await;
    let steps = steps_of(&pool, MISSION).await;
    let mission_before = mission_row(&pool, player_id, MISSION).await;
    let place_before = place(&pool, player_id).await;

    let mut bad_world = fixture(player_id, vec![active(MISSION, steps[1])]);
    bad_world.world = Some(NO_SUCH_WORLD.to_string());
    let world_result = apply(&pool, UNKNOWN_WORLD, &bad_world).await;
    let mission_after_world = mission_row(&pool, player_id, MISSION).await;
    let place_after_world = place(&pool, player_id).await;

    let mut nan = fixture(player_id, vec![active(MISSION, steps[1])]);
    nan.position = Some([f32::NAN, 70.0, 0.0]);
    let nan_result = apply(&pool, UNKNOWN_WORLD, &nan).await;
    let mission_after_nan = mission_row(&pool, player_id, MISSION).await;
    let place_after_nan = place(&pool, player_id).await;
    cleanup(&pool, UNKNOWN_WORLD).await;

    let error = world_result.expect_err("an unknown world is refused");
    assert!(error.contains(NO_SUCH_WORLD), "{error}");
    assert!(mission_before.is_some());
    assert_eq!(
        mission_after_world, mission_before,
        "unknown world: no mission write"
    );
    assert_eq!(place_after_world, place_before);

    let error = nan_result.expect_err("a NaN position is refused");
    assert!(error.contains("not finite"), "{error}");
    assert_eq!(
        mission_after_nan, mission_before,
        "NaN position: no mission write"
    );
    assert_eq!(place_after_nan, place_before);
}

/// **Guard: a mission write the persist refuses fails the fixture.**
/// `handle_mission_update` returns nothing, and TG-DB-12's
/// `refuse_persist_if_locked` drops the write for a player whose saved
/// missions are locked; only the read-back (`confirm_written`) sees it.
/// Remove the read-back and the apply reports success with no row.
#[tokio::test]
async fn live_db_fixture_reports_a_refused_mission_write() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, REFUSED_WRITE).await;
    let steps = steps_of(&pool, MISSION).await;

    state_lock::lock(player_id, 1);
    let applied = apply(
        &pool,
        REFUSED_WRITE,
        &fixture(player_id, vec![active(MISSION, steps[1])]),
    )
    .await;
    state_lock::clear(player_id);
    let row = mission_row(&pool, player_id, MISSION).await;
    cleanup(&pool, REFUSED_WRITE).await;

    let error = applied.expect_err("a refused mission write fails the fixture");
    assert!(error.contains("was not written"), "{error}");
    assert_eq!(row, None, "the persist refused the write");
}

/// **Guard (Rule 6): every refusal at Play logs a stable `reason`.** Five
/// rows, one Play each through `apply_at_play`: another account
/// (`account_mismatch`, and the fixture stays staged), a non-GM session
/// (`not_gm`), no pool (`no_database`), an unknown world
/// (`precheck_refused`) and a locked mission write (`write_failed`). The
/// store's `expired` row is pinned in its own unit test.
#[tokio::test]
async fn live_db_fixture_negative_log_rows_carry_a_stable_reason() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, LOG_ROWS).await;
    let steps = steps_of(&pool, MISSION).await;
    let db = Some(Arc::new(pool.clone()));
    let owner = LOG_ROWS as u32;
    let capture = LogCapture::install();

    // account_mismatch: the stranger leaves it staged for the owner.
    stage_fixture(fixture(player_id, Vec::new()));
    apply_at_play(
        &db,
        LOG_ROWS_STRANGER as u32,
        None,
        player_id,
        GM_ACCESS_LEVEL,
    )
    .await;
    let left_staged = fixture_pending(player_id);
    // not_gm: the owner's non-GM session takes and discards it.
    apply_at_play(&db, owner, None, player_id, 0).await;
    let not_gm_consumed = !fixture_pending(player_id);
    // no_database.
    stage_fixture(fixture(player_id, Vec::new()));
    apply_at_play(&None, owner, None, player_id, GM_ACCESS_LEVEL).await;
    // precheck_refused: an unknown world.
    let mut bad_world = fixture(player_id, Vec::new());
    bad_world.world = Some(NO_SUCH_WORLD.to_string());
    stage_fixture(bad_world);
    apply_at_play(&db, owner, None, player_id, GM_ACCESS_LEVEL).await;
    // write_failed: the persist refuses a locked player's mission row.
    stage_fixture(fixture(player_id, vec![active(MISSION, steps[1])]));
    state_lock::lock(player_id, 1);
    apply_at_play(&db, owner, None, player_id, GM_ACCESS_LEVEL).await;
    state_lock::clear(player_id);
    cleanup(&pool, LOG_ROWS).await;

    assert!(left_staged, "another account leaves the fixture staged");
    assert!(not_gm_consumed, "a non-GM session consumes the fixture");
    for (level, message, reason) in [
        (Level::WARN, "lab fixture left staged", "account_mismatch"),
        (Level::WARN, "lab fixture discarded", "not_gm"),
        (Level::WARN, "lab fixture discarded", "no_database"),
        (Level::ERROR, "lab fixture not applied", "precheck_refused"),
        (Level::ERROR, "lab fixture not applied", "write_failed"),
    ] {
        let row = capture
            .find_event(level, message, reason)
            .unwrap_or_else(|| panic!("no {level} {message:?} row with reason {reason}"));
        assert!(row.fields.contains_key("player_id"), "{reason}: player_id");
        assert!(
            row.fields.contains_key("player_name"),
            "{reason}: player_name"
        );
    }
}
