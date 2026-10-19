//! DK-30b: mission 1651 "Renewed Attack", step 4930 (dakara_e1_m1651_gate_chains.sql, 8310-8319).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The DHD (template 1, world 61).
const DHD_TAG: &str = "Dakara_E1_DHD";
/// A tag on Dakara that is not the DHD (the command terminal, DK-41).
const TERMINAL_TAG: &str = "Dakara_E1_CommandTerminal";
/// The SGC H3 arrival, as the labels print it.
const SGC_H3: &str = "cross_world_teleport(SGC_W1@172.79,-10.90,12.00)";

/// A player on 1651 at `step`, mission and step both `active`.
fn at_step(world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(TAURI, world), 1651, "active"),
        1651,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_dhd_at_4930_advances_to_4928_then_teleports() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_step(Some(DAKARA), 4930), DHD_TAG);
    assert_eq!(
        labels_in(&r, M1651_GATE_CHAINS),
        [
            "8310:advance_step(1651->4928)".to_string(),
            format!("8310:{SGC_H3}"),
        ],
    );
    assert_no_deferred_actions(&r, 8310);
    assert_eq!(display_dialog_count(&r), 0);
}

#[tokio::test]
async fn live_db_dhd_recovery_at_4928_and_4931_teleports_without_advance() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let at_4928 = fire_interact_tag(&engine, &at_step(Some(DAKARA), 4928), DHD_TAG);
    assert_eq!(
        labels_in(&at_4928, M1651_GATE_CHAINS),
        [format!("8311:{SGC_H3}")],
    );
    let at_4931 = fire_interact_tag(&engine, &at_step(Some(DAKARA), 4931), DHD_TAG);
    assert_eq!(
        labels_in(&at_4931, M1651_GATE_CHAINS),
        [format!("8312:{SGC_H3}")],
    );
}

#[tokio::test]
async fn live_db_gate_chains_stay_quiet_at_other_steps_worlds_and_tags() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for step in [4932, 4929, 5171] {
        let r = fire_interact_tag(&engine, &at_step(Some(DAKARA), step), DHD_TAG);
        assert!(labels_in(&r, M1651_GATE_CHAINS).is_empty(), "step {step}");
    }
    let on_story_rm = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4930), DHD_TAG);
    assert!(labels_in(&on_story_rm, M1651_GATE_CHAINS).is_empty());
    let other_tag = fire_interact_tag(&engine, &at_step(Some(DAKARA), 4930), TERMINAL_TAG);
    assert!(labels_in(&other_tag, M1651_GATE_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_dhd_without_1651_state_resolves_no_gate_chain() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // No 1651 state at all: the DHD click resolves nothing here, so the
    // dialling window still opens.
    let ctx = ctx_for(TAURI, Some(DAKARA));
    let r = fire_interact_tag(&engine, &ctx, DHD_TAG);
    assert!(labels_in(&r, M1651_GATE_CHAINS).is_empty());
}
