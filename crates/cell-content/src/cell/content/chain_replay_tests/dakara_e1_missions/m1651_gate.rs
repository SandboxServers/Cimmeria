//! DK-30b: mission 1651 "Renewed Attack", step 4930 (dakara_e1_m1651_gate_chains.sql, 8310-8319).
//! DK-30b-r: the crossing is real gate travel (D-DK24); Bra'tac's 4932 report grants gate 27.

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The DHD (template 1, world 61).
const DHD_TAG: &str = "Dakara_E1_DHD";
/// Bra'tac in the Story Room (world 62), the 4932 status report (chain 8313).
const BRATAC_TAG: &str = "Dakara_E1_StoryRm_Bratac";

/// A player on 1651 at `step`, mission and step both `active`.
fn at_step(world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(TAURI, world), 1651, "active"),
        1651,
        step,
        "active",
    )
}

/// A player on 1651 at `step` with `archetype`, mission and step both `active`.
fn report_ctx(archetype: i64, world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(archetype, world), 1651, "active"),
        1651,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_crossing_to_the_sgc_at_4930_advances_to_4928() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // 18 is Omega Site: dialling from its DHD counts too (F41).
    for world in [Some(DAKARA), Some(18)] {
        let r = fire_stargate_crossed(&engine, &at_step(world, 4930), "SGC_W1");
        assert_eq!(
            labels_in(&r, M1651_GATE_CHAINS),
            ["8310:advance_step(1651->4928)".to_string()],
            "world {world:?}",
        );
        assert_eq!(display_dialog_count(&r), 0, "world {world:?}");
        assert_no_deferred_actions(&r, 8310);
    }
}

#[tokio::test]
async fn live_db_crossing_stays_quiet_elsewhere() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for destination in [DAKARA_NAME, "Omega_Site"] {
        let r = fire_stargate_crossed(&engine, &at_step(Some(DAKARA), 4930), destination);
        assert!(
            labels_in(&r, M1651_GATE_CHAINS).is_empty(),
            "to {destination}"
        );
    }
    for step in [4932, 4928, 4931, 4929, 5171] {
        let r = fire_stargate_crossed(&engine, &at_step(Some(DAKARA), step), "SGC_W1");
        assert!(labels_in(&r, M1651_GATE_CHAINS).is_empty(), "step {step}");
    }
    // No 1651 state at all.
    let ctx = ctx_for(TAURI, Some(DAKARA));
    let r = fire_stargate_crossed(&engine, &ctx, "SGC_W1");
    assert!(labels_in(&r, M1651_GATE_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_the_dhd_click_resolves_no_chain_so_the_dial_window_opens() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // F25: no chain may match the DHD, or the dial window stays shut. A failure here
    // names the chain that matched; do not weaken the assertion.
    for step in [4932, 4930, 4928, 4931, 4929, 5171] {
        let r = fire_interact_tag(&engine, &at_step(Some(DAKARA), step), DHD_TAG);
        assert!(r.actions.is_empty(), "step {step}: {:?}", r.actions);
    }
    let ctx = ctx_for(TAURI, Some(DAKARA));
    let r = fire_interact_tag(&engine, &ctx, DHD_TAG);
    assert!(r.actions.is_empty(), "no 1651 state: {:?}", r.actions);
}

#[tokio::test]
async fn live_db_bratac_report_grants_the_sgc_address() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [TAURI, FREE_JAFFA] {
        let ctx = report_ctx(archetype, Some(STORY_RM), 4932);
        let r = fire_interact_tag(&engine, &ctx, BRATAC_TAG);
        assert_eq!(
            labels_in(&r, M1651_GATE_CHAINS),
            ["8313:grant_stargate_address(27)".to_string()],
            "archetype {archetype}",
        );
        assert!(
            labels_in(&r, M1651_CHAINS).contains(&"8292:advance_step(1651->4930)".to_string()),
            "archetype {archetype}",
        );
        assert_eq!(display_dialog_count(&r), 1, "archetype {archetype}");
        assert_no_deferred_actions(&r, 8313);
    }
}

#[tokio::test]
async fn live_db_no_grant_outside_the_report() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (step, world) in [
        (4930, Some(STORY_RM)),
        (5171, Some(STORY_RM)),
        (4932, Some(DAKARA)),
    ] {
        let r = fire_interact_tag(&engine, &report_ctx(TAURI, world, step), BRATAC_TAG);
        assert!(
            labels_in(&r, M1651_GATE_CHAINS).is_empty(),
            "step {step}, world {world:?}"
        );
    }
    // The grant is a click, never a load (F44).
    for world_name in [DAKARA_NAME, STORY_RM_NAME] {
        let r = fire_player_loaded(
            &engine,
            &report_ctx(TAURI, Some(STORY_RM), 4932),
            world_name,
        );
        assert!(
            labels_in(&r, M1651_GATE_CHAINS).is_empty(),
            "load {world_name}"
        );
    }
}
