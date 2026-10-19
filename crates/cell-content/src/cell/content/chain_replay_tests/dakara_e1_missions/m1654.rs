//! DK-33: mission 1654 "Aftermath" (dakara_e1_m1654_chains.sql, 8430-8449).
//! Crossing C3: the Dakara DHD completes objective 5961 and teleports to SGC_W1.

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_mission_completed,
    fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

const MISSION: i32 = 1654;

/// A context in the tent (world 62) with 1653 completed and 1654 not yet taken.
fn offer(archetype: i64) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(STORY_RM)), 1653, "completed");
    with_mission(ctx, MISSION, "not_active")
}

/// A context at the Dakara DHD (world 61) with 1654 at step 4939, objective 5961 `status`.
fn at_dhd(archetype: i64, objective_5961: &str) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(DAKARA)), MISSION, "active");
    let ctx = with_step(ctx, MISSION, 4939, "active");
    with_objective(ctx, MISSION, 5961, objective_5961)
}

#[tokio::test]
async fn live_db_1653_report_offers_1654_in_the_tent() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_mission_completed(&engine, &offer(TAURI), 1653);
    assert_eq!(
        labels_in(&r, M1654_CHAINS),
        vec!["8430:add_dialog_set(6780@59)"]
    );
    assert_no_deferred_actions(&r, 8430);

    // Not on Dakara, and not once 1654 is taken.
    let on_dakara = with_mission(ctx_for(TAURI, Some(DAKARA)), 1653, "completed");
    let on_dakara = with_mission(on_dakara, MISSION, "not_active");
    let taken = with_mission(offer(TAURI), MISSION, "active");
    assert!(labels_in(
        &fire_mission_completed(&engine, &on_dakara, 1653),
        M1654_CHAINS
    )
    .is_empty());
    assert!(labels_in(&fire_mission_completed(&engine, &taken, 1653), M1654_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_loading_the_tent_restores_the_offer_until_taken() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &offer(TAURI), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1654_CHAINS),
        vec!["8431:add_dialog_set(6780@59)"]
    );
    assert_no_deferred_actions(&r, 8431);

    let taken = with_mission(offer(TAURI), MISSION, "active");
    let taken_r = fire_player_loaded(&engine, &taken, STORY_RM_NAME);
    assert!(labels_in(&taken_r, M1654_CHAINS).is_empty());
    let on_dakara = fire_player_loaded(&engine, &offer(TAURI), DAKARA_NAME);
    assert!(labels_in(&on_dakara, M1654_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_tauri_offer_takes_1654_with_the_tauri_briefing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // 6 is the highest Tau'ri archetype (lte 6), so it is the boundary.
    for archetype in [TAURI, 6] {
        let r = fire_dialog_choice(&engine, &offer(archetype), 5987);
        assert_eq!(
            labels_in(&r, M1654_CHAINS),
            vec![
                "8432:accept_mission(1654)",
                "8432:remove_dialog_set(6780@59)",
                "8432:display_dialog(5850)",
            ],
        );
        assert_eq!(display_dialog_count(&r), 1);
        assert_no_deferred_actions(&r, 8432);
    }
}

#[tokio::test]
async fn live_db_free_jaffa_offer_takes_1654_with_the_jaffa_briefing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [FREE_JAFFA, PRAXIS_JAFFA] {
        let r = fire_dialog_choice(&engine, &offer(archetype), 5987);
        assert_eq!(
            labels_in(&r, M1654_CHAINS),
            vec![
                "8433:accept_mission(1654)",
                "8433:remove_dialog_set(6780@59)",
                "8433:display_dialog(5851)",
            ],
        );
        assert_eq!(display_dialog_count(&r), 1);
        assert_no_deferred_actions(&r, 8433);
    }
}

#[tokio::test]
async fn live_db_offer_is_not_taken_outside_its_state() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let on_dakara = ctx_for(TAURI, Some(DAKARA));
    let on_dakara = with_mission(
        with_mission(on_dakara, 1653, "completed"),
        MISSION,
        "not_active",
    );
    let taken = with_mission(offer(TAURI), MISSION, "completed");
    let not_done = with_mission(ctx_for(TAURI, Some(STORY_RM)), 1653, "active");
    let not_done = with_mission(not_done, MISSION, "not_active");
    for ctx in [&on_dakara, &taken, &not_done] {
        let r = fire_dialog_choice(&engine, ctx, 5987);
        assert!(labels_in(&r, M1654_CHAINS).is_empty());
    }
}

#[tokio::test]
async fn live_db_dhd_completes_5961_then_teleports_to_sgc() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_dhd(TAURI, "active"), "Dakara_E1_DHD");
    assert_eq!(
        labels_in(&r, M1654_CHAINS),
        vec![
            "8434:complete_objective(1654/5961)",
            "8434:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 0);
    assert_no_deferred_actions(&r, 8434);
}

#[tokio::test]
async fn live_db_dhd_recovery_teleports_once_5961_is_done() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let done = at_dhd(TAURI, "completed");
    let r = fire_interact_tag(&engine, &done, "Dakara_E1_DHD");
    assert_eq!(
        labels_in(&r, M1654_CHAINS),
        vec!["8435:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)"],
    );
    assert_no_deferred_actions(&r, 8435);
}

#[tokio::test]
async fn live_db_dhd_needs_dakara_and_step_4939() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let on_story_rm = with_step(
        with_mission(ctx_for(TAURI, Some(STORY_RM)), MISSION, "active"),
        MISSION,
        4939,
        "active",
    );
    // 1654 already completed: its step 4939 is no longer active.
    let completed = with_mission(ctx_for(TAURI, Some(DAKARA)), MISSION, "completed");
    // DK-30b's step 4930 on 1651 is not 1654 at 4939: no 1654 chain may match it.
    let at_4930 = with_step(
        with_mission(ctx_for(TAURI, Some(DAKARA)), MISSION, "not_active"),
        1651,
        4930,
        "active",
    );
    for ctx in [&on_story_rm, &completed, &at_4930] {
        let r = fire_interact_tag(&engine, ctx, "Dakara_E1_DHD");
        assert!(labels_in(&r, M1654_CHAINS).is_empty());
    }
}
