//! DK-10d-B: mission 1570 step 4903, Bra'tac's turn-in (dakara_e1_m1570_report_chains.sql, 8164-8165).
//!
//! DK-10d-A: mission 1570 step 4905, the radio report (8160-8163, 8166-8171).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_player_loaded,
    with_mission, with_step,
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

/// The number of `Action::DisplayRadioDialog` across the whole engine (the
/// radio counterpart of `display_dialog_count`).
fn radio_count(resolved: &ResolvedActions) -> usize {
    resolved
        .actions
        .iter()
        .filter(|(_, action)| matches!(action, Action::DisplayRadioDialog { .. }))
        .count()
}

/// The `delay_ms` of each action of `chain`, in order. `action_delays` is
/// index-aligned with `actions`; a missing entry reads 0.
fn delays_of(resolved: &ResolvedActions, chain: i64) -> Vec<i32> {
    resolved
        .actions
        .iter()
        .enumerate()
        .filter(|(_, (id, _))| *id == chain)
        .map(|(i, _)| resolved.action_delays.get(i).copied().unwrap_or(0))
        .collect()
}

/// The dog tags (6224 Louis, 6225 Nguyen, 6226 Waters), each with its Jaffa
/// chain and its Tau'ri chain.
const DOG_TAG_CHAINS: [(i32, i64, i64); 3] =
    [(6224, 8160, 8161), (6225, 8162, 8163), (6226, 8166, 8167)];

#[tokio::test]
async fn live_db_radio_report_on_dog_tag_close_at_4905() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (dialog, jaffa, tauri) in DOG_TAG_CHAINS {
        for archetype in [FREE_JAFFA, PRAXIS_JAFFA] {
            let r = fire_dialog_choice(&engine, &at_step(archetype, Some(DAKARA), 4905), dialog);
            assert_eq!(
                labels_in(&r, REPORT_CHAINS),
                vec![
                    format!("{jaffa}:display_radio_dialog(5816)"),
                    format!("{jaffa}:advance_step(1570->4903)"),
                ],
            );
            assert_eq!(radio_count(&r), 1);
            assert_eq!(display_dialog_count(&r), 0);
            assert_no_deferred_actions(&r, jaffa);
        }
        let r = fire_dialog_choice(&engine, &at_step(TAURI, Some(DAKARA), 4905), dialog);
        assert_eq!(
            labels_in(&r, REPORT_CHAINS),
            vec![
                format!("{tauri}:display_radio_dialog(5815)"),
                format!("{tauri}:advance_step(1570->4903)"),
            ],
        );
        assert_eq!(radio_count(&r), 1);
        assert_eq!(display_dialog_count(&r), 0);
        assert_no_deferred_actions(&r, tauri);
    }
}

#[tokio::test]
async fn live_db_radio_report_on_dog_tag_close_in_the_story_room_too() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_dialog_choice(&engine, &at_step(FREE_JAFFA, Some(STORY_RM), 4905), 6224);
    assert_eq!(
        labels_in(&r, REPORT_CHAINS),
        vec![
            "8160:display_radio_dialog(5816)",
            "8160:advance_step(1570->4903)",
        ],
    );
    assert_no_deferred_actions(&r, 8160);
}

#[tokio::test]
async fn live_db_radio_report_is_only_for_step_4905() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // Every dog tag, so a dropped step row on any of 8162/8163/8166/8167 is caught.
    for (dialog, _, _) in DOG_TAG_CHAINS {
        for (archetype, step) in [(FREE_JAFFA, 4901), (FREE_JAFFA, 4903), (TAURI, 4903)] {
            let r = fire_dialog_choice(&engine, &at_step(archetype, Some(DAKARA), step), dialog);
            assert!(labels_in(&r, REPORT_CHAINS).is_empty());
        }
        let completed = fire_dialog_choice(
            &engine,
            &with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1570, "completed"),
            dialog,
        );
        assert!(labels_in(&completed, REPORT_CHAINS).is_empty());
    }
}

#[tokio::test]
async fn live_db_radio_window_close_does_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // The radio's own close (EntityId 0) never moves the step: the advance
    // runs with the radio, not on its close.
    for dialog in [5816, 5815] {
        let r = fire_dialog_choice(&engine, &at_step(FREE_JAFFA, Some(DAKARA), 4903), dialog);
        assert!(labels_in(&r, REPORT_CHAINS).is_empty());
    }
}

