//! Mission 1571 auxiliary credit (US-13, chains 3080-3081). Mission 1571 is
//! the auxiliary record of the 1559 armory pickup: a Human who completes 1559
//! is credited 1571 at once (objective 5354, no second weapon), and a Human who
//! finished 1559 before the chain existed is credited on load. The hidden
//! optional objectives 5355 and 5356 stay uncredited (HELD-A).

use super::rig::*;
use crate::test_support::require_db_or_skip;

/// The required objective of 1571 (step 4616); 5355 and 5356 are optional.
const REQUIRED: i32 = 5354;
const OPTIONAL: [i32; 2] = [5355, 5356];

/// Every Human archetype completes 1559 and 1571 is credited once, with no
/// `Item` sent (the weapon came from 1559). Revert: delete chain 3080, then
/// 1571 stays ACTIVE or unaccepted and the test fails.
#[tokio::test]
async fn live_db_m1571_human_is_credited_when_1559_completes() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let mut rig = Rig::new(&pool, Some(archetype)).await;
        rig.accept(1559).await;
        rig.finish(1559).await;
        assert_eq!(
            rig.status(1571),
            COMPLETED,
            "archetype {archetype}: 1571 completes with 1559"
        );
        assert!(
            rig.objective_done(1571, REQUIRED),
            "archetype {archetype}: objective 5354 is credited"
        );
        assert!(
            !rig.sent().iter().any(|s| matches!(s, Sent::Item(..))),
            "archetype {archetype}: no second weapon is granted"
        );
    }
}

/// A Soldier who finished 1559 before chain 3080 existed is credited 1571 on
/// load. Revert: delete chain 3081, then the load leaves 1571 unaccepted.
#[tokio::test]
async fn live_db_m1571_catch_up_on_load_for_an_old_character() {
    let pool = require_db_or_skip!();
    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    rig.accept(1559).await;
    rig.complete(1559).await;
    rig.loaded().await;
    assert_eq!(
        rig.status(1571),
        COMPLETED,
        "the player_loaded catch-up credits 1571"
    );
    assert!(rig.objective_done(1571, REQUIRED));
}

/// A Soldier is credited 1571 once: a second load after the credit does not
/// accept it again and grants no item. Revert: drop the `not_active` row of
/// chain 3081 (or 3080), and the repeated accept re-opens 1571.
#[tokio::test]
async fn live_db_m1571_is_credited_once() {
    let pool = require_db_or_skip!();
    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    rig.accept(1559).await;
    rig.finish(1559).await;
    rig.loaded().await;
    rig.loaded().await;
    assert_eq!(
        rig.status(1571),
        COMPLETED,
        "1571 stays completed across loads"
    );
    assert!(
        !rig.sent().iter().any(|s| matches!(s, Sent::Item(..))),
        "no item is granted on a repeated load"
    );
    for optional in OPTIONAL {
        assert!(
            !rig.objective_done(1571, optional),
            "hidden optional objective {optional} stays uncredited"
        );
    }
}

/// A Soldier who already holds 1571 is left alone by both credit chains: the
/// offer guard refuses a second accept on its own, but `complete_objective`
/// would still run, so only the `not_active` rows keep 5354 uncredited.
/// Revert: drop the `not_active` row of chain 3080 (the `finish(1559)` half)
/// or of chain 3081 (the `loaded()` half), and 5354 is credited.
#[tokio::test]
async fn live_db_m1571_credit_chains_leave_a_held_1571_alone() {
    let pool = require_db_or_skip!();

    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    rig.accept(1559).await;
    rig.accept(1571).await;
    rig.finish(1559).await;
    assert_eq!(rig.status(1571), ACTIVE, "3080 skips a held 1571");
    assert!(!rig.objective_done(1571, REQUIRED), "3080 credits nothing");

    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    rig.accept(1559).await;
    rig.complete(1559).await;
    rig.accept(1571).await;
    rig.loaded().await;
    assert_eq!(rig.status(1571), ACTIVE, "3081 skips a held 1571");
    assert!(!rig.objective_done(1571, REQUIRED), "3081 credits nothing");
}

/// Non-Humans and an unstamped character never reach 1571, whether they finish
/// 1559 live or load after it. Revert: drop the `archetype gte '1'` rows; the
/// UNSTAMPED case (reads -1) then fails.
#[tokio::test]
async fn live_db_m1571_never_reaches_a_non_human() {
    let pool = require_db_or_skip!();
    let archetypes: Vec<Option<i32>> = NON_HUMANS
        .iter()
        .map(|&a| Some(a))
        .chain([UNSTAMPED])
        .collect();
    for archetype in archetypes {
        let mut rig = Rig::new(&pool, archetype).await;
        rig.accept(1559).await;
        rig.finish(1559).await;
        rig.loaded().await;
        assert_eq!(
            rig.status(1571),
            None,
            "archetype {archetype:?}: 1571 is never offered or credited"
        );
    }
}
