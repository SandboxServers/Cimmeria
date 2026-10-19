//! Beta Site E1 Mechanical Error (UB-32, mission 421): the blurb bind (23360),
//! the offer (23361), the Accept (23362), the region and emitter fallback
//! (23363, 23364), the emitter (23365), the drone (23366), the relog restore
//! (23367) and the Mimir turn-in (23368). Seeded in
//! `db/resources/Content/Seed/beta_e1_m421_chains.sql`.
//!
//! Omega UO-16 is HELD (HELD-B16), so no live Asgard reaches Mimir through
//! Beta yet. These tests replay archetype 5 (D-UB13).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, region_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, ASGARD, BETA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const BLURB: i64 = 23360;
const OFFER: i64 = 23361;
const ACCEPT: i64 = 23362;
const REGION: i64 = 23363;
const FALLBACK: i64 = 23364;
const EMITTER_SHOW: i64 = 23365;
const DRONE: i64 = 23366;
const RESTORE: i64 = 23367;
const TURN_IN: i64 = 23368;
const M421_CHAINS: std::ops::RangeInclusive<i64> = 23360..=23368;
const MIMIR: &str = "BetaE1_Mimir";
const EMITTER: &str = "BetaE1_HologramEmitter";
const DRONE_TAG: &str = "BetaE1_DisabledDrone";
const INFIRMARY: &str = "Beta_Site_Evo_1.Infirmary";
const OFFER_DIALOG: i64 = 1987;

/// The mission 421 chains that `resolved` matched.
fn m421_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M421_CHAINS.contains(id))
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

/// An `enter_region` context on `region_key`. `None` leaves the archetype unset.
fn region(region_key: &str, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
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

/// Mission 481 and 420 done, 421 not started: the state that opens the offer.
fn set_offer_open(ctx: &mut ExecutionContext) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "completed");
    with_mission(ctx, 421, "not_active");
}

/// Mission 420 done and 421 active at `step`.
fn set_step(ctx: &mut ExecutionContext, step: i32) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "completed");
    with_mission(ctx, 421, "active");
    with_step(ctx, 421, step, "active");
}

/// Mission 421 done (and 420 done).
fn set_finished(ctx: &mut ExecutionContext) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "completed");
    with_mission(ctx, 421, "completed");
}

/// Revert: drop 23360's `mission_completed 420` trigger row or its 420 gate.
/// Then the blurb bind is missing after 420 completes in the same session.
#[tokio::test]
async fn blurb_binds_on_login_and_on_420_completion() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut login = login_ctx(BETA, Some(ASGARD));
    set_offer_open(&mut login);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(2349, slot=2305)".to_string()],
    );
    assert_no_deferred_actions(&resolved, BLURB);
    assert_eq!(m421_matched(&resolved), BTreeSet::from([BLURB]));

    let mut done = completed(420, Some(ASGARD));
    set_offer_open(&mut done);
    let resolved = fire(&engine, TriggerType::MissionCompleted, &done);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(2349, slot=2305)".to_string()],
    );
    assert_eq!(m421_matched(&resolved), BTreeSet::from([BLURB]));

    let mut other_world = login_ctx("Beta_Site_Evo_2", Some(ASGARD));
    set_offer_open(&mut other_world);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &other_world);
    assert!(m421_matched(&resolved).is_empty());
}

/// Revert: drop 23360's archetype row. Then a non-Asgard login would bind the
/// blurb, and an active 420 or an active 421 would bind it too.
#[tokio::test]
async fn blurb_bind_ignores_other_races_and_states() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(summarized(&resolved, BLURB).is_empty(), "{archetype:?}");
    }
    let mut mid_quest = login_ctx(BETA, Some(ASGARD));
    with_mission(&mut mid_quest, 481, "completed");
    with_mission(&mut mid_quest, 420, "active");
    with_mission(&mut mid_quest, 421, "not_active");
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &mid_quest);
    assert!(summarized(&resolved, BLURB).is_empty());
    let mut started = login_ctx(BETA, Some(ASGARD));
    set_step(&mut started, 1460);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &started);
    assert!(summarized(&resolved, BLURB).is_empty());
}

