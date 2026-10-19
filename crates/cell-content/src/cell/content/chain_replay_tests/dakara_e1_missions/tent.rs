//! DK-10b: mission 1570 steps 4906 and 4904 (dakara_e1_m1570_tent_chains.sql, 8045-8059).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// A player on 1570 at `step`, mission and step both `active`.
fn at_step(archetype: i64, world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(archetype, world), 1570, "active"),
        1570,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_free_jaffa_arrival_gives_no_tent_chain() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // Nothing set: 1570 has not started, so no tent bind on either load.
    let ctx = ctx_for(FREE_JAFFA, Some(DAKARA));
    let dakara = fire_player_loaded(&engine, &ctx, DAKARA_NAME);
    let story_rm = fire_player_loaded(&engine, &ctx, STORY_RM_NAME);
    assert!(labels_in(&dakara, TENT_CHAINS).is_empty());
    assert!(labels_in(&story_rm, TENT_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_raknor_restore_at_4906_follows_archetype() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let jaffa = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4906),
        DAKARA_NAME,
    );
    assert_eq!(
        labels_in(&jaffa, TENT_CHAINS),
        vec!["8047:add_dialog_set(6890@441)"],
    );
    assert_no_deferred_actions(&jaffa, 8047);

    let praxis = fire_player_loaded(
        &engine,
        &at_step(PRAXIS_JAFFA, Some(DAKARA), 4906),
        DAKARA_NAME,
    );
    assert_eq!(
        labels_in(&praxis, TENT_CHAINS),
        vec!["8047:add_dialog_set(6890@441)"],
    );

    let tauri = fire_player_loaded(&engine, &at_step(TAURI, Some(DAKARA), 4906), DAKARA_NAME);
    assert_eq!(
        labels_in(&tauri, TENT_CHAINS),
        vec!["8048:add_dialog_set(6889@441)"],
    );
}

#[tokio::test]
async fn live_db_raknor_restore_stops_once_5936_is_done_or_off_step() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // The Free Jaffa's state after 8040: 5936 completed, so no Rak'nor bind.
    let done = with_objective(
        at_step(FREE_JAFFA, Some(DAKARA), 4906),
        1570,
        5936,
        "completed",
    );
    assert!(labels_in(
        &fire_player_loaded(&engine, &done, DAKARA_NAME),
        TENT_CHAINS
    )
    .is_empty());

    // Wrong world: the Rak'nor restore is world 61 only.
    let story_rm = at_step(FREE_JAFFA, Some(STORY_RM), 4906);
    assert!(labels_in(
        &fire_player_loaded(&engine, &story_rm, DAKARA_NAME),
        TENT_CHAINS
    )
    .is_empty());

    // Step 4904 binds Loth'ta, not Rak'nor.
    let at_4904 = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4904),
        DAKARA_NAME,
    );
    assert_eq!(
        labels_in(&at_4904, TENT_CHAINS),
        vec!["8053:add_dialog_set(6729@440)"],
    );
}

#[tokio::test]
async fn live_db_raknor_click_at_4906_completes_optional_5936() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let jaffa = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4906),
        "Dakara_E1_Raknor",
    );
    assert_eq!(
        labels_in(&jaffa, TENT_CHAINS),
        vec![
            "8045:display_dialog(6110)",
            "8045:complete_objective(1570/5936)",
            "8045:remove_dialog_set(6890@441)",
        ],
    );
    assert_eq!(display_dialog_count(&jaffa), 1);
    assert_no_deferred_actions(&jaffa, 8045);

    let tauri = fire_interact_tag(
        &engine,
        &at_step(TAURI, Some(DAKARA), 4906),
        "Dakara_E1_Raknor",
    );
    assert_eq!(
        labels_in(&tauri, TENT_CHAINS),
        vec![
            "8046:display_dialog(6111)",
            "8046:complete_objective(1570/5936)",
            "8046:remove_dialog_set(6889@441)",
        ],
    );
    assert_eq!(display_dialog_count(&tauri), 1);
    assert_no_deferred_actions(&tauri, 8046);
}

