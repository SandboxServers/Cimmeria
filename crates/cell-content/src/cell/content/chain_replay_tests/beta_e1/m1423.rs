//! Beta Site E1 1423 receiver and Carter (UB-10): a Human who gates into Beta
//! with 1423 accepted and S4223 active moves to S4224 and gets Carter's talk
//! dialog set (chain 23020). A relog while reporting re-binds Carter (23021),
//! and clicking Carter shows the report dialog 4643 and completes 1423 (23022).
//! Seeded in `db/resources/Content/Seed/beta_e1_m1423_chains.sql`. Other races
//! and states must match none of the three.

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, engine_for, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    BETA, HUMANS, NON_HUMANS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const RECEIVER_CHAIN: i64 = 23020;
const RESTORE_CHAIN: i64 = 23021;
const CARTER_CHAIN: i64 = 23022;

const MISSION: i32 = 1423;
const ARRIVAL_STEP: i32 = 4223;
const TALK_STEP: i32 = 4224;
const CARTER_TAG: &str = "BetaE1_Carter";

/// The 1423 chains that `resolved` matched, for the exclusivity checks.
fn m1423_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    let block = [RECEIVER_CHAIN, RESTORE_CHAIN, CARTER_CHAIN];
    chains_matched(resolved)
        .into_iter()
        .filter(|id| block.contains(id))
        .collect()
}

/// A `player_loaded` into Beta with 1423 `active` and `step` at `status`.
fn login_state(archetype: Option<i64>, step: i32, status: &str) -> ExecutionContext {
    let mut ctx = login_ctx(BETA, archetype);
    with_mission(&mut ctx, MISSION, "active");
    with_step(&mut ctx, MISSION, step, status);
    ctx
}

/// An `interact_tag` click on Carter with 1423 `active` and `step` at `status`.
fn tap_state(archetype: Option<i64>, step: i32, status: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(CARTER_TAG));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    with_mission(&mut ctx, MISSION, "active");
    with_step(&mut ctx, MISSION, step, status);
    ctx
}

/// Revert: delete chain 23020's rows, drop one of its archetype rows, drop the
/// mission gate, or change the advance target away from S4224. Every Human
/// with S4223 active must then resolve exactly the advance and Carter's bind.
#[tokio::test]
async fn receiver_human_arrival_with_s4223_active_moves_to_carter() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23020).await;
    for archetype in HUMANS {
        let resolved = fire(
            &engine,
            TriggerType::PlayerLoaded,
            &login_state(Some(archetype), ARRIVAL_STEP, "active"),
        );
        assert_eq!(
            summarized(&resolved, RECEIVER_CHAIN),
            vec![
                "advance_step(1423, 4224)".to_string(),
                "add_dialog_set(5433, slot=2300)".to_string(),
            ],
            "Human archetype {archetype} must move 1423 to S4224 and bind Carter",
        );
        assert_no_deferred_actions(&resolved, RECEIVER_CHAIN);
    }

    // Whole seed: the arrival matches the receiver and nothing else in the block.
    let whole = whole_seed(&pool).await;
    let resolved = fire(
        &whole,
        TriggerType::PlayerLoaded,
        &login_state(Some(1), ARRIVAL_STEP, "active"),
    );
    assert_eq!(
        m1423_matched(&resolved),
        BTreeSet::from([RECEIVER_CHAIN]),
        "a Human arrival with S4223 active must match only chain 23020",
    );
}

/// Revert: widen the receiver's archetype gate to cover 5-8 or the unset
/// archetype, drop the step gate, or drop the mission gate. A non-Union race,
/// an unstamped player, a completed S4223 or a 1423 that is not active must
/// each match nothing.
#[tokio::test]
async fn receiver_ignores_other_races_and_states() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23020).await;
    let mut cases: Vec<ExecutionContext> = NON_HUMANS
        .iter()
        .map(|&a| login_state(Some(a), ARRIVAL_STEP, "active"))
        .collect();
    cases.push(login_state(Some(0), ARRIVAL_STEP, "active"));
    cases.push(login_state(None, ARRIVAL_STEP, "active"));
    cases.push(login_state(Some(1), ARRIVAL_STEP, "completed"));
    let mut not_accepted = login_state(Some(1), ARRIVAL_STEP, "active");
    with_mission(&mut not_accepted, MISSION, "not_active");
    cases.push(not_accepted);
    for ctx in cases {
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(
            summarized(&resolved, RECEIVER_CHAIN).is_empty(),
            "23020 must not match this race or state, got {:?}",
            summarized(&resolved, RECEIVER_CHAIN),
        );
    }
}