/// Revert: drop 23361's `display_dialog 1987` or its 420 gate. The offer then
/// shows early, or never.
#[tokio::test]
async fn offer_shows_to_asgard_once_420_is_done() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_offer_open(&mut ctx);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, OFFER),
        vec!["display_dialog(1987)".to_string()],
    );
    assert_no_deferred_actions(&resolved, OFFER);
    assert_eq!(m421_matched(&resolved), BTreeSet::from([OFFER]));
    for archetype in non_asgard() {
        let mut ctx = click(MIMIR, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, OFFER).is_empty(), "{archetype:?}");
    }
    let mut not_yet = click(MIMIR, Some(ASGARD));
    with_mission(&mut not_yet, 481, "completed");
    with_mission(&mut not_yet, 420, "active");
    with_mission(&mut not_yet, 421, "not_active");
    let resolved = fire(&engine, TriggerType::InteractTag, &not_yet);
    assert!(summarized(&resolved, OFFER).is_empty());
}

/// Revert: drop 23362's `accept_mission 421`, its `remove_dialog_set 2349`, or
/// move the accept off the offer 1987.
#[tokio::test]
async fn accept_starts_421_and_drops_the_blurb() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = choice(OFFER_DIALOG, Some(ASGARD));
    set_offer_open(&mut ctx);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_eq!(
        summarized(&resolved, ACCEPT),
        vec![
            "accept_mission(421)".to_string(),
            "remove_dialog_set(2349, slot=2305)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, ACCEPT);
    assert_eq!(m421_matched(&resolved), BTreeSet::from([ACCEPT]));
    for archetype in non_asgard() {
        let mut ctx = choice(OFFER_DIALOG, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
        assert!(summarized(&resolved, ACCEPT).is_empty(), "{archetype:?}");
    }
    let mut wrong_dialog = choice(1953, Some(ASGARD));
    set_offer_open(&mut wrong_dialog);
    let resolved = fire(&engine, TriggerType::DialogChoice, &wrong_dialog);
    assert!(m421_matched(&resolved).is_empty());
    let mut active = choice(OFFER_DIALOG, Some(ASGARD));
    set_step(&mut active, 1460);
    let resolved = fire(&engine, TriggerType::DialogChoice, &active);
    assert!(m421_matched(&resolved).is_empty());
}

/// Revert: drop 23363's region trigger or its S1460 gate, or drop the
/// 23364 fallback. Then a player already inside the infirmary never advances.
#[tokio::test]
async fn region_and_fallback_both_reach_1461() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut inside = region_ctx(INFIRMARY, ASGARD);
    set_step(&mut inside, 1460);
    let resolved = fire(&engine, TriggerType::RegionEnter, &inside);
    assert_eq!(
        summarized(&resolved, REGION),
        vec!["advance_step(421, 1461)".to_string()],
    );
    assert_no_deferred_actions(&resolved, REGION);
    assert_eq!(m421_matched(&resolved), BTreeSet::from([REGION]));

    let mut clicked = click(EMITTER, Some(ASGARD));
    set_step(&mut clicked, 1460);
    let resolved = fire(&engine, TriggerType::InteractTag, &clicked);
    assert_eq!(
        summarized(&resolved, FALLBACK),
        vec!["advance_step(421, 1461)".to_string()],
    );
    assert_eq!(m421_matched(&resolved), BTreeSet::from([FALLBACK]));

    let mut past = region_ctx(INFIRMARY, ASGARD);
    set_step(&mut past, 1461);
    let resolved = fire(&engine, TriggerType::RegionEnter, &past);
    assert!(m421_matched(&resolved).is_empty());
    let mut elsewhere = region_ctx("Beta_Site_Evo_1.Hub", ASGARD);
    set_step(&mut elsewhere, 1460);
    let resolved = fire(&engine, TriggerType::RegionEnter, &elsewhere);
    assert!(m421_matched(&resolved).is_empty());
    for archetype in non_asgard() {
        let mut ctx = region(INFIRMARY, archetype);
        set_step(&mut ctx, 1460);
        let resolved = fire(&engine, TriggerType::RegionEnter, &ctx);
        assert!(m421_matched(&resolved).is_empty(), "{archetype:?}");
    }
}

/// Revert: drop 23365's gate, so the emitter shows twice, or make 23364 also
/// match S1461. The emitter chains must be disjoint by step status.
#[tokio::test]
async fn emitter_states_are_disjoint() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut s1460 = click(EMITTER, Some(ASGARD));
    set_step(&mut s1460, 1460);
    let resolved = fire(&engine, TriggerType::InteractTag, &s1460);
    assert_eq!(chains_matched(&resolved), BTreeSet::from([FALLBACK]));

    let mut s1461 = click(EMITTER, Some(ASGARD));
    set_step(&mut s1461, 1461);
    let resolved = fire(&engine, TriggerType::InteractTag, &s1461);
    assert_eq!(
        summarized(&resolved, EMITTER_SHOW),
        vec![
            "display_dialog(3722)".to_string(),
            "advance_step(421, 1462)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, EMITTER_SHOW);
    assert_eq!(chains_matched(&resolved), BTreeSet::from([EMITTER_SHOW]));

    for step in [1462, 1464] {
        let mut ctx = click(EMITTER, Some(ASGARD));
        set_step(&mut ctx, step);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(chains_matched(&resolved).is_empty(), "step {step}");
    }
}

/// Revert: drop 23366's `advance_step 1464` or its turn-in bind, or drop
/// 23367's bind (a relog at S1464 then loses the turn-in).
#[tokio::test]
async fn drone_examine_binds_the_turn_in() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut examine = click(DRONE_TAG, Some(ASGARD));
    set_step(&mut examine, 1462);
    let resolved = fire(&engine, TriggerType::InteractTag, &examine);
    assert_eq!(
        summarized(&resolved, DRONE),
        vec![
            "advance_step(421, 1464)".to_string(),
            "add_dialog_set(4585, slot=2305)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, DRONE);
    assert_eq!(m421_matched(&resolved), BTreeSet::from([DRONE]));

    let mut too_early = click(DRONE_TAG, Some(ASGARD));
    set_step(&mut too_early, 1461);
    let resolved = fire(&engine, TriggerType::InteractTag, &too_early);
    assert!(m421_matched(&resolved).is_empty());
    for archetype in non_asgard() {
        let mut ctx = click(DRONE_TAG, archetype);
        set_step(&mut ctx, 1462);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, DRONE).is_empty(), "{archetype:?}");
    }

    let mut relog = login_ctx(BETA, Some(ASGARD));
    set_step(&mut relog, 1464);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &relog);
    assert_eq!(
        summarized(&resolved, RESTORE),
        vec!["add_dialog_set(4585, slot=2305)".to_string()],
    );
    assert_eq!(m421_matched(&resolved), BTreeSet::from([RESTORE]));
    let mut before = login_ctx(BETA, Some(ASGARD));
    set_step(&mut before, 1462);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &before);
    assert!(m421_matched(&resolved).is_empty());
}

