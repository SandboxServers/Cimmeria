//! Mission 568 executor walk (T-03a3): drives the real executor through the
//! Copplemann accept, the food-unit props and turn-in, and asserts what the
//! executor does with the chains. The chain shapes are pinned in `props.rs`
//! and `copplemann.rs`; this file pins the state transitions.
//!
//! The auto-complete trap: `complete_objective` completes mission 568 once
//! every non-optional active objective is done, so the Core's double advance
//! must not reach 2311 through `complete_objective` (stop rule 5).

use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED};

use super::rig::{rig, Rig, PLAYER};
use crate::test_support::require_db_or_skip;

/// Status, step, completed objectives and slot-420 binds of `PLAYER`.
fn state(r: &Rig) -> (Option<i8>, Option<i32>, Vec<i32>, Vec<i32>) {
    (
        r.status(PLAYER),
        r.step(PLAYER),
        r.completed(PLAYER),
        r.binds(PLAYER),
    )
}

/// `PLAYER` entered Tollana, clicked the Copplemann and closed 2153: 568 is
/// active on step 1964.
async fn accepted(pool: &sqlx::PgPool) -> Rig {
    let mut r = rig(pool).await;
    r.enter(PLAYER).await;
    assert!(r.click(PLAYER, "Tollana_Copplemann").await);
    r.close(PLAYER, 2153).await;
    assert_eq!(r.status(PLAYER), Some(MISSION_ACTIVE), "2153 accepts 568");
    assert_eq!(r.step(PLAYER), Some(1964));
    r
}

#[tokio::test]
async fn live_db_568_reclose_2153_when_active_is_a_no_op() {
    let pool = require_db_or_skip!();
    let mut r = accepted(&pool).await;
    let before = state(&r);
    r.close(PLAYER, 2153).await;
    assert_eq!(
        state(&r),
        before,
        "re-closing 2153 while active changes nothing"
    );
}

#[tokio::test]
async fn live_db_568_core_at_1964_reaches_1966() {
    let pool = require_db_or_skip!();
    let mut r = accepted(&pool).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    assert_eq!(
        r.status(PLAYER),
        Some(MISSION_ACTIVE),
        "the Core walk must not auto-complete 568"
    );
    assert_eq!(r.step(PLAYER), Some(1966));
    let done = r.completed(PLAYER);
    // Each advance_step force-completes the old step's objectives.
    assert!(done.contains(&2310), "2310 done by 1964->1965: {done:?}");
    assert!(done.contains(&2311), "2311 done by 1965->1966: {done:?}");
    // Observed: 2312 (optional) is also in `completed`, force-completed by the
    // 1965->1966 advance. Recorded in the report, not asserted.
    assert_eq!(r.binds(PLAYER), vec![6393], "turn-in bind after the Core");
}

#[tokio::test]
async fn live_db_568_core_at_1965_reaches_1966() {
    let pool = require_db_or_skip!();
    let mut r = accepted(&pool).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Fungus").await);
    assert_eq!(r.step(PLAYER), Some(1965));
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    assert_eq!(r.step(PLAYER), Some(1966));
    assert_eq!(r.binds(PLAYER), vec![6393]);
    assert_eq!(r.status(PLAYER), Some(MISSION_ACTIVE));
}

#[tokio::test]
async fn live_db_568_nest_credits_2312_once() {
    let pool = require_db_or_skip!();
    let mut r = accepted(&pool).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Nest").await);
    assert_eq!(r.step(PLAYER), Some(1965));
    let done = r.completed(PLAYER);
    assert!(done.contains(&2312), "Nest credits optional 2312: {done:?}");
    assert!(!done.contains(&2311), "Nest does not finish 1965: {done:?}");
    assert_eq!(r.status(PLAYER), Some(MISSION_ACTIVE));
    let before = state(&r);
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Nest").await);
    assert_eq!(state(&r), before, "a second Nest click changes nothing");
}

#[tokio::test]
async fn live_db_568_fungus_advances_1964_only() {
    let pool = require_db_or_skip!();
    let mut r = accepted(&pool).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Fungus").await);
    assert_eq!(r.step(PLAYER), Some(1965));
    let done = r.completed(PLAYER);
    assert!(!done.contains(&2311), "Fungus leaves 2311 open: {done:?}");
    assert!(
        !done.contains(&2312),
        "Fungus does not credit 2312: {done:?}"
    );
    let before = state(&r);
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Fungus").await);
    assert_eq!(state(&r), before, "a second Fungus click changes nothing");
}

#[tokio::test]
async fn live_db_568_turn_in_completes_and_unbinds() {
    let pool = require_db_or_skip!();
    let mut r = accepted(&pool).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    assert!(r.click(PLAYER, "Tollana_Copplemann").await);
    r.close(PLAYER, 2154).await;
    assert_eq!(r.status(PLAYER), Some(MISSION_COMPLETED));
    assert!(r.binds(PLAYER).is_empty(), "turn-in unbinds slot 420");
}

#[tokio::test]
async fn live_db_568_done_state_is_silent() {
    let pool = require_db_or_skip!();
    let mut r = accepted(&pool).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    assert!(r.click(PLAYER, "Tollana_Copplemann").await);
    r.close(PLAYER, 2154).await;
    assert!(
        !r.click(PLAYER, "Tollana_Copplemann").await,
        "Done fires no Copplemann chain"
    );
    for tag in [
        "Tollana_FoodUnit_Core",
        "Tollana_FoodUnit_Nest",
        "Tollana_FoodUnit_Fungus",
    ] {
        r.click(PLAYER, tag).await;
        assert_eq!(
            r.status(PLAYER),
            Some(MISSION_COMPLETED),
            "{tag} keeps 568 completed"
        );
        assert!(r.binds(PLAYER).is_empty(), "{tag} adds no bind in Done");
    }
}
