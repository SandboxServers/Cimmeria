//! DK-10d-B: mission 1570 step 4903, Bra'tac's turn-in (dakara_e1_m1570_report_chains.sql, 8164-8165).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// Bra'tac's click tag in the story room (world 62, template 59).
const BRATAC_TAG: &str = "Dakara_E1_StoryRm_Bratac";

/// A context at `step` of mission 1570 (active), for `archetype` in `world`.
fn at_step(archetype: i64, world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(archetype, world), 1570, "active"),
        1570,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_bratac_bind_on_story_room_entry_at_4903() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [FREE_JAFFA, TAURI] {
        let ctx = at_step(archetype, Some(STORY_RM), 4903);
        let r = fire_player_loaded(&engine, &ctx, STORY_RM_NAME);
        assert_eq!(
            labels_in(&r, REPORT_CHAINS),
            vec!["8164:add_dialog_set(6733@59)"],
        );
        assert_no_deferred_actions(&r, 8164);
    }
}

#[tokio::test]
async fn live_db_bratac_bind_is_only_for_step_4903_in_world_62() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // DK-10d-A's 8170 binds Bra'tac at 4905 by design, so this negative
    // checks 8164 alone.
    let at_4905 = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4905),
        STORY_RM_NAME,
    );
    assert!(labels_in(&at_4905, 8164..=8164).is_empty());
    let at_4906 = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4906),
        STORY_RM_NAME,
    );
    assert!(labels_in(&at_4906, REPORT_CHAINS).is_empty());
    let completed = fire_player_loaded(
        &engine,
        &with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1570, "completed"),
        STORY_RM_NAME,
    );
    assert!(labels_in(&completed, REPORT_CHAINS).is_empty());
    let in_dakara = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4903),
        STORY_RM_NAME,
    );
    assert!(labels_in(&in_dakara, REPORT_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_bratac_turn_in_at_4903() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4903),
        BRATAC_TAG,
    );
    assert_eq!(
        labels_in(&r, REPORT_CHAINS),
        vec![
            "8165:display_dialog(5817)",
            "8165:remove_dialog_set(6733@59)",
            "8165:complete_mission(1570)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8165);
}

#[tokio::test]
async fn live_db_bratac_turn_in_is_only_at_4903_in_world_62() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let at_4906 = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4906),
        BRATAC_TAG,
    );
    let at_4905 = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4905),
        BRATAC_TAG,
    );
    let in_dakara = fire_interact_tag(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4903),
        BRATAC_TAG,
    );
    assert!(labels_in(&at_4906, REPORT_CHAINS).is_empty());
    assert!(labels_in(&at_4905, REPORT_CHAINS).is_empty());
    assert!(labels_in(&in_dakara, REPORT_CHAINS).is_empty());
}