/// Revert: drop 23368's `complete_mission 421` or its `remove_dialog_set
/// 4585`. The turn-in then leaves the mission open or the bind on the NPC.
#[tokio::test]
async fn mimir_turn_in_completes_421() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_step(&mut ctx, 1464);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, TURN_IN),
        vec![
            "display_dialog(3901)".to_string(),
            "remove_dialog_set(4585, slot=2305)".to_string(),
            "complete_mission(421)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, TURN_IN);
    assert_eq!(m421_matched(&resolved), BTreeSet::from([TURN_IN]));
    let mut early = click(MIMIR, Some(ASGARD));
    set_step(&mut early, 1462);
    let resolved = fire(&engine, TriggerType::InteractTag, &early);
    assert!(m421_matched(&resolved).is_empty());
    for archetype in non_asgard() {
        let mut ctx = click(MIMIR, archetype);
        set_step(&mut ctx, 1464);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, TURN_IN).is_empty(), "{archetype:?}");
    }
}

/// Revert: drop the `Beta_Site_Evo_1` event key from 23367. An Asgard who logs
/// into another world at S1464 then gets the turn-in bind there.
#[tokio::test]
async fn relog_restore_ignores_other_worlds() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut relog = login_ctx("Castle_Cellblock", Some(ASGARD));
    set_step(&mut relog, 1464);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &relog);
    assert!(m421_matched(&resolved).is_empty());
}

