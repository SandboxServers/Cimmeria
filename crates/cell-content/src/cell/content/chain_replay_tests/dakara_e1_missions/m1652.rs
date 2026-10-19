//! DK-31b: mission 1652 "Fireball" (dakara_e1_m1652_chains.sql, 8320-8349).
//!
//! Bra'tac (template 59) offers 1652 in the story room once 1651 is done
//! (8320 on the completion, 8321 on re-entry); the accept (8322) shows the
//! briefing 5846. On Dakara_E1 the western ring control (455) and then the
//! eastern one (462) are bound by step (8323, 8324). Each click (8325, 8326)
//! shows its bomb dialog; the eastern click takes the generator (6784) and
//! completes the mission.

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_mission_completed,
    fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The offer is made in the story room once 1651 is completed and 1652 not begun.
fn offer_state(world: Option<i32>) -> ExecutionContext {
    with_mission(
        with_mission(ctx_for(FREE_JAFFA, world), 1651, "completed"),
        1652,
        "not_active",
    )
}

/// 1652 accepted and at `step` (4933 western, 4934 eastern) on `world`.
fn at_step(world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(FREE_JAFFA, world), 1652, "active"),
        1652,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_1651_done_offers_1652_in_the_story_room() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_mission_completed(&engine, &offer_state(Some(STORY_RM)), 1651);
    assert_eq!(
        labels_in(&r, M1652_CHAINS),
        vec!["8320:add_dialog_set(6776@59)"],
    );

    // Wrong world (1651 finished out on Dakara) or 1652 already under way.
    let outside = fire_mission_completed(&engine, &offer_state(Some(DAKARA)), 1651);
    assert!(labels_in(&outside, M1652_CHAINS).is_empty());
    let started = with_mission(offer_state(Some(STORY_RM)), 1652, "active");
    assert!(labels_in(
        &fire_mission_completed(&engine, &started, 1651),
        M1652_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_entering_the_story_room_restores_the_1652_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &offer_state(Some(STORY_RM)), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1652_CHAINS),
        vec!["8321:add_dialog_set(6776@59)"],
    );

    // 1651 not yet done, 1652 already accepted, or the world is not 62.
    let not_done = with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1651, "active");
    let accepted = with_mission(offer_state(Some(STORY_RM)), 1652, "active");
    let wrong_world = offer_state(Some(DAKARA));
    assert!(labels_in(
        &fire_player_loaded(&engine, &not_done, STORY_RM_NAME),
        M1652_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_player_loaded(&engine, &accepted, STORY_RM_NAME),
        M1652_CHAINS
    )
    .is_empty());
    assert!(labels_in(
        &fire_player_loaded(&engine, &wrong_world, STORY_RM_NAME),
        M1652_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_accepting_the_offer_takes_1652_and_shows_the_briefing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = offer_state(Some(STORY_RM));
    let r = fire_dialog_choice(&engine, &ctx, 5986);
    assert_eq!(
        labels_in(&r, M1652_CHAINS),
        vec![
            "8322:accept_mission(1652)",
            "8322:display_dialog(5846)",
            "8322:remove_dialog_set(6776@59)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8322);

    // Wrong world, 1652 already active, or 1651 not yet completed.
    let wrong_world = fire_dialog_choice(&engine, &offer_state(Some(DAKARA)), 5986);
    let accepted = with_mission(offer_state(Some(STORY_RM)), 1652, "active");
    let not_done = with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1651, "active");
    assert!(labels_in(&wrong_world, M1652_CHAINS).is_empty());
    assert!(labels_in(&fire_dialog_choice(&engine, &accepted, 5986), M1652_CHAINS).is_empty());
    assert!(labels_in(&fire_dialog_choice(&engine, &not_done, 5986), M1652_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_entering_dakara_binds_the_control_of_the_current_step() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let western = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4933), DAKARA_NAME);
    assert_eq!(
        labels_in(&western, M1652_CHAINS),
        vec!["8323:add_dialog_set(6887@455)"],
    );
    let eastern = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4934), DAKARA_NAME);
    assert_eq!(
        labels_in(&eastern, M1652_CHAINS),
        vec!["8324:add_dialog_set(6888@462)"],
    );

    // The story room is world 62: the accept is there, the bind waits for Dakara.
    let story_room = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4933), STORY_RM_NAME);
    assert!(labels_in(&story_room, M1652_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_clicking_the_western_control_bombs_it_and_lights_the_east() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = at_step(Some(DAKARA), 4933);
    let r = fire_interact_tag(&engine, &ctx, "Dakara_E1_RingControl_XMinus");
    assert_eq!(
        labels_in(&r, M1652_CHAINS),
        vec![
            "8325:display_dialog(6108)",
            "8325:advance_step(1652->4934)",
            "8325:remove_dialog_set(6887@455)",
            "8325:add_dialog_set(6888@462)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8325);

    // The eastern control at 4933 resolves nothing, and neither does the
    // western one once the step is 4934 or 1652 is completed.
    let eastern_first = fire_interact_tag(&engine, &ctx, "Dakara_E1_RingControl_XPlus");
    assert!(labels_in(&eastern_first, M1652_CHAINS).is_empty());
    let at_4934 = at_step(Some(DAKARA), 4934);
    assert!(labels_in(
        &fire_interact_tag(&engine, &at_4934, "Dakara_E1_RingControl_XMinus"),
        M1652_CHAINS
    )
    .is_empty());
    let completed = with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1652, "completed");
    assert!(labels_in(
        &fire_interact_tag(&engine, &completed, "Dakara_E1_RingControl_XMinus"),
        M1652_CHAINS
    )
    .is_empty());
}

#[tokio::test]
async fn live_db_clicking_the_eastern_control_spends_the_generator_and_completes() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = at_step(Some(DAKARA), 4934);
    let r = fire_interact_tag(&engine, &ctx, "Dakara_E1_RingControl_XPlus");
    assert_eq!(
        labels_in(&r, M1652_CHAINS),
        vec![
            "8326:display_dialog(6109)",
            "8326:remove_item(6784x1)",
            "8326:remove_dialog_set(6888@462)",
            "8326:complete_mission(1652)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8326);

    // The western control at 4934 resolves nothing, and neither does the
    // eastern one at 4933 or once 1652 is completed.
    assert!(labels_in(
        &fire_interact_tag(&engine, &ctx, "Dakara_E1_RingControl_XMinus"),
        M1652_CHAINS
    )
    .is_empty());
    let at_4933 = at_step(Some(DAKARA), 4933);
    assert!(labels_in(
        &fire_interact_tag(&engine, &at_4933, "Dakara_E1_RingControl_XPlus"),
        M1652_CHAINS
    )
    .is_empty());
    let completed = with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1652, "completed");
    assert!(labels_in(
        &fire_interact_tag(&engine, &completed, "Dakara_E1_RingControl_XPlus"),
        M1652_CHAINS
    )
    .is_empty());
}
