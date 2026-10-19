//! DK-10fc: the bridge mission 63100 "Home to Dakara" (dakara_e1_bridge_chains.sql, 8075-8079).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_mission_completed, fire_player_loaded,
    with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The bridge mission and its one step (D-DK1 option F).
const BRIDGE_MISSION: i32 = 63100;
const BRIDGE_STEP: i32 = 63101;
/// Rak'nor's spawn tag (spawnlist_dakara_e1.sql, spawn 8006).
const RAKNOR_TAG: &str = "Dakara_E1_Raknor";

/// A Free Jaffa on Dakara with 63100 at its step 63101, as the runtime sees it.
fn at_bridge_step() -> ExecutionContext {
    with_step(
        with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), BRIDGE_MISSION, "active"),
        BRIDGE_MISSION,
        BRIDGE_STEP,
        "active",
    )
}

#[tokio::test]
async fn live_db_free_jaffa_first_arrival_accepts_home_to_dakara() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, Some(DAKARA)), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, BRIDGE_CHAINS),
        vec![
            "8075:accept_mission(63100)",
            "8075:add_dialog_set(6890@441)",
        ],
    );
    assert_no_deferred_actions(&r, 8075);
}

#[tokio::test]
async fn live_db_home_to_dakara_restores_raknor() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_bridge_step(), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, BRIDGE_CHAINS),
        vec!["8076:add_dialog_set(6890@441)"],
    );
}

#[tokio::test]
async fn live_db_raknor_completes_home_to_dakara() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_bridge_step(), RAKNOR_TAG);
    assert_eq!(
        labels_in(&r, BRIDGE_CHAINS),
        vec![
            "8077:display_dialog(6110)",
            "8077:remove_dialog_set(6890@441)",
            "8077:complete_mission(63100)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8077);
}

#[tokio::test]
async fn live_db_home_to_dakara_hands_over_to_1570() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let done = with_mission(
        ctx_for(FREE_JAFFA, Some(DAKARA)),
        BRIDGE_MISSION,
        "completed",
    );
    let r = fire_mission_completed(&engine, &done, BRIDGE_MISSION);
    assert_eq!(
        labels_in(&r, ENTRY_CHAINS),
        vec![
            "8040:accept_mission(1570)",
            "8040:advance_step(1570->4642)",
            "8040:advance_step(1570->4906)",
            "8040:complete_objective(1570/5936)",
        ],
    );
    assert!(labels_in(&r, BRIDGE_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_home_to_dakara_is_free_jaffa_only_and_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // Each of these is off the bridge on the load and on the Rak'nor click.
    let wrong_states = vec![
        ctx_for(PRAXIS_JAFFA, Some(DAKARA)),
        ctx_for(TAURI, Some(DAKARA)),
        with_mission(
            ctx_for(FREE_JAFFA, Some(DAKARA)),
            BRIDGE_MISSION,
            "completed",
        ),
        with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1570, "active"),
        with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1570, "completed"),
        with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1645, "active"),
    ];
    for ctx in &wrong_states {
        let load = fire_player_loaded(&engine, ctx, DAKARA_NAME);
        let click = fire_interact_tag(&engine, ctx, RAKNOR_TAG);
        assert!(labels_in(&load, BRIDGE_CHAINS).is_empty());
        assert!(labels_in(&click, BRIDGE_CHAINS).is_empty());
    }
    // A Free Jaffa on Story Room gets no bridge on that world's load.
    let story_rm = fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, Some(STORY_RM)), STORY_RM_NAME);
    assert!(labels_in(&story_rm, BRIDGE_CHAINS).is_empty());
    // A Free Jaffa with nothing set gets no bridge from the Rak'nor click.
    let nothing_set = fire_interact_tag(&engine, &ctx_for(FREE_JAFFA, Some(DAKARA)), RAKNOR_TAG);
    assert!(labels_in(&nothing_set, BRIDGE_CHAINS).is_empty());
}

/// The sorted chain ids of every resolved action, one entry per action.
fn sorted_ids(r: &ResolvedActions) -> Vec<i64> {
    let mut ids: Vec<i64> = r.actions.iter().map(|(id, _)| *id).collect();
    ids.sort();
    ids
}

/// Pins the whole Free Jaffa arrival set, unfiltered by range, on purpose.
/// The 8075 bridge is the deliberate addition to a Free Jaffa's arrival, and
/// the context without a world id is the one `dakara_e1_space`'s `arrive()`
/// builds, which is why its resolve tests never see 8075.
#[tokio::test]
async fn live_db_free_jaffa_first_arrival_resolves_exactly_the_space_and_bridge_chains() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let on_dakara = fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, Some(DAKARA)), DAKARA_NAME);
    assert_eq!(sorted_ids(&on_dakara), vec![8001, 8002, 8007, 8075, 8075]);
    let no_world = fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, None), DAKARA_NAME);
    assert_eq!(sorted_ids(&no_world), vec![8001, 8002, 8007]);
}
