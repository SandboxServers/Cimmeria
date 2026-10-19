//! Beta Site E1 Intriguing Data (UB-33, mission 1147): the blurb bind (23390),
//! the offer (23391), the Accept (23392), the analytical machine (23393), the
//! relog restore (23394) and the Mimir turn-in (23395). Seeded in
//! `db/resources/Content/Seed/beta_e1_m1147_chains.sql`.
//!
//! Omega UO-16 is HELD (HELD-B16), so no live Asgard reaches Mimir through
//! Beta yet. These tests replay archetype 5 (D-UB13).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, ASGARD, BETA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const BLURB: i64 = 23390;
const OFFER: i64 = 23391;
const ACCEPT: i64 = 23392;
const MACHINE: i64 = 23393;
const RESTORE: i64 = 23394;
const TURN_IN: i64 = 23395;
const M1147_CHAINS: std::ops::RangeInclusive<i64> = 23390..=23395;
const MIMIR: &str = "BetaE1_Mimir";
const MACHINE_TAG: &str = "BetaE1_AnalyticalMachine";
const OFFER_DIALOG: i64 = 3904;
const TURN_IN_DIALOG: i64 = 3905;

/// The mission 1147 chains that `resolved` matched.
fn m1147_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M1147_CHAINS.contains(id))
        .collect()
}

/// An `interact_tag` context on `tag`. `None` leaves the archetype unset.
fn click(tag: &str, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// A `dialog_choice` context on `dialog_id`. `None` leaves the archetype unset.
fn choice(dialog_id: i64, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// A `mission_completed` context on `mission_id`. `None` leaves the archetype unset.
fn completed(mission_id: i64, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(mission_id));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// Every race that is not Asgard: `None` and archetypes 0-4, 6-8.
fn non_asgard() -> Vec<Option<i64>> {
    std::iter::once(None)
        .chain(
            ALL_ARCHETYPES
                .iter()
                .copied()
                .filter(|a| *a != ASGARD)
                .map(Some),
        )
        .collect()
}

/// Missions 481 and 420 done, 421 done, 1147 not started: the state that opens the offer.
fn set_offer_open(ctx: &mut ExecutionContext) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "completed");
    with_mission(ctx, 421, "completed");
    with_mission(ctx, 1147, "not_active");
}

/// Missions 481, 420 and 421 done and 1147 active at `step`.
fn set_step(ctx: &mut ExecutionContext, step: i32) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "completed");
    with_mission(ctx, 421, "completed");
    with_mission(ctx, 1147, "active");
    with_step(ctx, 1147, step, "active");
}

/// Missions 481, 420 and 421 done, and 1147 done.
fn set_finished(ctx: &mut ExecutionContext) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "completed");
    with_mission(ctx, 421, "completed");
    with_mission(ctx, 1147, "completed");
}

/// Mission 421 still active at S1464 and 1147 not started: the offer is closed.
fn set_421_open(ctx: &mut ExecutionContext) {
    with_mission(ctx, 421, "active");
    with_mission(ctx, 1147, "not_active");
}

/// Revert: drop 23390's `mission_completed 421` trigger row or its 421 gate.
/// Then the blurb bind is missing after 421 completes in the same session.
#[tokio::test]
async fn blurb_binds_on_login_and_on_421_completion() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut login = login_ctx(BETA, Some(ASGARD));
    set_offer_open(&mut login);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(4588, slot=2305)".to_string()],
    );
    assert_no_deferred_actions(&resolved, BLURB);
    assert_eq!(m1147_matched(&resolved), BTreeSet::from([BLURB]));

    let mut done = completed(421, Some(ASGARD));
    set_offer_open(&mut done);
    let resolved = fire(&engine, TriggerType::MissionCompleted, &done);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(4588, slot=2305)".to_string()],
    );
    assert_no_deferred_actions(&resolved, BLURB);

    let mut other_world = login_ctx("Some_Other_World", Some(ASGARD));
    set_offer_open(&mut other_world);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &other_world);
    assert!(m1147_matched(&resolved).is_empty());
}

/// Revert: drop the 421 gate or the 1147 `not_active` gate of 23390. The blurb
/// then binds for a player who has not finished 421, or who is already in 1147.
#[tokio::test]
async fn blurb_bind_ignores_other_races_and_states() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut login = login_ctx(BETA, archetype);
        set_offer_open(&mut login);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
        assert!(m1147_matched(&resolved).is_empty(), "login {archetype:?}");

        let mut done = completed(421, archetype);
        set_offer_open(&mut done);
        let resolved = fire(&engine, TriggerType::MissionCompleted, &done);
        assert!(m1147_matched(&resolved).is_empty(), "421 {archetype:?}");
    }
    let mut open_421 = login_ctx(BETA, Some(ASGARD));
    set_421_open(&mut open_421);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &open_421);
    assert!(m1147_matched(&resolved).is_empty());
    let mut started = login_ctx(BETA, Some(ASGARD));
    set_step(&mut started, 3458);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &started);
    assert!(m1147_matched(&resolved).is_empty());
    let mut finished = login_ctx(BETA, Some(ASGARD));
    set_finished(&mut finished);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &finished);
    assert!(m1147_matched(&resolved).is_empty());
}

