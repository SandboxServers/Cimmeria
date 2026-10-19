//! DK-10a: mission 1570 entry, the Free Jaffa who finish the bridge mission 63100
//! (dakara_e1_m1570_entry_chains.sql, 8040-8044).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, fire_mission_completed, fire_player_loaded, with_mission,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// The bridge mission that starts 1570 (D-DK1 option F).
const BRIDGE_MISSION: i32 = 63100;

/// A context in which mission 63100 already reads `completed`, as the runtime sees it.
fn done(archetype: i64, world: Option<i32>) -> ExecutionContext {
    with_mission(ctx_for(archetype, world), BRIDGE_MISSION, "completed")
}

/// Replay the `mission_completed` dispatch for mission 63100.
fn fire(engine: &ChainEngine, ctx: &ExecutionContext) -> ResolvedActions {
    fire_mission_completed(engine, ctx, BRIDGE_MISSION)
}

#[tokio::test]
async fn live_db_finishing_home_to_dakara_starts_1570_at_bratac() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire(&engine, &done(FREE_JAFFA, Some(DAKARA)));
    assert_eq!(
        labels_in(&r, ENTRY_CHAINS),
        vec![
            "8040:accept_mission(1570)",
            "8040:advance_step(1570->4642)",
            "8040:advance_step(1570->4906)",
            "8040:complete_objective(1570/5936)",
        ],
    );
    assert_no_deferred_actions(&r, 8040);
}

#[tokio::test]
async fn live_db_no_other_archetype_is_given_1570() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let praxis = fire(&engine, &done(PRAXIS_JAFFA, Some(DAKARA)));
    let tauri = fire(&engine, &done(TAURI, Some(DAKARA)));
    assert!(labels_in(&praxis, ENTRY_CHAINS).is_empty());
    assert!(labels_in(&tauri, ENTRY_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_1570_is_given_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let active = with_mission(done(FREE_JAFFA, Some(DAKARA)), 1570, "active");
    let completed = with_mission(done(FREE_JAFFA, Some(DAKARA)), 1570, "completed");
    assert!(labels_in(&fire(&engine, &active), ENTRY_CHAINS).is_empty());
    assert!(labels_in(&fire(&engine, &completed), ENTRY_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_1570_entry_needs_dakara_and_63100() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let on_story_rm = fire(&engine, &done(FREE_JAFFA, Some(STORY_RM)));
    let no_world = fire(&engine, &done(FREE_JAFFA, None));
    assert!(labels_in(&on_story_rm, ENTRY_CHAINS).is_empty());
    assert!(labels_in(&no_world, ENTRY_CHAINS).is_empty());

    // Another mission completing does not start 1570.
    let other_mission = fire_mission_completed(
        &engine,
        &with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1324, "completed"),
        1324,
    );
    assert!(labels_in(&other_mission, ENTRY_CHAINS).is_empty());

    // Arrival alone gives no 1570 under option F.
    let arrival = fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, Some(DAKARA)), DAKARA_NAME);
    assert!(labels_in(&arrival, ENTRY_CHAINS).is_empty());
}