#[tokio::test]
async fn live_db_radio_restore_on_dakara_load_at_4905() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (archetype, chain, dialog) in [
        (FREE_JAFFA, 8168, 5816),
        (PRAXIS_JAFFA, 8168, 5816),
        (TAURI, 8169, 5815),
    ] {
        let r = fire_player_loaded(
            &engine,
            &at_step(archetype, Some(DAKARA), 4905),
            DAKARA_NAME,
        );
        assert_eq!(
            labels_in(&r, REPORT_CHAINS),
            vec![
                format!("{chain}:display_radio_dialog({dialog})"),
                format!("{chain}:advance_step(1570->4903)"),
            ],
        );
        assert_eq!(delays_of(&r, chain), vec![2000, 0]);
        assert_eq!(radio_count(&r), 1);
    }
    for step in [4901, 4903] {
        let r = fire_player_loaded(
            &engine,
            &at_step(FREE_JAFFA, Some(DAKARA), step),
            DAKARA_NAME,
        );
        assert!(labels_in(&r, REPORT_CHAINS).is_empty());
    }
    // A Dakara-keyed load in the story room (world 62), and a completed
    // mission, must not restore the radio either.
    let in_story_room = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4905),
        DAKARA_NAME,
    );
    assert!(labels_in(&in_story_room, REPORT_CHAINS).is_empty());
    let completed = fire_player_loaded(
        &engine,
        &with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1570, "completed"),
        DAKARA_NAME,
    );
    assert!(labels_in(&completed, REPORT_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_radio_restore_in_the_story_room_at_4905() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (archetype, chain, dialog) in [(FREE_JAFFA, 8170, 5816), (TAURI, 8171, 5815)] {
        let r = fire_player_loaded(
            &engine,
            &at_step(archetype, Some(STORY_RM), 4905),
            STORY_RM_NAME,
        );
        assert_eq!(
            labels_in(&r, REPORT_CHAINS),
            vec![
                format!("{chain}:display_radio_dialog({dialog})"),
                format!("{chain}:advance_step(1570->4903)"),
                format!("{chain}:add_dialog_set(6733@59)"),
            ],
        );
        assert_eq!(delays_of(&r, chain), vec![2000, 0, 0]);
    }
    // At 4903 the same load binds Bra'tac through 8164 alone.
    let at_4903 = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(STORY_RM), 4903),
        STORY_RM_NAME,
    );
    assert_eq!(
        labels_in(&at_4903, REPORT_CHAINS),
        vec!["8164:add_dialog_set(6733@59)"],
    );
    // A story-room-keyed load in Dakara (world 61), and a completed mission,
    // restore nothing: the world and step rows gate both chains.
    let in_dakara = fire_player_loaded(
        &engine,
        &at_step(FREE_JAFFA, Some(DAKARA), 4905),
        STORY_RM_NAME,
    );
    assert!(labels_in(&in_dakara, REPORT_CHAINS).is_empty());
    let completed = fire_player_loaded(
        &engine,
        &with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1570, "completed"),
        STORY_RM_NAME,
    );
    assert!(labels_in(&completed, REPORT_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_report_radio_rows_and_dialog_shape() {
    let pool = require_db_or_skip!();
    // (a) The ten radio rows, one per chain, with the dialog each one shows.
    let rows: Vec<(i32, Option<i32>)> = sqlx::query_as(
        "SELECT chain_id, target_id FROM resources.content_actions \
         WHERE chain_id BETWEEN 8160 AND 8179 AND action_type::text = 'display_radio_dialog' \
         ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("radio rows");
    assert_eq!(
        rows,
        vec![
            (8160, Some(5816)),
            (8161, Some(5815)),
            (8162, Some(5816)),
            (8163, Some(5815)),
            (8166, Some(5816)),
            (8167, Some(5815)),
            (8168, Some(5816)),
            (8169, Some(5815)),
            (8170, Some(5816)),
            (8171, Some(5815)),
        ]
    );
    // (b) The radio and dog-tag dialogs have no buttons, so their only
    // response is the close (button -1), which 8160-8167 and the window-close
    // test rely on.
    let buttons: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.dialog_screen_buttons b \
         JOIN resources.dialog_screens s ON s.screen_id = b.screen_id \
         WHERE s.dialog_id IN (5815, 5816, 6224, 6225, 6226)",
    )
    .fetch_one(&pool)
    .await
    .expect("buttons");
    assert_eq!(buttons, 0);
    // (c) The screen counts of the two radio dialogs.
    let screens: Vec<(i32, i64)> = sqlx::query_as(
        "SELECT dialog_id, count(*) FROM resources.dialog_screens \
         WHERE dialog_id IN (5815, 5816) GROUP BY dialog_id ORDER BY dialog_id",
    )
    .fetch_all(&pool)
    .await
    .expect("screens");
    assert_eq!(screens, vec![(5815, 7), (5816, 10)]);
}

#[tokio::test]
async fn live_db_report_turn_in_tauri() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_step(TAURI, Some(STORY_RM), 4903), BRATAC_TAG);
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
