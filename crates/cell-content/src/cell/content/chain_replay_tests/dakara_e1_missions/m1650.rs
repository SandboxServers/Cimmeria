//! DK-15: mission 1650 "Confront Moh'katan" (dakara_e1_m1650_chains.sql, 8250-8289).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_mission_completed,
    fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::actions::Action;

/// Moh'katan's tag (template 54, world 62).
const MOHKATAN_TAG: &str = "Dakara_E1_StoryRm_Mohkatan";
/// Bra'tac's tag (template 59, world 62).
const BRATAC_TAG: &str = "Dakara_E1_StoryRm_Bratac";
/// The command-tent terminal's tag (prop 454, world 62).
const TERMINAL_TAG: &str = "Dakara_E1_StoryRm_MohkatanTerminal";
/// The Moh'katan tent point set (2124, world 61).
const TENT_REGION: &str = "Dakara_E1.MohkatanTent";

/// A context in which 1645, 1646, 1648 and 1649 are completed: the state
/// every 1650 chain runs in.
fn done_before_1650(world: Option<i32>) -> ExecutionContext {
    [1645, 1646, 1648, 1649]
        .into_iter()
        .fold(ctx_for(TAURI, world), |c, m| {
            with_mission(c, m, "completed")
        })
}

/// 1649 completed and 1650 not yet active: the state in which Bra'tac offers 1650.
fn offered(world: Option<i32>) -> ExecutionContext {
    with_mission(done_before_1650(world), 1650, "not_active")
}

/// 1650 active at `step`.
fn at_step(world: Option<i32>, step: i32) -> ExecutionContext {
    with_step(
        with_mission(done_before_1650(world), 1650, "active"),
        1650,
        step,
        "active",
    )
}

#[tokio::test]
async fn live_db_offer_follows_the_1649_turn_in_in_the_tent() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_mission_completed(&engine, &offered(Some(STORY_RM)), 1649);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec!["8250:add_dialog_set(6767@59)"],
    );
    let on_dakara = fire_mission_completed(&engine, &offered(Some(DAKARA)), 1649);
    assert!(labels_in(&on_dakara, M1650_CHAINS).is_empty());
    let active = with_mission(offered(Some(STORY_RM)), 1650, "active");
    let r = fire_mission_completed(&engine, &active, 1649);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_offer_restore_binds_bratac_after_a_relog() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &offered(Some(STORY_RM)), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec!["8251:add_dialog_set(6767@59)"],
    );
    // 1649 not yet completed: no offer yet.
    let before_1649 = with_mission(offered(Some(STORY_RM)), 1649, "active");
    let r = fire_player_loaded(&engine, &before_1649, STORY_RM_NAME);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
    let r = fire_player_loaded(&engine, &offered(Some(DAKARA)), DAKARA_NAME);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_accept_choice_takes_1650_and_briefs() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_dialog_choice(&engine, &offered(Some(STORY_RM)), 5985);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec![
            "8252:accept_mission(1650)",
            "8252:display_dialog(5839)",
            "8252:remove_dialog_set(6767@59)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8252);
    // Not in the tent, or with 1649 still open, the choice does nothing.
    let on_dakara = fire_dialog_choice(&engine, &offered(Some(DAKARA)), 5985);
    assert!(labels_in(&on_dakara, M1650_CHAINS).is_empty());
    let before_1649 = with_mission(offered(Some(STORY_RM)), 1649, "active");
    let r = fire_dialog_choice(&engine, &before_1649, 5985);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
    let accepted = at_step(Some(STORY_RM), 4927);
    let r = fire_dialog_choice(&engine, &accepted, 5985);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_tent_region_advances_to_the_confrontation() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let at_4927 = at_step(Some(DAKARA), 4927);
    let r = fire_enter_region(&engine, &at_4927, TENT_REGION);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec!["8253:advance_step(1650->4924)"],
    );
    assert_no_deferred_actions(&r, 8253);
    // The command tent is a different region; 4924 has already advanced.
    let r = fire_enter_region(&engine, &at_4927, "Dakara_E1.CommandTent");
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
    let r = fire_enter_region(&engine, &at_step(Some(DAKARA), 4924), TENT_REGION);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
    let r = fire_enter_region(&engine, &at_step(Some(STORY_RM), 4927), TENT_REGION);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_entering_the_tent_at_4924_binds_moh_katan_and_spawns_the_hologram() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4924), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec![
            "8254:add_dialog_set(6768@54)",
            "8254:spawn_entity(443#Dakara_E1_StoryRm_BaalHologram)",
        ],
    );
    for step in [4927, 4926] {
        let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), step), STORY_RM_NAME);
        assert!(labels_in(&r, M1650_CHAINS)
            .iter()
            .all(|l| !l.starts_with("8254:")));
    }
    let r = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4924), DAKARA_NAME);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
}