/// Revert: any 421 chain that loses its archetype `eq 5` gate matches a
/// non-Asgard race on the same event. Every positive 421 event is replayed for
/// each non-Asgard race and must match no 421 chain.
#[tokio::test]
async fn wrong_race_fires_no_421_chain() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut login = login_ctx(BETA, archetype);
        set_offer_open(&mut login);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
        assert!(m421_matched(&resolved).is_empty(), "login {archetype:?}");

        let mut done = completed(420, archetype);
        set_offer_open(&mut done);
        let resolved = fire(&engine, TriggerType::MissionCompleted, &done);
        assert!(m421_matched(&resolved).is_empty(), "420 {archetype:?}");

        let mut offer = click(MIMIR, archetype);
        set_offer_open(&mut offer);
        let resolved = fire(&engine, TriggerType::InteractTag, &offer);
        assert!(m421_matched(&resolved).is_empty(), "offer {archetype:?}");

        let mut accept = choice(OFFER_DIALOG, archetype);
        set_offer_open(&mut accept);
        let resolved = fire(&engine, TriggerType::DialogChoice, &accept);
        assert!(m421_matched(&resolved).is_empty(), "accept {archetype:?}");

        let mut entered = region(INFIRMARY, archetype);
        set_step(&mut entered, 1460);
        let resolved = fire(&engine, TriggerType::RegionEnter, &entered);
        assert!(m421_matched(&resolved).is_empty(), "region {archetype:?}");

        let mut emitter = click(EMITTER, archetype);
        set_step(&mut emitter, 1461);
        let resolved = fire(&engine, TriggerType::InteractTag, &emitter);
        assert!(m421_matched(&resolved).is_empty(), "emitter {archetype:?}");

        let mut drone = click(DRONE_TAG, archetype);
        set_step(&mut drone, 1462);
        let resolved = fire(&engine, TriggerType::InteractTag, &drone);
        assert!(m421_matched(&resolved).is_empty(), "drone {archetype:?}");

        let mut relog = login_ctx(BETA, archetype);
        set_step(&mut relog, 1464);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &relog);
        assert!(m421_matched(&resolved).is_empty(), "relog {archetype:?}");

        let mut turn_in = click(MIMIR, archetype);
        set_step(&mut turn_in, 1464);
        let resolved = fire(&engine, TriggerType::InteractTag, &turn_in);
        assert!(m421_matched(&resolved).is_empty(), "turn-in {archetype:?}");
    }
}

/// Revert: add a sibling on the Mimir tag or the blurb dsm that fires in the
/// same state. The whole seed must match exactly one 421 chain per Mimir state.
#[tokio::test]
async fn mimir_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut offer = click(MIMIR, Some(ASGARD));
    set_offer_open(&mut offer);
    let resolved = fire(&engine, TriggerType::InteractTag, &offer);
    assert_eq!(chains_matched(&resolved), BTreeSet::from([OFFER]));

    let mut s1460 = click(MIMIR, Some(ASGARD));
    set_step(&mut s1460, 1460);
    let resolved = fire(&engine, TriggerType::InteractTag, &s1460);
    assert!(chains_matched(&resolved).is_empty());

    let mut turn_in = click(MIMIR, Some(ASGARD));
    set_step(&mut turn_in, 1464);
    let resolved = fire(&engine, TriggerType::InteractTag, &turn_in);
    assert_eq!(chains_matched(&resolved), BTreeSet::from([TURN_IN]));

    let mut finished = click(MIMIR, Some(ASGARD));
    set_finished(&mut finished);
    let resolved = fire(&engine, TriggerType::InteractTag, &finished);
    assert!(chains_matched(&resolved).is_empty());
}
