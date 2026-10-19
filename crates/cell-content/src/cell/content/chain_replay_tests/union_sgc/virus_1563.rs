//! Mission 1563 "Virus" (US-12, chains 3051-3057). Carter offers 1563 ("?")
//! once a Human has finished 1562 (3051, shown by 3052). Accepting the offer
//! (dialog 5370, 3053) starts 1563 and turns her "?" into "!". Pressing her
//! again (3054) shows her notes (dialog 5371, display only), advances to step
//! 4629 and lights the terminal. Pressing the terminal (3055) shows 5372,
//! clears its glow and completes objective 5369, which completes 1563.
//! Re-login restores Carter's marker at 4628 (3056) and the terminal glow at
//! 4629 (3057). Every chain gates on the Union Human range, `archetype gte 1`
//! and `archetype lte 4`.

use sqlx::PgPool;

use super::rig::*;
use crate::cell::spawner::load_step_objectives;
use crate::test_support::require_db_or_skip;

const M1562: i32 = 1562;
const M1563: i32 = 1563;
const S4628: i32 = 4628;
const S4629: i32 = 4629;
const O5369: i32 = 5369;
const CARTER: &str = "SGC_W1_SamCarter";
const TERMINAL: &str = "SGC_W1_CarterTerminal";

/// A player of `archetype` who has just finished 1562, with its
/// `mission_completed` chains fired. The rig does not load the step-objective
/// cache (`rig.rs` is sealed), and `advance_step` needs it to put 5369 on the
/// roster of step 4629, so the helper loads it here.
async fn after_1562(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load step objectives");
    rig.accept(M1562).await;
    rig.finish(M1562).await;
    rig.sent();
    rig
}

/// **Guard: a Human who finished 1562 is offered 1563 and completes it
/// through Carter and her terminal.** Revert proof: delete chain 3053 (no
/// accept) and `status(1563)` stays `None` after `choose(5370, 1)`; delete
/// chain 3055 (the terminal never completes) and `objective_done(1563, 5369)`
/// stays false after the terminal press.
#[tokio::test]
async fn live_db_m1563_human_takes_the_virus_from_carter_and_completes_it() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let mut rig = after_1562(&pool, Some(archetype)).await;
        assert!(
            rig.flags(CARTER) & OFFER != 0,
            "archetype {archetype}: Carter shows '?' once 1562 is done"
        );

        rig.press(CARTER).await;
        assert!(
            rig.sent().contains(&Sent::Dialog(5370)),
            "archetype {archetype}: pressing Carter shows offer 5370"
        );
        assert_eq!(rig.status(M1563), None, "archetype {archetype}: not yet");

        rig.choose(5370, 1).await;
        assert_eq!(rig.status(M1563), ACTIVE, "archetype {archetype}: accepted");
        assert_eq!(rig.step(M1563), Some(S4628), "archetype {archetype}");
        assert!(
            rig.flags(CARTER) & STORY_ACTIVE != 0 && rig.flags(CARTER) & OFFER == 0,
            "archetype {archetype}: Carter shows '!' and no longer '?'"
        );

        rig.press(CARTER).await;
        assert!(
            rig.sent().contains(&Sent::Dialog(5371)),
            "archetype {archetype}: Carter hands over her notes (5371)"
        );
        assert_eq!(rig.step(M1563), Some(S4629), "archetype {archetype}");
        assert!(
            rig.flags(TERMINAL) & MISSION_OBJECT != 0,
            "archetype {archetype}: the terminal glows"
        );
        assert!(
            rig.flags(CARTER) & STORY_ACTIVE == 0,
            "archetype {archetype}: Carter's '!' is cleared"
        );

        rig.press(TERMINAL).await;
        assert!(
            rig.sent().contains(&Sent::Dialog(5372)),
            "archetype {archetype}: the terminal shows 5372"
        );
        assert_eq!(rig.status(M1563), COMPLETED, "archetype {archetype}");
        assert!(rig.objective_done(M1563, O5369), "archetype {archetype}");
        assert!(
            rig.flags(TERMINAL) & MISSION_OBJECT == 0,
            "archetype {archetype}: the terminal glow is cleared"
        );
    }
}

/// **Guard: a non-Human (and a player with no archetype) is never offered
/// 1563.** Revert proof: drop the `archetype gte '1'` row from chains 3051
/// and 3053; the unstamped player (reads -1) is then offered and accepts
/// 1563, which fails `status(1563) == None`. The `lte '4'` row alone still
/// excludes the four non-Humans.
#[tokio::test]
async fn live_db_m1563_is_never_offered_to_a_non_human() {
    let pool = require_db_or_skip!();
    let mut cases: Vec<Option<i32>> = NON_HUMANS.iter().map(|a| Some(*a)).collect();
    cases.push(UNSTAMPED);
    for archetype in cases {
        let mut rig = after_1562(&pool, archetype).await;
        assert_eq!(
            rig.flags(CARTER) & OFFER,
            0,
            "{archetype:?}: Carter shows no '?'"
        );
        rig.press(CARTER).await;
        rig.choose(5370, 1).await;
        assert!(
            !rig.sent().contains(&Sent::Dialog(5370)),
            "{archetype:?}: offer 5370 is never shown"
        );
        assert_eq!(rig.status(M1563), None, "{archetype:?}: never accepted");
    }
}