/// Revert: change 23021's step gate to S4223, drop its archetype gate, or
/// delete its rows. A Human relogging on S4224 must re-bind Carter, and only
/// that; on S4223 the receiver owns the login, so the restore must not fire.
#[tokio::test]
async fn restore_rebinds_carter_while_reporting() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23021).await;
    for archetype in HUMANS {
        let resolved = fire(
            &engine,
            TriggerType::PlayerLoaded,
            &login_state(Some(archetype), TALK_STEP, "active"),
        );
        assert_eq!(
            summarized(&resolved, RESTORE_CHAIN),
            vec!["add_dialog_set(5433, slot=2300)".to_string()],
            "Human archetype {archetype} reporting on S4224 must re-bind Carter",
        );
        assert_no_deferred_actions(&resolved, RESTORE_CHAIN);
    }
    for archetype in NON_HUMANS.iter().copied().map(Some).chain([None]) {
        let resolved = fire(
            &engine,
            TriggerType::PlayerLoaded,
            &login_state(archetype, TALK_STEP, "active"),
        );
        assert!(
            summarized(&resolved, RESTORE_CHAIN).is_empty(),
            "archetype {archetype:?} must not re-bind Carter",
        );
    }
    let resolved = fire(
        &engine,
        TriggerType::PlayerLoaded,
        &login_state(Some(1), ARRIVAL_STEP, "active"),
    );
    assert!(
        summarized(&resolved, RESTORE_CHAIN).is_empty(),
        "a Human still on S4223 must not get the restore bind",
    );

    // Whole seed: the reporting relog matches the restore and no other 1423 chain.
    let whole = whole_seed(&pool).await;
    let resolved = fire(
        &whole,
        TriggerType::PlayerLoaded,
        &login_state(Some(1), TALK_STEP, "active"),
    );
    assert_eq!(
        m1423_matched(&resolved),
        BTreeSet::from([RESTORE_CHAIN]),
        "a Human relog on S4224 must match only chain 23021",
    );
}

/// Revert: drop 23022's step gate, change its display dialog from 4643, drop
/// the unbind or the completion, or drop an archetype row. Carter's click for
/// a Human on S4224 must show 4643, unbind, then complete 1423, in that order.
#[tokio::test]
async fn carter_completes_1423() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23022).await;
    for archetype in HUMANS {
        let resolved = fire(
            &engine,
            TriggerType::InteractTag,
            &tap_state(Some(archetype), TALK_STEP, "active"),
        );
        assert_eq!(
            summarized(&resolved, CARTER_CHAIN),
            vec![
                "display_dialog(4643)".to_string(),
                "remove_dialog_set(5433, slot=2300)".to_string(),
                "complete_mission(1423)".to_string(),
            ],
            "Human archetype {archetype} reporting to Carter must complete 1423",
        );
        assert_no_deferred_actions(&resolved, CARTER_CHAIN);
    }
    for archetype in NON_HUMANS.iter().copied().map(Some).chain([None]) {
        let resolved = fire(
            &engine,
            TriggerType::InteractTag,
            &tap_state(archetype, TALK_STEP, "active"),
        );
        assert!(
            summarized(&resolved, CARTER_CHAIN).is_empty(),
            "archetype {archetype:?} must not complete 1423 at Carter",
        );
    }
    let resolved = fire(
        &engine,
        TriggerType::InteractTag,
        &tap_state(Some(1), ARRIVAL_STEP, "active"),
    );
    assert!(
        summarized(&resolved, CARTER_CHAIN).is_empty(),
        "a Human still on S4223 must not complete 1423 at Carter",
    );
}

/// Revert: add a second chain on the `BetaE1_Carter` tag that matches a Human
/// on S4224, or drop 23022's step gate so it also fires on S4223. The whole
/// seed must match exactly chain 23022 for the report click, and no 1423
/// chain for the click before the report.
#[tokio::test]
async fn carter_click_matches_one_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let resolved = fire(
        &whole,
        TriggerType::InteractTag,
        &tap_state(Some(1), TALK_STEP, "active"),
    );
    assert_eq!(
        chains_matched(&resolved),
        BTreeSet::from([CARTER_CHAIN]),
        "the report click must match exactly chain 23022",
    );
    let resolved = fire(
        &whole,
        TriggerType::InteractTag,
        &tap_state(Some(1), ARRIVAL_STEP, "active"),
    );
    assert!(
        m1423_matched(&resolved).is_empty(),
        "a click on S4223 must match no 1423 chain, matched {:?}",
        m1423_matched(&resolved),
    );
}
