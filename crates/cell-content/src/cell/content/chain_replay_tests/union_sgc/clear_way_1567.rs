//! Mission 1567 Clear the Way (US-21, chains 3095-3099): the offer on 1566,
//! the five route Jaffa kill trigger, the accept backstop, Carter's line and
//! the relog marker. Seeds: `sgc_w1_union_1567_chains.sql`.

use sqlx::PgPool;

use super::rig::*;
use crate::test_support::require_db_or_skip;

const CARTER: &str = "SGC_W1_SamCarter";

/// The five route Jaffa of spawnlist rows 59-63 (US-11, D-US10 option B).
const CLEAR: [&str; 5] = [
    "SGC_W1_ClearJaffa1",
    "SGC_W1_ClearJaffa2",
    "SGC_W1_ClearJaffa3",
    "SGC_W1_ClearJaffa4",
    "SGC_W1_ClearJaffa5",
];

/// A player who has just completed 1566: 1564 on step 4631, 1566 accepted,
/// then 1566 finished through the live engine so `mission_completed 1566`
/// fires chain 3095.
async fn on_1566_done(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.accept(1564).await;
    rig.advance(1564, 4631).await;
    rig.accept(1566).await;
    rig.finish(1566).await;
    rig.sent();
    rig
}

/// **Guard: 1567 is offered when 1566 completes, to every Human archetype.**
/// Revert proof: delete chain 3095; the offer never fires and `status(1567)`
/// stays `None`.
#[tokio::test]
async fn live_db_m1567_offered_when_1566_completes() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let rig = on_1566_done(&pool, Some(archetype)).await;
        assert_eq!(
            rig.status(1567),
            ACTIVE,
            "archetype {archetype} is offered 1567 when 1566 completes"
        );
    }
}

/// **Guard: 1567 completes only when all five route Jaffa are dead.**
/// Revert proof: delete chain 3096, or drop the `ClearJaffa5` `entity_tag_state`
/// row of 3096; the first assertion then fails or the completion never comes.
#[tokio::test]
async fn live_db_m1567_needs_all_five_dead() {
    let pool = require_db_or_skip!();
    let mut rig = on_1566_done(&pool, Some(SOLDIER)).await;
    assert_eq!(rig.status(1567), ACTIVE);
    for tag in &CLEAR[..4] {
        rig.kill(tag).await;
    }
    assert_eq!(
        rig.status(1567),
        ACTIVE,
        "four dead Jaffa do not finish 1567"
    );
    rig.kill(CLEAR[4]).await;
    assert!(rig.objective_done(1567, 5377), "5377 is credited");
    assert_eq!(rig.status(1567), COMPLETED);
}

/// **Guard: a 1567 accepted when the five route Jaffa are already dead
/// completes on accept.** Revert proof: delete chain 3097; the mission stays
/// active after `finish(1566)` because nothing else credits 5377.
#[tokio::test]
async fn live_db_m1567_backstop_completes_on_accept_when_already_clear() {
    let pool = require_db_or_skip!();
    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    rig.accept(1564).await;
    rig.advance(1564, 4631).await;
    for tag in CLEAR {
        rig.kill(tag).await;
    }
    rig.accept(1566).await;
    rig.finish(1566).await;
    assert_eq!(rig.status(1567), COMPLETED);
}

/// **Guard: Carter's line 5381 plays on a press during 1567, and the relog
/// restores her story marker.** Revert proof: delete chain 3098 (no
/// `Dialog(5381)`) or chain 3099 (the flag is zero after the relog).
#[tokio::test]
async fn live_db_m1567_carter_line_and_relog_marker() {
    let pool = require_db_or_skip!();
    let mut rig = on_1566_done(&pool, Some(SOLDIER)).await;
    assert_eq!(rig.status(1567), ACTIVE);
    assert!(rig.press(CARTER).await, "Carter's press matches a chain");
    assert!(
        rig.sent().contains(&Sent::Dialog(5381)),
        "Carter shows 5381 during 1567"
    );
    let rig = rig.relog(&pool).await;
    assert_ne!(
        rig.flags(CARTER) & STORY_ACTIVE,
        0,
        "the relog restores Carter's story marker"
    );
}

/// **Guard: Carter's line 5381 needs 1564 on step 4631.** With 1567 active but
/// 1564 already past it, a press shows nothing. Revert proof: drop the
/// `step_status 1564 '4631'` row of chain 3098 and the press shows 5381.
#[tokio::test]
async fn live_db_m1567_carter_line_needs_1564_on_step_4631() {
    let pool = require_db_or_skip!();
    let mut rig = on_1566_done(&pool, Some(SOLDIER)).await;
    assert_eq!(rig.status(1567), ACTIVE);
    rig.advance(1564, 4632).await;
    rig.press(CARTER).await;
    assert!(
        !rig.sent().contains(&Sent::Dialog(5381)),
        "5381 is not shown once 1564 has left step 4631"
    );
}

/// **Guard: 1567 is never offered to a non-Human, or to an unstamped player.**
/// Revert proof: drop the `archetype gte '1'` row of chain 3095; the unstamped
/// player (reads -1) is then offered 1567. The four non-Humans stay refused by
/// the `lte '4'` row alone.
#[tokio::test]
async fn live_db_m1567_never_offered_to_a_non_human() {
    let pool = require_db_or_skip!();
    let cases: Vec<Option<i32>> = NON_HUMANS
        .iter()
        .map(|&archetype| Some(archetype))
        .chain([UNSTAMPED])
        .collect();
    for archetype in cases {
        let rig = on_1566_done(&pool, archetype).await;
        assert_eq!(
            rig.status(1567),
            None,
            "archetype {archetype:?} is never offered 1567"
        );
    }
}
