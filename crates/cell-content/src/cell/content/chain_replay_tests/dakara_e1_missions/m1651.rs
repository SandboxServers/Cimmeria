//! DK-30a: mission 1651 "Renewed Attack" (dakara_e1_m1651_chains.sql, 8290-8309).
//!
//! 1651 is accepted by the 1650 turn-in in the tent (8290, D-DK26), which binds
//! Bra'tac's status-report offer. The offer is restored on re-entry at 4932
//! (8291), and the report (8292) advances to 4930, the DHD to the SGC (DK-30b).
//! The SGC steps (4928, 4931, 4929) are not this packet's. On the return at 5171
//! the player arrives on Dakara_E1 and the report binds again (8293); the report
//! (8294) unbinds it and completes 1651.

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_mission_completed, fire_player_loaded,
    with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// 1650 done and 1651 not started: the state in which 1651 is accepted.
fn accept_state(world: Option<i32>) -> ExecutionContext {
    with_mission(
        with_mission(ctx_for(FREE_JAFFA, world), 1650, "completed"),
        1651,
        "not_active",
    )
}

/// 1651 accepted and at `step`.
fn at_step(world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(accept_state(world), 1651, "active"),
        1651,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_1650_done_accepts_1651_and_binds_brattac() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_mission_completed(&engine, &accept_state(Some(STORY_RM)), 1650);
    assert_eq!(
        labels_in(&r, M1651_CHAINS),
        vec!["8290:accept_mission(1651)", "8290:add_dialog_set(6772@59)"],
    );
    assert_no_deferred_actions(&r, 8290);

    // Wrong world (1650 finished out on Dakara) or 1651 already active.
    let outside = fire_mission_completed(&engine, &accept_state(Some(DAKARA)), 1650);
    assert!(labels_in(&outside, M1651_CHAINS).is_empty());
    let accepted = with_mission(accept_state(Some(STORY_RM)), 1651, "active");
    assert!(labels_in(
        &fire_mission_completed(&engine, &accepted, 1650),
        M1651_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_entering_the_tent_at_4932_restores_the_report_bind() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4932), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1651_CHAINS),
        vec!["8291:add_dialog_set(6772@59)"],
    );

    // Past the report (4930), or the world is not 62.
    let at_4930 = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4930), STORY_RM_NAME);
    assert!(labels_in(&at_4930, M1651_CHAINS).is_empty());
    let wrong_world = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4932), DAKARA_NAME);
    assert!(labels_in(&wrong_world, M1651_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_status_report_at_4932_advances_to_4930() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(
        &engine,
        &at_step(Some(STORY_RM), 4932),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert_eq!(
        labels_in(&r, M1651_CHAINS),
        vec![
            "8292:display_dialog(5843)",
            "8292:advance_step(1651->4930)",
            "8292:remove_dialog_set(6772@59)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8292);

    // Already at 4930 (the report is done), or the world is not 62.
    let at_4930 = fire_interact_tag(
        &engine,
        &at_step(Some(STORY_RM), 4930),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert!(labels_in(&at_4930, M1651_CHAINS).is_empty());
    let wrong_world = fire_interact_tag(
        &engine,
        &at_step(Some(DAKARA), 4932),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert!(labels_in(&wrong_world, M1651_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_entering_the_tent_at_5171_binds_the_return_report() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 5171), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1651_CHAINS),
        vec!["8293:add_dialog_set(6775@59)"],
    );

    // The SGC steps (4928, 4931, 4929) and the DHD step (4930) bind nothing.
    for step in [4928, 4929, 4930, 4931] {
        let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), step), STORY_RM_NAME);
        assert!(labels_in(&r, M1651_CHAINS).is_empty(), "step {step}");
    }
}

#[tokio::test]
async fn live_db_return_report_at_5171_completes_1651() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(
        &engine,
        &at_step(Some(STORY_RM), 5171),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert_eq!(
        labels_in(&r, M1651_CHAINS),
        vec![
            "8294:display_dialog(5847)",
            "8294:remove_dialog_set(6775@59)",
            "8294:complete_mission(1651)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8294);

    // Still at 4932 (the status report not yet made, so 8292 answers instead),
    // or the world is not 62.
    let at_4932 = fire_interact_tag(
        &engine,
        &at_step(Some(STORY_RM), 4932),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert!(labels_in(&at_4932, 8294..=8294).is_empty());
    let wrong_world = fire_interact_tag(
        &engine,
        &at_step(Some(DAKARA), 5171),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert!(labels_in(&wrong_world, M1651_CHAINS).is_empty());
}
