//! DK-14a: mission 1649 "Moh'katan" offer, Bra'tac, the radio call and the turn-in (dakara_e1_m1649_offer_chains.sql, 8200-8219).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_player_loaded,
    with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The Bra'tac tag (template 59, world 62).
const BRATAC_TAG: &str = "Dakara_E1_StoryRm_Bratac";
/// The radio offer (narrator text 5831) and its choice.
const OFFER_CHOICE: i32 = 5831;
/// Bra'tac's talk on his daughter (dsm 6758) and its choice.
const DAUGHTER_CHOICE: i32 = 5832;

/// A context in which 1648 is completed and 1649 is not yet active: the
/// state in which the radio offer is due on entering Dakara_E1.
fn offered(world: Option<i32>) -> ExecutionContext {
    with_mission(
        with_mission(ctx_for(TAURI, world), 1648, "completed"),
        1649,
        "not_active",
    )
}

/// A context with 1649 active at `step`, with 1648 completed.
fn at_step(world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(
            with_mission(ctx_for(TAURI, world), 1648, "completed"),
            1649,
            "active",
        ),
        1649,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_radio_offer_on_entry_waits_two_seconds() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &offered(Some(DAKARA)), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec!["8200:display_radio_dialog(5831)"],
    );
    assert_eq!(delays_of(&r, 8200), [2000]);
}

#[tokio::test]
async fn live_db_radio_offer_needs_dakara_and_1648_done_and_1649_unaccepted() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // The tent is not the radio's world: Bra'tac's restore owns that place.
    let on_story_rm = fire_player_loaded(&engine, &offered(Some(STORY_RM)), STORY_RM_NAME);
    assert!(labels_in(&on_story_rm, M1649_OFFER_CHAINS).is_empty());
    let active_1648 = with_mission(offered(Some(DAKARA)), 1648, "active");
    let r = fire_player_loaded(&engine, &active_1648, DAKARA_NAME);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
    let active_1649 = with_mission(offered(Some(DAKARA)), 1649, "active");
    let r = fire_player_loaded(&engine, &active_1649, DAKARA_NAME);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_closing_the_radio_offer_accepts_1649_without_a_world() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // A window closed after a flap trip carries no world; the accept still runs.
    let r = fire_dialog_choice(&engine, &offered(None), OFFER_CHOICE);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec!["8201:accept_mission(1649)"],
    );
    let active_1649 = with_mission(offered(None), 1649, "active");
    let r = fire_dialog_choice(&engine, &active_1649, OFFER_CHOICE);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_bratac_restore_binds_the_daughter_talk_at_4923() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4923), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec!["8202:add_dialog_set(6758@59)"],
    );
    let r = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4923), DAKARA_NAME);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_bratac_daughter_click_advances_and_unbinds() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4923), BRATAC_TAG);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec![
            "8203:display_dialog(5832)",
            "8203:advance_step(1649->4921)",
            "8203:remove_dialog_set(6758@59)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8203);
    // At 4921 the daughter talk is already done, so a click does nothing here.
    let r = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4921), BRATAC_TAG);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_closing_the_daughter_talk_makes_the_radio_call() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_dialog_choice(&engine, &at_step(Some(STORY_RM), 4921), DAUGHTER_CHOICE);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec![
            "8204:display_radio_dialog(5833)",
            "8204:advance_step(1649->4922)",
        ],
    );
    assert_eq!(radio_count(&r), 1);
    assert_no_deferred_actions(&r, 8204);
    // Closing the talk at 4923 (before the click's advance) does not call Teal'c.
    let r = fire_dialog_choice(&engine, &at_step(Some(STORY_RM), 4923), DAUGHTER_CHOICE);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_radio_recovery_on_entry_at_4921_waits_then_advances() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4921), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec![
            "8205:display_radio_dialog(5833)",
            "8205:advance_step(1649->4922)",
        ],
    );
    assert_eq!(delays_of(&r, 8205), [2000, 0]);
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4921), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec![
            "8206:display_radio_dialog(5833)",
            "8206:advance_step(1649->4922)",
        ],
    );
    assert_eq!(delays_of(&r, 8206), [2000, 0]);
    // Past 4921 the recovery is quiet.
    let r = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4922), DAKARA_NAME);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_turn_in_restore_binds_the_evidence_talk_at_4920() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4920), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec!["8207:add_dialog_set(6765@59)"],
    );
    let r = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4920), DAKARA_NAME);
    assert!(labels_in(&r, M1649_OFFER_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_turn_in_click_hands_over_the_evidence_and_completes() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4920), BRATAC_TAG);
    assert_eq!(
        labels_in(&r, M1649_OFFER_CHAINS),
        vec![
            "8208:display_dialog(5838)",
            "8208:remove_item(6782x1)",
            "8208:remove_item(6783x1)",
            "8208:remove_dialog_set(6765@59)",
            "8208:complete_mission(1649)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8208);
    // Not yet at 4920: at 4923 only the daughter click (8203) answers.
    let r = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4923), BRATAC_TAG);
    let labels = labels_in(&r, M1649_OFFER_CHAINS);
    assert!(!labels.is_empty());
    assert!(labels.iter().all(|l| l.starts_with("8203:")));
}
