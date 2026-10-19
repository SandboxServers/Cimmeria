//! DK-11: mission 1645 (dakara_e1_m1645_chains.sql, 8080-8119).
//!
//! Moh'katan (template 54) offers 1645 in the tent once 1570 is done
//! (8080 on the completion, 8081 on re-entry), and the offer has no
//! `interact_tag` chain: a click chain would skip `handle_interact`, the
//! only place that pins him as the speaker for the accept chain's
//! `display 5807` (`mission_1324.rs` lines 17-28). The accept (8082) moves
//! to 4907, Loth'ta binds his turn-in on entering Dakara_E1 (8083) and the
//! click (8084) takes the orders back and completes the mission.

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_mission_completed,
    fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The orders are item 6779 (work-packets.md, C6).
const ORDERS: &str = "6779";

/// A Free Jaffa in world `world` with 1570 completed and 1645 not started:
/// the state in which the offer is made.
fn offer_state(world: Option<i32>) -> ExecutionContext {
    with_mission(
        with_mission(ctx_for(FREE_JAFFA, world), 1570, "completed"),
        1645,
        "not_active",
    )
}

/// The offer state with 1645 accepted and at step 4907 (deliver to Loth'ta).
fn at_4907(world: Option<i32>) -> ExecutionContext {
    with_step(
        with_mission(offer_state(world), 1645, "active"),
        1645,
        4907,
        "active",
    )
}

#[tokio::test]
async fn live_db_1570_done_binds_the_1645_offer_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_mission_completed(&engine, &offer_state(Some(STORY_RM)), 1570);
    assert_eq!(
        labels_in(&r, M1645_CHAINS),
        vec!["8080:add_dialog_set(6736@54)"],
    );

    // Wrong world (1570 finished out on Dakara) or 1645 already under way.
    let outside = fire_mission_completed(&engine, &offer_state(Some(DAKARA)), 1570);
    assert!(labels_in(&outside, M1645_CHAINS).is_empty());
    let started = with_mission(offer_state(Some(STORY_RM)), 1645, "active");
    assert!(labels_in(
        &fire_mission_completed(&engine, &started, 1570),
        M1645_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_entering_the_tent_restores_the_1645_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &offer_state(Some(STORY_RM)), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1645_CHAINS),
        vec!["8081:add_dialog_set(6736@54)"],
    );

    // 1570 not yet done, 1645 already accepted, or the world is not 62.
    let not_done = with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1570, "active");
    let accepted = with_mission(offer_state(Some(STORY_RM)), 1645, "active");
    let wrong_world = offer_state(Some(DAKARA));
    assert!(labels_in(
        &fire_player_loaded(&engine, &not_done, STORY_RM_NAME),
        M1645_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_player_loaded(&engine, &accepted, STORY_RM_NAME),
        M1645_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_player_loaded(&engine, &wrong_world, STORY_RM_NAME),
        M1645_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_clicking_moh_katan_before_accepting_resolves_no_chain() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(
        &engine,
        &offer_state(Some(STORY_RM)),
        "Dakara_E1_StoryRm_Mohkatan",
    );
    assert!(r.actions.is_empty());
}

#[tokio::test]
async fn live_db_accepting_the_offer_takes_the_orders_scene_and_moves_to_4907() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = offer_state(Some(STORY_RM));
    let r = fire_dialog_choice(&engine, &ctx, 5980);
    assert_eq!(
        labels_in(&r, M1645_CHAINS),
        vec![
            "8082:accept_mission(1645)",
            "8082:display_dialog(5807)",
            "8082:add_item(6779x1)",
            "8082:advance_step(1645->4907)",
            "8082:remove_dialog_set(6736@54)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8082);

    // Wrong world, 1645 already active, or 1570 not yet completed.
    let wrong_world = fire_dialog_choice(&engine, &offer_state(Some(DAKARA)), 5980);
    let accepted = with_mission(offer_state(Some(STORY_RM)), 1645, "active");
    let not_done = with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1570, "active");
    assert!(labels_in(&wrong_world, M1645_CHAINS).is_empty());
    assert!(labels_in(&fire_dialog_choice(&engine, &accepted, 5980), M1645_CHAINS).is_empty());
    assert!(labels_in(&fire_dialog_choice(&engine, &not_done, 5980), M1645_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_entering_dakara_at_4907_binds_loths_turn_in() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_4907(Some(DAKARA)), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1645_CHAINS),
        vec!["8083:add_dialog_set(6735@440)"],
    );

    // Still at 5168 (the orders not yet taken), or the world is not 61.
    let at_5168 = with_step(
        with_mission(offer_state(Some(DAKARA)), 1645, "active"),
        1645,
        5168,
        "active",
    );
    assert!(labels_in(
        &fire_player_loaded(&engine, &at_5168, DAKARA_NAME),
        M1645_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_player_loaded(&engine, &at_4907(Some(STORY_RM)), DAKARA_NAME),
        M1645_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_loths_turn_in_takes_the_orders_and_completes_1645() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_4907(Some(DAKARA)), "Dakara_E1_Lothta");
    assert_eq!(
        labels_in(&r, M1645_CHAINS),
        vec![
            "8084:display_dialog(5808)".to_string(),
            format!("8084:remove_item({ORDERS}x1)"),
            "8084:remove_dialog_set(6735@440)".to_string(),
            "8084:complete_mission(1645)".to_string(),
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8084);

    // Before the orders are delivered (5168), or in the tent rather than Dakara.
    let at_5168 = with_step(
        with_mission(offer_state(Some(DAKARA)), 1645, "active"),
        1645,
        5168,
        "active",
    );
    assert!(labels_in(
        &fire_interact_tag(&engine, &at_5168, "Dakara_E1_Lothta"),
        M1645_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_interact_tag(&engine, &at_4907(Some(STORY_RM)), "Dakara_E1_Lothta"),
        M1645_CHAINS
    )
    .is_empty());
}