/// Revert: drop 23391's `display_dialog` or its 421 gate. Then Mimir opens
/// nothing, or opens the offer before 421 is done.
#[tokio::test]
async fn offer_shows_to_asgard_once_421_is_done() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_offer_open(&mut ctx);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, OFFER),
        vec!["display_dialog(3904)".to_string()],
    );
    assert_no_deferred_actions(&resolved, OFFER);
    assert_eq!(m1147_matched(&resolved), BTreeSet::from([OFFER]));
    for archetype in non_asgard() {
        let mut ctx = click(MIMIR, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, OFFER).is_empty(), "{archetype:?}");
    }
    let mut not_yet = click(MIMIR, Some(ASGARD));
    set_421_open(&mut not_yet);
    let resolved = fire(&engine, TriggerType::InteractTag, &not_yet);
    assert!(m1147_matched(&resolved).is_empty());
    let mut started = click(MIMIR, Some(ASGARD));
    set_step(&mut started, 3458);
    let resolved = fire(&engine, TriggerType::InteractTag, &started);
    assert!(m1147_matched(&resolved).is_empty());
}

/// Revert: drop 23392's `remove_dialog_set` or its `accept_mission`. Then the
/// Accept leaves 1147 unstarted, or leaves the blurb bound on Mimir.
#[tokio::test]
async fn accept_starts_1147_and_drops_the_blurb() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = choice(OFFER_DIALOG, Some(ASGARD));
    set_offer_open(&mut ctx);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_eq!(
        summarized(&resolved, ACCEPT),
        vec![
            "accept_mission(1147)".to_string(),
            "remove_dialog_set(4588, slot=2305)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, ACCEPT);
    assert_eq!(m1147_matched(&resolved), BTreeSet::from([ACCEPT]));
    for archetype in non_asgard() {
        let mut ctx = choice(OFFER_DIALOG, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
        assert!(summarized(&resolved, ACCEPT).is_empty(), "{archetype:?}");
    }
    let mut wrong_dialog = choice(TURN_IN_DIALOG, Some(ASGARD));
    set_offer_open(&mut wrong_dialog);
    let resolved = fire(&engine, TriggerType::DialogChoice, &wrong_dialog);
    assert!(m1147_matched(&resolved).is_empty());
    let mut active = choice(OFFER_DIALOG, Some(ASGARD));
    set_step(&mut active, 3458);
    let resolved = fire(&engine, TriggerType::DialogChoice, &active);
    assert!(m1147_matched(&resolved).is_empty());
}

/// Revert: drop 23393's `advance_step` or its S3458 gate. Then the machine click
/// never reaches S3459, or fires at the wrong step.
#[tokio::test]
async fn machine_click_advances_to_s3459_and_binds_the_turn_in() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MACHINE_TAG, Some(ASGARD));
    set_step(&mut ctx, 3458);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, MACHINE),
        vec![
            "advance_step(1147, 3459)".to_string(),
            "add_dialog_set(4590, slot=2305)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, MACHINE);
    assert_eq!(m1147_matched(&resolved), BTreeSet::from([MACHINE]));
    for archetype in non_asgard() {
        let mut ctx = click(MACHINE_TAG, archetype);
        set_step(&mut ctx, 3458);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, MACHINE).is_empty(), "{archetype:?}");
    }
    let mut s3459 = click(MACHINE_TAG, Some(ASGARD));
    set_step(&mut s3459, 3459);
    let resolved = fire(&engine, TriggerType::InteractTag, &s3459);
    assert!(m1147_matched(&resolved).is_empty());
}

/// Revert: drop 23394 or its S3459 gate. Then a relog at S3459 leaves Mimir
/// with no turn-in bind, and a relog at S3458 binds one too early.
#[tokio::test]
async fn relog_at_s3459_restores_the_turn_in_bind() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut relog = login_ctx(BETA, Some(ASGARD));
    set_step(&mut relog, 3459);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &relog);
    assert_eq!(
        summarized(&resolved, RESTORE),
        vec!["add_dialog_set(4590, slot=2305)".to_string()],
    );
    assert_no_deferred_actions(&resolved, RESTORE);
    assert_eq!(m1147_matched(&resolved), BTreeSet::from([RESTORE]));
    let mut before = login_ctx(BETA, Some(ASGARD));
    set_step(&mut before, 3458);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &before);
    assert!(m1147_matched(&resolved).is_empty());
    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_step(&mut ctx, 3459);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(m1147_matched(&resolved).is_empty(), "{archetype:?}");
    }
}