/// **Guard: the terminal does nothing before Carter hands over her notes.**
/// Revert proof: drop the `step_status 1563 '4629'` row from chain 3055; the
/// terminal press at 4628 then shows 5372 and completes 5369, which fails
/// `step == Some(4628)`.
#[tokio::test]
async fn live_db_m1563_terminal_does_nothing_before_carter_hands_over_her_notes() {
    let pool = require_db_or_skip!();
    let mut rig = after_1562(&pool, Some(SOLDIER)).await;
    rig.press(CARTER).await;
    rig.choose(5370, 1).await;
    rig.sent();
    rig.press(TERMINAL).await;
    let sent = rig.sent();
    assert_eq!(rig.step(M1563), Some(S4628), "still at 4628");
    assert!(!rig.objective_done(M1563, O5369), "5369 is not done");
    assert!(!sent.contains(&Sent::Dialog(5372)), "5372 is not shown");
}

/// **Guard: a press before the offer is accepted re-shows the offer.**
/// Revert proof: change chain 3052 to need `mission_status 1563 eq 'active'`;
/// the first press then shows nothing, so `shown == 2` fails with `shown == 0`.
#[tokio::test]
async fn live_db_m1563_a_repress_before_accepting_reshows_the_offer() {
    let pool = require_db_or_skip!();
    let mut rig = after_1562(&pool, Some(SOLDIER)).await;
    rig.press(CARTER).await;
    rig.press(CARTER).await;
    let shown = rig
        .sent()
        .iter()
        .filter(|s| **s == Sent::Dialog(5370))
        .count();
    assert_eq!(shown, 2, "both presses show offer 5370");
    assert_eq!(rig.status(M1563), None, "the offer is still unaccepted");
}

/// **Guard: every 1563 marker comes back on relog.** Revert proof: delete
/// chain 3056 and Carter's `!` is missing after the first relog
/// (`STORY_ACTIVE` assertion); delete chain 3057 and the terminal glow is
/// missing after the second; delete the `player_loaded` row of chain 3051 and
/// Carter's `?` is missing after the third.
#[tokio::test]
async fn live_db_m1563_markers_come_back_on_relog() {
    let pool = require_db_or_skip!();

    let mut rig = after_1562(&pool, Some(SOLDIER)).await;
    rig.press(CARTER).await;
    rig.choose(5370, 1).await;
    let rig = rig.relog(&pool).await;
    assert!(
        rig.flags(CARTER) & STORY_ACTIVE != 0,
        "Carter's '!' comes back at 4628"
    );

    let mut rig = after_1562(&pool, Some(SOLDIER)).await;
    rig.press(CARTER).await;
    rig.choose(5370, 1).await;
    rig.press(CARTER).await;
    let rig = rig.relog(&pool).await;
    assert_eq!(rig.step(M1563), Some(S4629), "the step survives relog");
    assert!(
        rig.flags(TERMINAL) & MISSION_OBJECT != 0,
        "the terminal glow comes back at 4629"
    );

    let rig = after_1562(&pool, Some(SOLDIER)).await.relog(&pool).await;
    assert!(
        rig.flags(CARTER) & OFFER != 0,
        "Carter's '?' comes back before the offer is taken"
    );
}

/// **Guard: every 1563 chain carries the Union Human gate and the world
/// check.** Revert proof: drop any `archetype` row, or change `lte '4'` to
/// `lt '5'`, and the per-chain pairs differ; drop the `world 58` row of 3051,
/// 3056 or 3057 and the world list is short.
#[tokio::test]
async fn live_db_m1563_chains_carry_their_guards() {
    let pool = require_db_or_skip!();
    let archetype_rows: Vec<(i32, String, Option<String>)> = sqlx::query_as(
        "SELECT chain_id, operator, value FROM resources.content_conditions \
         WHERE condition_type = 'archetype' AND chain_id BETWEEN 3051 AND 3057 \
         ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("archetype rows read");
    for chain in 3051..=3057 {
        let pairs: Vec<(String, Option<String>)> = archetype_rows
            .iter()
            .filter(|r| r.0 == chain)
            .map(|r| (r.1.clone(), r.2.clone()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("gte".to_string(), Some("1".to_string())),
                ("lte".to_string(), Some("4".to_string())),
            ],
            "chain {chain} gates on the Union Human range 1-4"
        );
    }

    let worlds: Vec<(i32,)> = sqlx::query_as(
        "SELECT chain_id FROM resources.content_conditions \
         WHERE condition_type = 'world' AND target_id = 58 \
           AND chain_id BETWEEN 3051 AND 3057 ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("world rows read");
    assert_eq!(
        worlds,
        vec![(3051,), (3056,), (3057,)],
        "3051, 3056 and 3057 are world-58 only"
    );
}
