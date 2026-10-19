//! DK-12b: mission 1646, offer, accept and turn-in (dakara_e1_m1646_offer_chains.sql, 8120-8139).

use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_player_loaded,
    with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The Moh'katan tag (template 54, world 62).
const MOHKATAN_TAG: &str = "Dakara_E1_StoryRm_Mohkatan";
/// The offer blurb (dsm 6737) and its choice.
const OFFER_CHOICE: i32 = 5981;

/// A context in which 1645 is completed and 1646 is not yet active: the
/// state in which Moh'katan offers 1646 on entering the tent.
fn offered(world: Option<i32>) -> ExecutionContext {
    with_mission(
        with_mission(ctx_for(TAURI, world), 1645, "completed"),
        1646,
        "not_active",
    )
}

/// A context with 1646 active at step 4909, the turn-in step.
fn turn_in(world: Option<i32>) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(TAURI, world), 1646, "active"),
        1646,
        4909,
        "active",
    )
}

#[tokio::test]
async fn live_db_moh_katan_click_before_the_offer_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &offered(Some(STORY_RM)), MOHKATAN_TAG);
    assert!(r.actions.is_empty());
}

#[tokio::test]
async fn live_db_offer_restore_binds_moh_katan_for_the_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &offered(Some(STORY_RM)), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1646_OFFER_CHAINS),
        vec!["8120:add_dialog_set(6737@54)"],
    );
}

#[tokio::test]
async fn live_db_offer_choice_accepts_1646_and_unbinds() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_dialog_choice(&engine, &offered(Some(STORY_RM)), OFFER_CHOICE);
    assert_eq!(
        labels_in(&r, M1646_OFFER_CHAINS),
        vec![
            "8121:accept_mission(1646)",
            "8121:display_dialog(5819)",
            "8121:add_item(6780x1)",
            "8121:remove_dialog_set(6737@54)",
        ],
    );
}

#[tokio::test]
async fn live_db_offer_gives_nothing_without_1645_or_once_1646_is_active() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let active_1645 = with_mission(offered(Some(STORY_RM)), 1645, "active");
    let not_active_1645 = with_mission(offered(Some(STORY_RM)), 1645, "not_active");
    let active_1646 = with_mission(offered(Some(STORY_RM)), 1646, "active");
    for ctx in [&active_1645, &not_active_1645] {
        assert!(labels_in(
            &fire_player_loaded(&engine, ctx, STORY_RM_NAME),
            M1646_OFFER_CHAINS
        )
        .is_empty());
        assert!(labels_in(
            &fire_dialog_choice(&engine, ctx, OFFER_CHOICE),
            M1646_OFFER_CHAINS
        )
        .is_empty());
    }
    assert!(labels_in(
        &fire_player_loaded(&engine, &active_1646, STORY_RM_NAME),
        M1646_OFFER_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_dialog_choice(&engine, &active_1646, OFFER_CHOICE),
        M1646_OFFER_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_offer_restore_needs_the_tent_world() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let on_dakara = fire_player_loaded(&engine, &offered(Some(DAKARA)), DAKARA_NAME);
    assert!(labels_in(&on_dakara, M1646_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_offer_choice_needs_the_tent_world() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // A stale dialog choice from Dakara_E1 must not accept 1646 outside the tent.
    let on_dakara = fire_dialog_choice(&engine, &offered(Some(DAKARA)), OFFER_CHOICE);
    assert!(labels_in(&on_dakara, M1646_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_turn_in_click_completes_1646_after_unbinding() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &turn_in(Some(STORY_RM)), MOHKATAN_TAG);
    assert_eq!(
        labels_in(&r, M1646_OFFER_CHAINS),
        vec![
            "8123:display_dialog(5820)",
            "8123:remove_dialog_set(6744@54)",
            "8123:complete_mission(1646)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
}

#[tokio::test]
async fn live_db_turn_in_restore_binds_moh_katan_at_4909_in_the_tent() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &turn_in(Some(STORY_RM)), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1646_OFFER_CHAINS),
        vec!["8122:add_dialog_set(6744@54)"],
    );
    // Step 4908 (the drops) binds nothing here, and world 61 is not the tent.
    let at_4908 = with_step(
        with_mission(ctx_for(TAURI, Some(STORY_RM)), 1646, "active"),
        1646,
        4908,
        "active",
    );
    let r = fire_player_loaded(&engine, &at_4908, STORY_RM_NAME);
    assert!(labels_in(&r, M1646_OFFER_CHAINS).is_empty());
    let r = fire_player_loaded(&engine, &turn_in(Some(DAKARA)), DAKARA_NAME);
    assert!(labels_in(&r, M1646_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_turn_in_click_needs_4909_and_the_tent() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let at_4908 = with_step(
        with_mission(ctx_for(TAURI, Some(STORY_RM)), 1646, "active"),
        1646,
        4908,
        "active",
    );
    let r = fire_interact_tag(&engine, &at_4908, MOHKATAN_TAG);
    assert!(labels_in(&r, M1646_OFFER_CHAINS).is_empty());
    let r = fire_interact_tag(&engine, &turn_in(Some(DAKARA)), MOHKATAN_TAG);
    assert!(labels_in(&r, M1646_OFFER_CHAINS).is_empty());
    // Once 1646 is completed its step is no longer active, so the turn-in does not run again.
    let done = with_mission(ctx_for(TAURI, Some(STORY_RM)), 1646, "completed");
    let r = fire_interact_tag(&engine, &done, MOHKATAN_TAG);
    assert!(labels_in(&r, M1646_OFFER_CHAINS).is_empty());
}
