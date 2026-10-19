//! DK-13: mission 1648 "Loth'ta's Withdrawal" (dakara_e1_m1648_chains.sql, 8180-8199).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_mission_completed,
    fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// Bra'tac's tag (template 59, world 62).
const BRATAC_TAG: &str = "Dakara_E1_StoryRm_Bratac";
/// Loth'ta's tag (template 440, world 61).
const LOTH_TA_TAG: &str = "Dakara_E1_Lothta";
/// Bra'tac's offer blurb (dsm 6752) and its choice.
const OFFER_CHOICE: i32 = 5983;

/// A context in which 1647 is completed, the state every 1648 chain assumes.
fn base(archetype: i64, world: Option<i32>) -> ExecutionContext {
    with_mission(ctx_for(archetype, world), 1647, "completed")
}

/// A context in which 1648 is offered: 1647 completed, 1648 not yet active.
fn offered(archetype: i64, world: Option<i32>) -> ExecutionContext {
    with_mission(base(archetype, world), 1648, "not_active")
}

/// A context with 1648 active at `step`.
fn at_step(archetype: i64, world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(base(archetype, world), 1648, "active"),
        1648,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_offer_completion_in_tent_binds_brat_ac() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_mission_completed(&engine, &base(FREE_JAFFA, Some(STORY_RM)), 1647);
    assert_eq!(
        labels_in(&r, M1648_CHAINS),
        vec!["8180:add_dialog_set(6752@59)"],
    );
    // 1647 completed on Dakara_E1 is bound by 8181 on entering the tent instead.
    let r = fire_mission_completed(&engine, &base(FREE_JAFFA, Some(DAKARA)), 1647);
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_offer_restore_binds_brat_ac_on_tent_entry() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &offered(FREE_JAFFA, Some(STORY_RM)), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1648_CHAINS),
        vec!["8181:add_dialog_set(6752@59)"],
    );
    // Once 1648 is active the offer is no longer restored.
    let r = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4919),
        STORY_RM_NAME,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
    let r = fire_player_loaded(&engine, &offered(FREE_JAFFA, Some(DAKARA)), DAKARA_NAME);
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_offer_choice_accepts_1648_and_unbinds() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_dialog_choice(&engine, &offered(FREE_JAFFA, Some(STORY_RM)), OFFER_CHOICE);
    assert_eq!(
        labels_in(&r, M1648_CHAINS),
        vec![
            "8182:accept_mission(1648)",
            "8182:display_dialog(5828)",
            "8182:remove_dialog_set(6752@59)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8182);
    // Not before 1647 is completed, and not once 1648 is active.
    let before_1647 = with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1647, "active");
    let r = fire_dialog_choice(&engine, &before_1647, OFFER_CHOICE);
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
    let r = fire_dialog_choice(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4919),
        OFFER_CHOICE,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_loth_ta_bind_at_4919_on_dakara() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4919),
        DAKARA_NAME,
    );
    assert_eq!(
        labels_in(&r, M1648_CHAINS),
        vec!["8183:add_dialog_set(6754@440)"],
    );
    // Bra'tac's world, and step 4918, bind nothing here.
    let r = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4919),
        STORY_RM_NAME,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
    let r = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4918),
        DAKARA_NAME,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_loth_ta_click_at_4919_refuses_and_advances() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let at_4919 = at_step(FREE_JAFFA, Some(DAKARA), 4919);
    let r = fire_interact_tag(&engine, &at_4919, LOTH_TA_TAG);
    assert_eq!(
        labels_in(&r, M1648_CHAINS),
        vec![
            "8184:display_dialog(5829)",
            "8184:add_item(6781x1)",
            "8184:advance_step(1648->4918)",
            "8184:remove_dialog_set(6754@440)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8184);
    // The wrong step and the wrong world both leave Loth'ta silent.
    let r = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4918),
        LOTH_TA_TAG,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
    let r = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4919),
        LOTH_TA_TAG,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_brat_ac_bind_at_4918_in_the_tent() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4918),
        STORY_RM_NAME,
    );
    assert_eq!(
        labels_in(&r, M1648_CHAINS),
        vec!["8185:add_dialog_set(6755@59)"],
    );
    // Step 4919 and the drops' world bind nothing here.
    let r = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4919),
        STORY_RM_NAME,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
    let r = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4918),
        DAKARA_NAME,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_brat_ac_turn_in_at_4918_completes_1648() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let at_4918 = at_step(FREE_JAFFA, Some(STORY_RM), 4918);
    let r = fire_interact_tag(&engine, &at_4918, BRATAC_TAG);
    assert_eq!(
        labels_in(&r, M1648_CHAINS),
        vec![
            "8186:display_dialog(5830)",
            "8186:remove_item(6781x1)",
            "8186:remove_dialog_set(6755@59)",
            "8186:complete_mission(1648)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8186);
    // Wrong world, wrong step, and a completed 1648 all give nothing.
    let r = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4918),
        BRATAC_TAG,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
    let r = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4919),
        BRATAC_TAG,
    );
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
    let done = with_mission(base(FREE_JAFFA, Some(STORY_RM)), 1648, "completed");
    let r = fire_interact_tag(&engine, &done, BRATAC_TAG);
    assert!(labels_in(&r, M1648_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_tau_ri_gets_the_same_chains_as_a_free_jaffa() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // 1648 has no archetype split: the Tau'ri archetype fires every chain the
    // Free Jaffa archetype does.
    let jaffa = fire_mission_completed(&engine, &base(FREE_JAFFA, Some(STORY_RM)), 1647);
    let tauri = fire_mission_completed(&engine, &base(TAURI, Some(STORY_RM)), 1647);
    assert_eq!(
        labels_in(&tauri, M1648_CHAINS),
        labels_in(&jaffa, M1648_CHAINS)
    );
    let jaffa = fire_dialog_choice(&engine, &offered(FREE_JAFFA, Some(STORY_RM)), OFFER_CHOICE);
    let tauri = fire_dialog_choice(&engine, &offered(TAURI, Some(STORY_RM)), OFFER_CHOICE);
    assert_eq!(
        labels_in(&tauri, M1648_CHAINS),
        labels_in(&jaffa, M1648_CHAINS)
    );
    let jaffa = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4918),
        BRATAC_TAG,
    );
    let tauri = fire_interact_tag(&engine, &at_step(TAURI, Some(STORY_RM), 4918), BRATAC_TAG);
    assert_eq!(
        labels_in(&tauri, M1648_CHAINS),
        labels_in(&jaffa, M1648_CHAINS)
    );
    assert!(!labels_in(&tauri, M1648_CHAINS).is_empty());
}