// The seed's heading is the packet's literal 1.5708, not `FRAC_PI_2`.
#[allow(clippy::approx_constant)]
#[tokio::test]
async fn live_db_hologram_spawn_is_at_b07_and_not_shared() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4924), STORY_RM_NAME);
    let (position, heading, allow_shared) = r
        .actions
        .iter()
        .find_map(|(id, a)| match a {
            Action::SpawnEntity {
                position,
                heading,
                allow_shared,
                ..
            } if *id == 8254 => Some((*position, *heading, *allow_shared)),
            _ => None,
        })
        .expect("8254 spawns the hologram");
    for (got, want) in position.iter().zip([66.2_f32, -0.12, 34.3]) {
        assert!((got - want).abs() < 0.01, "position {position:?}");
    }
    assert!((heading - 1.5708).abs() < 0.001, "heading {heading}");
    assert_ne!(allow_shared, Some(true));
}

#[tokio::test]
async fn live_db_moh_katan_click_starts_the_confrontation() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4924), MOHKATAN_TAG);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec![
            "8255:display_dialog(5840)",
            "8255:advance_step(1650->4926)",
            "8255:remove_dialog_set(6768@54)",
            "8255:add_dialog_set(6760@454)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8255);
    for ctx in [
        at_step(Some(STORY_RM), 4926),
        at_step(Some(STORY_RM), 4927),
        at_step(Some(DAKARA), 4924),
    ] {
        let r = fire_interact_tag(&engine, &ctx, MOHKATAN_TAG);
        assert!(labels_in(&r, M1650_CHAINS).is_empty());
    }
}

#[tokio::test]
async fn live_db_confrontation_choice_sends_bratac_on_the_radio() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_dialog_choice(&engine, &at_step(Some(STORY_RM), 4926), 5840);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec!["8256:display_radio_dialog(5841)"],
    );
    assert_eq!(radio_count(&r), 1);
    let r = fire_dialog_choice(&engine, &at_step(Some(STORY_RM), 4924), 5840);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
    let r = fire_dialog_choice(&engine, &at_step(Some(DAKARA), 4926), 5840);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_step_4926_restore_rebinds_the_terminal_and_the_hologram() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4926), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec![
            "8257:add_dialog_set(6760@454)",
            "8257:spawn_entity(443#Dakara_E1_StoryRm_BaalHologram)",
        ],
    );
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4925), STORY_RM_NAME);
    assert!(labels_in(&r, M1650_CHAINS)
        .iter()
        .all(|l| !l.starts_with("8257:")));
}

#[tokio::test]
async fn live_db_terminal_gives_the_file_and_removes_the_hologram() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4926), TERMINAL_TAG);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec![
            "8258:add_item(6787x1)",
            "8258:advance_step(1650->4925)",
            "8258:remove_dialog_set(6760@454)",
            "8258:despawn_entity(Dakara_E1_StoryRm_BaalHologram)",
            "8258:add_dialog_set(6771@59)",
        ],
    );
    // The terminal shows no dialog: the item and the step change are its feedback.
    assert_eq!(display_dialog_count(&r), 0);
    assert_no_deferred_actions(&r, 8258);
    // The despawn comes before Bra'tac's bind, as the packet orders it.
    let labels = labels_in(&r, M1650_CHAINS);
    let despawn = labels
        .iter()
        .position(|l| l == "8258:despawn_entity(Dakara_E1_StoryRm_BaalHologram)");
    let bind = labels
        .iter()
        .position(|l| l == "8258:add_dialog_set(6771@59)");
    assert!(despawn < bind);
    for ctx in [at_step(Some(STORY_RM), 4924), at_step(Some(STORY_RM), 4925)] {
        let r = fire_interact_tag(&engine, &ctx, TERMINAL_TAG);
        assert!(labels_in(&r, M1650_CHAINS).is_empty());
    }
    let r = fire_interact_tag(&engine, &at_step(Some(DAKARA), 4926), TERMINAL_TAG);
    assert!(labels_in(&r, M1650_CHAINS).is_empty());
}

#[tokio::test]
async fn live_db_step_4925_restore_rebinds_bratac() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4925), STORY_RM_NAME);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec!["8259:add_dialog_set(6771@59)"],
    );
    let r = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4926), STORY_RM_NAME);
    assert!(labels_in(&r, M1650_CHAINS)
        .iter()
        .all(|l| !l.starts_with("8259:")));
}

#[tokio::test]
async fn live_db_bratac_turn_in_completes_1650_after_unbinding() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &at_step(Some(STORY_RM), 4925), BRATAC_TAG);
    assert_eq!(
        labels_in(&r, M1650_CHAINS),
        vec![
            "8260:display_dialog(5842)",
            "8260:remove_item(6787x1)",
            "8260:remove_dialog_set(6771@59)",
            "8260:complete_mission(1650)",
        ],
    );
    assert_eq!(display_dialog_count(&r), 1);
    assert_no_deferred_actions(&r, 8260);
    for ctx in [
        at_step(Some(STORY_RM), 4924),
        at_step(Some(STORY_RM), 4926),
        at_step(Some(DAKARA), 4925),
    ] {
        let r = fire_interact_tag(&engine, &ctx, BRATAC_TAG);
        assert!(labels_in(&r, M1650_CHAINS).is_empty());
    }
}
