//! DK-32a: mission 1653 "Superweapon" (dakara_e1_m1653_chains.sql, 8350-8389).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_mission_completed,
    fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

const MISSION: i32 = 1653;
const STEP_COURTYARD: i32 = 4936;
const STEP_SUPPLIES: i32 = 4938;
const STEP_RADIO: i32 = 4937;
const STEP_REPORT: i32 = 4935;
/// Objective of the western power supply (`Dakara_E1_PowerSupply1`).
const WEST: i32 = 5642;
/// Objective of the eastern power supply (`Dakara_E1_PowerSupply2`).
const EAST: i32 = 5641;
const SUPPLY_WEST: &str = "Dakara_E1_PowerSupply1";
const SUPPLY_EAST: &str = "Dakara_E1_PowerSupply2";
const VOCUUM: &str = "Dakara_E1_Vocuum";
/// Optional objective of the Vocuum (DK-32c).
const VOCUUM_OBJ: i32 = 5643;
const BRATAC: &str = "Dakara_E1_StoryRm_Bratac";
const COURTYARD: &str = "Dakara_E1.SuperweaponCourtyard";
const MOHKATAN_TENT: &str = "Dakara_E1.MohkatanTent";
const RADIO: i32 = 5849;

fn want(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Mission 1652 completed and 1653 at `world`, with 1653 in `status`.
fn base(world: Option<i32>, status: &str) -> ExecutionContext {
    let ctx = with_mission(ctx_for(FREE_JAFFA, world), 1652, "completed");
    with_mission(ctx, MISSION, status)
}

/// Mission 1653 active at `step` in `world`.
fn at(world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(base(world, "active"), MISSION, step, "active")
}

/// Step 4938 in Dakara_E1 with the given supply objectives completed.
fn supplies(done: &[i32]) -> ExecutionContext {
    let mut ctx = at(Some(DAKARA), STEP_SUPPLIES);
    for objective in done {
        ctx = with_objective(ctx, MISSION, *objective, "completed");
    }
    ctx
}

#[tokio::test]
async fn live_db_accept_on_1652_done_only_while_not_active() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_mission_completed(&engine, &base(Some(DAKARA), "not_active"), 1652);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&["8350:accept_mission(1653)"])
    );
    assert_no_deferred_actions(&r, 8350);

    let r = fire_mission_completed(&engine, &base(Some(DAKARA), "active"), 1652);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
    let r = fire_mission_completed(&engine, &base(Some(DAKARA), "not_active"), 1651);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_courtyard_enter_advances_and_binds_supplies_and_vocuum() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_enter_region(&engine, &at(Some(DAKARA), STEP_COURTYARD), COURTYARD);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8351:advance_step(1653->4938)",
            "8351:add_dialog_set(6892@456)",
            "8351:add_dialog_set(6892@463)",
            "8351:add_dialog_set(6892@457)",
        ])
    );
    assert_no_deferred_actions(&r, 8351);

    // Nothing at 4938, nor in the other region.
    let r = fire_enter_region(&engine, &at(Some(DAKARA), STEP_SUPPLIES), COURTYARD);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
    let r = fire_enter_region(&engine, &at(Some(DAKARA), STEP_COURTYARD), MOHKATAN_TENT);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_restore_binds_only_the_open_props() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &supplies(&[]), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8352:add_dialog_set(6892@456)",
            "8353:add_dialog_set(6892@463)",
            "8364:add_dialog_set(6892@457)",
        ])
    );
    assert_no_deferred_actions(&r, 8352);
    assert_no_deferred_actions(&r, 8353);
    assert_no_deferred_actions(&r, 8364);

    let r = fire_player_loaded(&engine, &supplies(&[EAST]), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8352:add_dialog_set(6892@456)",
            "8364:add_dialog_set(6892@457)",
        ])
    );
    let r = fire_player_loaded(&engine, &supplies(&[WEST]), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8353:add_dialog_set(6892@463)",
            "8364:add_dialog_set(6892@457)",
        ])
    );
    let r = fire_player_loaded(&engine, &supplies(&[WEST, EAST]), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&["8364:add_dialog_set(6892@457)"])
    );
    // The Vocuum done: only the supplies restore.
    let r = fire_player_loaded(&engine, &supplies(&[VOCUUM_OBJ]), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8352:add_dialog_set(6892@456)",
            "8353:add_dialog_set(6892@463)",
        ])
    );
    // Not on Dakara, or not at 4938: no restore.
    let r = fire_player_loaded(&engine, &at(Some(STORY_RM), STEP_SUPPLIES), STORY_RM_NAME);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
    let r = fire_player_loaded(&engine, &at(Some(DAKARA), STEP_COURTYARD), DAKARA_NAME);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_supply_clicks_in_both_orders_resolve_one_chain_each() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // West first, then east.
    let r = fire_interact_tag(&engine, &supplies(&[]), SUPPLY_WEST);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8354:complete_objective(1653/5642)",
            "8354:remove_dialog_set(6892@456)",
        ])
    );
    let west_done = supplies(&[WEST]);
    let r = fire_interact_tag(&engine, &west_done, SUPPLY_WEST);
    assert!(labels_in(&r, M1653_CHAINS).is_empty(), "west done");
    let r = fire_interact_tag(&engine, &west_done, SUPPLY_EAST);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8357:remove_dialog_set(6892@463)",
            "8357:remove_dialog_set(6892@457)",
            "8357:advance_step(1653->4937)",
            "8357:display_radio_dialog(5849)",
        ])
    );
    assert_eq!(radio_count(&r), 1);
    assert_no_deferred_actions(&r, 8357);

    // East first, then west.
    let r = fire_interact_tag(&engine, &supplies(&[]), SUPPLY_EAST);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8356:complete_objective(1653/5641)",
            "8356:remove_dialog_set(6892@463)",
        ])
    );
    let east_done = supplies(&[EAST]);
    let r = fire_interact_tag(&engine, &east_done, SUPPLY_EAST);
    assert!(labels_in(&r, M1653_CHAINS).is_empty(), "east done");
    let r = fire_interact_tag(&engine, &east_done, SUPPLY_WEST);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8355:remove_dialog_set(6892@456)",
            "8355:remove_dialog_set(6892@457)",
            "8355:advance_step(1653->4937)",
            "8355:display_radio_dialog(5849)",
        ])
    );
    assert_eq!(radio_count(&r), 1);
    assert_no_deferred_actions(&r, 8355);
}