#[tokio::test]
async fn live_db_raknor_click_gives_nothing_once_5936_is_done_or_off_step() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let done = with_objective(
        at_step(FREE_JAFFA, Some(DAKARA), 4906),
        1570,
        5936,
        "completed",
    );
    assert!(labels_in(
        &fire_interact_tag(&engine, &done, "Dakara_E1_Raknor"),
        TENT_CHAINS
    )
    .is_empty());

    let at_4904 = at_step(FREE_JAFFA, Some(DAKARA), 4904);
    assert!(labels_in(
        &fire_interact_tag(&engine, &at_4904, "Dakara_E1_Raknor"),
        TENT_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_bratac_restore_binds_by_archetype_in_the_tent() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let jaffa = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4906),
        STORY_RM_NAME,
    );
    assert_eq!(
        labels_in(&jaffa, TENT_CHAINS),
        vec!["8049:add_dialog_set(6727@59)"],
    );
    let praxis = fire_player_loaded(
        &engine,
        &at_step(PRAXIS_JAFFA, Some(STORY_RM), 4906),
        STORY_RM_NAME,
    );
    assert_eq!(
        labels_in(&praxis, TENT_CHAINS),
        vec!["8049:add_dialog_set(6727@59)"],
    );
    let tauri = fire_player_loaded(
        &engine,
        &at_step(TAURI, Some(STORY_RM), 4906),
        STORY_RM_NAME,
    );
    assert_eq!(
        labels_in(&tauri, TENT_CHAINS),
        vec!["8050:add_dialog_set(6726@59)"],
    );

    // Bra'tac is in world 62 only, and not at 4904.
    let on_dakara = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4906),
        STORY_RM_NAME,
    );
    assert!(labels_in(&on_dakara, TENT_CHAINS).is_empty());
    let at_4904 = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4904),
        STORY_RM_NAME,
    );
    assert!(labels_in(&at_4904, TENT_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_bratac_click_at_4906_advances_to_4904() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let jaffa = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4906),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert_eq!(
        labels_in(&jaffa, TENT_CHAINS),
        vec![
            "8051:display_dialog(5811)",
            "8051:advance_step(1570->4904)",
            "8051:remove_dialog_set(6727@59)",
        ],
    );
    assert_eq!(display_dialog_count(&jaffa), 1);
    assert_no_deferred_actions(&jaffa, 8051);

    let tauri = fire_interact_tag(
        &engine,
        &at_step(TAURI, Some(STORY_RM), 4906),
        "Dakara_E1_StoryRm_Bratac",
    );
    assert_eq!(
        labels_in(&tauri, TENT_CHAINS),
        vec![
            "8052:display_dialog(5810)",
            "8052:advance_step(1570->4904)",
            "8052:remove_dialog_set(6726@59)",
        ],
    );
    assert_eq!(display_dialog_count(&tauri), 1);
    assert_no_deferred_actions(&tauri, 8052);
}

#[tokio::test]
async fn live_db_bratac_click_needs_4906_and_tent_world() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // Step 4904 (the next step) and the Dakara world (the wrong world).
    let at_4904 = at_step(FREE_JAFFA, Some(STORY_RM), 4904);
    let on_dakara = at_step(FREE_JAFFA, Some(DAKARA), 4906);
    assert!(labels_in(
        &fire_interact_tag(&engine, &at_4904, "Dakara_E1_StoryRm_Bratac"),
        TENT_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_interact_tag(&engine, &on_dakara, "Dakara_E1_StoryRm_Bratac"),
        TENT_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_lothta_restore_binds_by_archetype_at_4904() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let jaffa = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4904),
        DAKARA_NAME,
    );
    assert_eq!(
        labels_in(&jaffa, TENT_CHAINS),
        vec!["8053:add_dialog_set(6729@440)"],
    );
    let tauri = fire_player_loaded(&engine, &at_step(TAURI, Some(DAKARA), 4904), DAKARA_NAME);
    assert_eq!(
        labels_in(&tauri, TENT_CHAINS),
        vec!["8054:add_dialog_set(6728@440)"],
    );

    // Still at 4906 with 5936 done, Loth'ta has no bind yet.
    let at_4906 = with_objective(
        at_step(FREE_JAFFA, Some(DAKARA), 4906),
        1570,
        5936,
        "completed",
    );
    assert!(labels_in(
        &fire_player_loaded(&engine, &at_4906, DAKARA_NAME),
        TENT_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_lothta_click_at_4904_advances_to_4902() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let jaffa = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4904),
        "Dakara_E1_Lothta",
    );
    assert_eq!(
        labels_in(&jaffa, TENT_CHAINS),
        vec![
            "8055:display_dialog(5813)",
            "8055:advance_step(1570->4902)",
            "8055:remove_dialog_set(6729@440)",
        ],
    );
    assert_eq!(display_dialog_count(&jaffa), 1);
    assert_no_deferred_actions(&jaffa, 8055);

    let tauri = fire_interact_tag(
        &engine,
        &at_step(TAURI, Some(DAKARA), 4904),
        "Dakara_E1_Lothta",
    );
    assert_eq!(
        labels_in(&tauri, TENT_CHAINS),
        vec![
            "8056:display_dialog(5812)",
            "8056:advance_step(1570->4902)",
            "8056:remove_dialog_set(6728@440)",
        ],
    );
    assert_eq!(display_dialog_count(&tauri), 1);
    assert_no_deferred_actions(&tauri, 8056);

    // Loth'ta's click at 4906 (the previous step) gives nothing.
    let at_4906 = at_step(FREE_JAFFA, Some(DAKARA), 4906);
    assert!(labels_in(
        &fire_interact_tag(&engine, &at_4906, "Dakara_E1_Lothta"),
        TENT_CHAINS
    )
    .is_empty());
}