/// Revert: drop the `Beta_Site_Evo_1` event key from 23394. An Asgard who logs
/// into another world at S3459 then gets the turn-in bind there.
#[tokio::test]
async fn relog_restore_ignores_other_worlds() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut relog = login_ctx("Castle_Cellblock", Some(ASGARD));
    set_step(&mut relog, 3459);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &relog);
    assert!(m1147_matched(&resolved).is_empty());
}

/// Revert: drop 23395's `complete_mission` or its `remove_dialog_set`. Then the
/// turn-in leaves the mission open, or leaves the bind on Mimir.
#[tokio::test]
async fn mimir_turn_in_completes_1147() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_step(&mut ctx, 3459);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, TURN_IN),
        vec![
            "display_dialog(3905)".to_string(),
            "remove_dialog_set(4590, slot=2305)".to_string(),
            "complete_mission(1147)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, TURN_IN);
    assert_eq!(m1147_matched(&resolved), BTreeSet::from([TURN_IN]));
    let mut early = click(MIMIR, Some(ASGARD));
    set_step(&mut early, 3458);
    let resolved = fire(&engine, TriggerType::InteractTag, &early);
    assert!(m1147_matched(&resolved).is_empty());
    for archetype in non_asgard() {
        let mut ctx = click(MIMIR, archetype);
        set_step(&mut ctx, 3459);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, TURN_IN).is_empty(), "{archetype:?}");
    }
}

/// Revert: any 1147 chain that loses its archetype `eq 5` gate matches a
/// non-Asgard race on the same event. Every positive 1147 event is replayed for
/// each non-Asgard race and must match no 1147 chain.
#[tokio::test]
async fn wrong_race_fires_no_1147_chain() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut login = login_ctx(BETA, archetype);
        set_offer_open(&mut login);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
        assert!(m1147_matched(&resolved).is_empty(), "login {archetype:?}");

        let mut done = completed(421, archetype);
        set_offer_open(&mut done);
        let resolved = fire(&engine, TriggerType::MissionCompleted, &done);
        assert!(m1147_matched(&resolved).is_empty(), "421 {archetype:?}");

        let mut offer = click(MIMIR, archetype);
        set_offer_open(&mut offer);
        let resolved = fire(&engine, TriggerType::InteractTag, &offer);
        assert!(m1147_matched(&resolved).is_empty(), "offer {archetype:?}");

        let mut accept = choice(OFFER_DIALOG, archetype);
        set_offer_open(&mut accept);
        let resolved = fire(&engine, TriggerType::DialogChoice, &accept);
        assert!(m1147_matched(&resolved).is_empty(), "accept {archetype:?}");

        let mut machine = click(MACHINE_TAG, archetype);
        set_step(&mut machine, 3458);
        let resolved = fire(&engine, TriggerType::InteractTag, &machine);
        assert!(m1147_matched(&resolved).is_empty(), "machine {archetype:?}");

        let mut relog = login_ctx(BETA, archetype);
        set_step(&mut relog, 3459);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &relog);
        assert!(m1147_matched(&resolved).is_empty(), "relog {archetype:?}");

        let mut turn_in = click(MIMIR, archetype);
        set_step(&mut turn_in, 3459);
        let resolved = fire(&engine, TriggerType::InteractTag, &turn_in);
        assert!(m1147_matched(&resolved).is_empty(), "turn-in {archetype:?}");
    }
}

/// Revert: add a sibling on the Mimir tag or the machine tag that fires in the
/// same state. The whole seed must match exactly one 1147 chain per state.
#[tokio::test]
async fn mimir_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut offer = click(MIMIR, Some(ASGARD));
    set_offer_open(&mut offer);
    let resolved = fire(&engine, TriggerType::InteractTag, &offer);
    assert_eq!(chains_matched(&resolved), BTreeSet::from([OFFER]));

    let mut s3458 = click(MIMIR, Some(ASGARD));
    set_step(&mut s3458, 3458);
    let resolved = fire(&engine, TriggerType::InteractTag, &s3458);
    assert!(chains_matched(&resolved).is_empty());

    let mut turn_in = click(MIMIR, Some(ASGARD));
    set_step(&mut turn_in, 3459);
    let resolved = fire(&engine, TriggerType::InteractTag, &turn_in);
    assert_eq!(chains_matched(&resolved), BTreeSet::from([TURN_IN]));

    let mut finished = click(MIMIR, Some(ASGARD));
    set_finished(&mut finished);
    let resolved = fire(&engine, TriggerType::InteractTag, &finished);
    assert!(chains_matched(&resolved).is_empty());

    let mut machine = click(MACHINE_TAG, Some(ASGARD));
    set_step(&mut machine, 3458);
    let resolved = fire(&engine, TriggerType::InteractTag, &machine);
    assert_eq!(chains_matched(&resolved), BTreeSet::from([MACHINE]));

    let mut machine_done = click(MACHINE_TAG, Some(ASGARD));
    set_step(&mut machine_done, 3459);
    let resolved = fire(&engine, TriggerType::InteractTag, &machine_done);
    assert!(chains_matched(&resolved).is_empty());
}