#[tokio::test]
async fn live_db_vocuum_click_completes_5643_once_and_never_moves_the_step() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let vocuum_labels = want(&[
        "8365:complete_objective(1653/5643)",
        "8365:remove_dialog_set(6892@457)",
    ]);
    let dones: [&[i32]; 3] = [&[], &[WEST], &[EAST]];
    for done in dones {
        let r = fire_interact_tag(&engine, &supplies(done), VOCUUM);
        assert_eq!(labels_in(&r, M1653_CHAINS), vocuum_labels);
        assert_eq!(display_dialog_count(&r), 0);
        assert_eq!(radio_count(&r), 0);
        assert_no_deferred_actions(&r, 8365);
    }

    // Optional objective already done, or not at 4938 in Dakara_E1: no Vocuum chain.
    let no_vocuum = [
        supplies(&[VOCUUM_OBJ]),
        at(Some(DAKARA), STEP_COURTYARD),
        at(Some(DAKARA), STEP_RADIO),
        at(Some(STORY_RM), STEP_SUPPLIES),
    ];
    for ctx in no_vocuum {
        let r = fire_interact_tag(&engine, &ctx, VOCUUM);
        assert!(labels_in(&r, M1653_CHAINS).is_empty());
    }

    // The supplies ignore the Vocuum; the step still advances with it done.
    let r = fire_interact_tag(&engine, &supplies(&[VOCUUM_OBJ]), SUPPLY_WEST);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8354:complete_objective(1653/5642)",
            "8354:remove_dialog_set(6892@456)",
        ])
    );
    let r = fire_interact_tag(&engine, &supplies(&[VOCUUM_OBJ, EAST]), SUPPLY_WEST);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8355:remove_dialog_set(6892@456)",
            "8355:remove_dialog_set(6892@457)",
            "8355:advance_step(1653->4937)",
            "8355:display_radio_dialog(5849)",
        ])
    );
}

#[tokio::test]
async fn live_db_radio_close_advances_anywhere_and_binds_bratac_in_tent() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let advance = "8358:advance_step(1653->4935)";
    let r = fire_dialog_choice(&engine, &at(None, STEP_RADIO), RADIO);
    assert_eq!(labels_in(&r, M1653_CHAINS), want(&[advance]));
    for world in [Some(DAKARA), Some(STORY_RM)] {
        let r = fire_dialog_choice(&engine, &at(world, STEP_RADIO), RADIO);
        assert!(labels_in(&r, M1653_CHAINS).contains(&advance.to_string()));
    }
    let r = fire_dialog_choice(&engine, &at(Some(STORY_RM), STEP_RADIO), RADIO);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[advance, "8359:add_dialog_set(6775@59)"])
    );
    let r = fire_dialog_choice(&engine, &at(Some(DAKARA), STEP_RADIO), RADIO);
    assert_eq!(labels_in(&r, M1653_CHAINS), want(&[advance]));
    assert_no_deferred_actions(&r, 8358);

    // Not at 4937: nothing.
    let r = fire_dialog_choice(&engine, &at(Some(STORY_RM), STEP_REPORT), RADIO);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_radio_recovery_replays_after_load_with_two_second_delay() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at(Some(DAKARA), STEP_RADIO), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&["8360:display_radio_dialog(5849)"])
    );
    assert_eq!(delays_of(&r, 8360), vec![2000]);
    assert_eq!(radio_count(&r), 1);

    let r = fire_player_loaded(&engine, &at(Some(STORY_RM), STEP_RADIO), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&["8361:display_radio_dialog(5849)"])
    );
    assert_eq!(delays_of(&r, 8361), vec![2000]);

    // Past the call: no replay.
    let r = fire_player_loaded(&engine, &at(Some(DAKARA), STEP_REPORT), DAKARA_NAME);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_bratac_report_unbinds_before_complete() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at(Some(STORY_RM), STEP_REPORT), BRATAC);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&[
            "8363:display_dialog(5848)",
            "8363:remove_dialog_set(6775@59)",
            "8363:complete_mission(1653)",
        ])
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8363);

    // Bra'tac is only clickable for the report in the tent.
    let r = fire_interact_tag(&engine, &at(Some(DAKARA), STEP_REPORT), BRATAC);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
    let r = fire_interact_tag(&engine, &at(Some(STORY_RM), STEP_RADIO), BRATAC);
    assert!(labels_in(&r, M1653_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_bratac_bind_restores_on_storyrm_load_at_report() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at(Some(STORY_RM), STEP_REPORT), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&["8362:add_dialog_set(6775@59)"])
    );
    assert_no_deferred_actions(&r, 8362);
    // At 4937 in the tent the radio recovery (8361) fires, not the report bind.
    let r = fire_player_loaded(&engine, &at(Some(STORY_RM), STEP_RADIO), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1653_CHAINS),
        want(&["8361:display_radio_dialog(5849)"])
    );
}
