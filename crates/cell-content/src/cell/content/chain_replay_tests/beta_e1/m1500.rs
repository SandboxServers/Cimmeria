//! Beta Site E1 mission 1500 receiver and Carter (UB-20): a Free Jaffa who
//! arrives from Omega with S4369 active is moved to S4370 and gets Carter's
//! talk dialog bound (chain 23220). A relog re-binds it (23221) and clicking
//! Carter shows dialog 5026, unbinds it and completes 1500 (23222). Seeded in
//! `db/resources/Content/Seed/beta_e1_m1500_chains.sql`. The gate is archetype
//! 7 (Free Jaffa) only: Jaffa (8) is Praxis and every other archetype is wrong.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, engine_for, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, ASGARD, BETA, FREE_JAFFA, HUMANS, JAFFA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const RECEIVER: i64 = 23220;
const RESTORE: i64 = 23221;
const CARTER: i64 = 23222;
/// The mission 1500 block of the seed (`beta_e1_m1500_chains.sql`).
const M1500_CHAINS: RangeInclusive<i64> = 23220..=23239;

/// The 1500 chains that `resolved` matched, for the exclusivity checks.
fn m1500_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M1500_CHAINS.contains(id))
        .collect()
}

/// Every archetype a Free Jaffa gate must refuse: unset, 0-6 and 8 (Jaffa).
fn not_free_jaffa() -> Vec<Option<i64>> {
    let mut archetypes = vec![None];
    archetypes.extend(
        ALL_ARCHETYPES
            .iter()
            .filter(|a| **a != FREE_JAFFA)
            .map(|a| Some(*a)),
    );
    archetypes
}

/// Stage mission 1500 as active at `step` with `status`, the way
/// `populate_mission_context` does.
fn stage(ctx: &mut ExecutionContext, step: i32, status: &str) {
    with_mission(ctx, 1500, "active");
    with_step(ctx, 1500, step, status);
}

/// A `player_loaded` into `world` by `archetype` with 1500 staged at `step`.
fn login_at(world: &str, archetype: Option<i64>, step: i32, status: &str) -> ExecutionContext {
    let mut ctx = login_ctx(world, archetype);
    stage(&mut ctx, step, status);
    ctx
}

/// An `interact_tag` on `BetaE1_Carter` by `archetype` with 1500 staged at `step`.
/// `archetype` is optional so the unset (Any) player can be tested.
fn carter_at(archetype: Option<i64>, step: i32, status: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!("BetaE1_Carter"));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    stage(&mut ctx, step, status);
    ctx
}

/// Revert: delete chain 23220's rows (`engine_for` then panics), drop its
/// `step_status` row, or change its archetype away from 7. The whole seed
/// must then resolve something other than exactly 23220 for this arrival.
#[tokio::test]
async fn receiver_free_jaffa_arrival_with_s4369_active_moves_to_carter() {
    let pool = require_db_or_skip!();
    let ctx = login_at(BETA, Some(FREE_JAFFA), 4369, "active");
    let engine = engine_for(&pool, RECEIVER as i32).await;
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_eq!(
        summarized(&resolved, RECEIVER),
        vec![
            "advance_step(1500, 4370)".to_string(),
            "add_dialog_set(5871, slot=2300)".to_string(),
        ],
        "a Free Jaffa at S4369 must move to S4370 and bind Carter",
    );
    assert_no_deferred_actions(&resolved, RECEIVER);

    let whole = whole_seed(&pool).await;
    let matched = m1500_matched(&fire(&whole, TriggerType::PlayerLoaded, &ctx));
    assert_eq!(
        matched,
        BTreeSet::from([RECEIVER]),
        "only the receiver may fire for a Free Jaffa at S4369",
    );
}

/// Revert: widen chain 23220's archetype gate (to Human, Asgard or Jaffa), or
/// drop its `step_status` row so a completed or unstaged S4369 also fires.
/// Every archetype except 7, every other world and every S4369 state that is
/// not `active` must match nothing in 23220-23239.
#[tokio::test]
async fn receiver_ignores_other_races_and_states() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in not_free_jaffa() {
        let resolved = fire(
            &whole,
            TriggerType::PlayerLoaded,
            &login_at(BETA, archetype, 4369, "active"),
        );
        assert!(
            m1500_matched(&resolved).is_empty(),
            "archetype {archetype:?} at S4369 must match no 1500 chain, matched {:?}",
            m1500_matched(&resolved),
        );
    }
    for status in ["completed", "not_active"] {
        let resolved = fire(
            &whole,
            TriggerType::PlayerLoaded,
            &login_at(BETA, Some(FREE_JAFFA), 4369, status),
        );
        assert!(
            m1500_matched(&resolved).is_empty(),
            "a Free Jaffa with S4369 {status} must match no 1500 chain",
        );
    }
    let resolved = fire(
        &whole,
        TriggerType::PlayerLoaded,
        &login_at("Omega_Site", Some(FREE_JAFFA), 4369, "active"),
    );
    assert!(
        m1500_matched(&resolved).is_empty(),
        "a Free Jaffa login on Omega_Site must match no Beta 1500 chain",
    );
}

/// Revert: delete chain 23221's rows, or change its step to S4369 (the
/// restore would then bind Carter before the receiver has moved the player).
/// The restore binds Carter only while S4370 is active, for a Free Jaffa.
#[tokio::test]
async fn restore_rebinds_carter_only_while_s4370_is_active() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, RESTORE as i32).await;
    let ctx = login_at(BETA, Some(FREE_JAFFA), 4370, "active");
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_eq!(
        summarized(&resolved, RESTORE),
        vec!["add_dialog_set(5871, slot=2300)".to_string()],
        "a Free Jaffa at S4370 must have Carter bound on relog",
    );
    assert_no_deferred_actions(&resolved, RESTORE);

    let whole = whole_seed(&pool).await;
    for archetype in not_free_jaffa() {
        let resolved = fire(
            &whole,
            TriggerType::PlayerLoaded,
            &login_at(BETA, archetype, 4370, "active"),
        );
        assert!(
            m1500_matched(&resolved).is_empty(),
            "archetype {archetype:?} at S4370 must match no 1500 chain",
        );
    }
    let at_s4369 = fire(
        &whole,
        TriggerType::PlayerLoaded,
        &login_at(BETA, Some(FREE_JAFFA), 4369, "active"),
    );
    assert_eq!(
        m1500_matched(&at_s4369),
        BTreeSet::from([RECEIVER]),
        "at S4369 only the receiver fires, never the restore",
    );
    for status in ["completed", "not_active"] {
        let resolved = fire(
            &whole,
            TriggerType::PlayerLoaded,
            &login_at(BETA, Some(FREE_JAFFA), 4370, status),
        );
        assert!(
            m1500_matched(&resolved).is_empty(),
            "a Free Jaffa with S4370 {status} must not re-bind Carter",
        );
    }
}

/// Revert: delete chain 23222's rows, drop one of its three actions, or
/// change its archetype away from 7. Clicking Carter as a Free Jaffa at S4370
/// must show dialog 5026, unbind it and complete 1500, in that order.
#[tokio::test]
async fn carter_welcome_for_free_jaffa_at_s4370() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, CARTER as i32).await;
    let resolved = fire(
        &engine,
        TriggerType::InteractTag,
        &carter_at(Some(FREE_JAFFA), 4370, "active"),
    );
    assert_eq!(
        summarized(&resolved, CARTER),
        vec![
            "display_dialog(5026)".to_string(),
            "remove_dialog_set(5871, slot=2300)".to_string(),
            "complete_mission(1500)".to_string(),
        ],
        "Carter's welcome must show 5026, unbind Carter and complete 1500",
    );
    assert_no_deferred_actions(&resolved, CARTER);
}

/// Revert: drop chain 23222's `step_status` row or its archetype gate, so a
/// Human, an Asgard, a Jaffa or an unset player (or a Free Jaffa at another
/// step) gets the welcome. Each of those must match no 1500 chain.
#[tokio::test]
async fn carter_welcome_gates_on_race_and_step() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in not_free_jaffa() {
        let resolved = fire(
            &whole,
            TriggerType::InteractTag,
            &carter_at(archetype, 4370, "active"),
        );
        assert!(
            m1500_matched(&resolved).is_empty(),
            "archetype {archetype:?} clicking Carter at S4370 must match no 1500 chain",
        );
    }
    for (step, status) in [(4370, "completed"), (4370, "not_active"), (4369, "active")] {
        let resolved = fire(
            &whole,
            TriggerType::InteractTag,
            &carter_at(Some(FREE_JAFFA), step, status),
        );
        assert!(
            m1500_matched(&resolved).is_empty(),
            "a Free Jaffa clicking Carter at S{step} {status} must match no 1500 chain",
        );
    }
}

/// Revert: widen one of the three Carter chains' gates so two of them match
/// the same click, or drop the Free Jaffa gate so a Union Human or Asgard
/// gets the welcome. The Free Jaffa click alone matches exactly 23222, and the
/// same state for a Human, an Asgard or a Jaffa matches no 1500 chain.
#[tokio::test]
async fn carter_click_is_race_exclusive() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let free_jaffa = fire(
        &whole,
        TriggerType::InteractTag,
        &carter_at(Some(FREE_JAFFA), 4370, "active"),
    );
    assert_eq!(
        m1500_matched(&free_jaffa),
        BTreeSet::from([CARTER]),
        "the Free Jaffa click at S4370 must match exactly 23222",
    );
    let others = HUMANS
        .iter()
        .map(|a| Some(*a))
        .chain([Some(ASGARD), Some(JAFFA)]);
    for archetype in others {
        let resolved = fire(
            &whole,
            TriggerType::InteractTag,
            &carter_at(archetype, 4370, "active"),
        );
        assert!(
            m1500_matched(&resolved).is_empty(),
            "archetype {archetype:?} clicking Carter at S4370 must match no 1500 chain",
        );
    }
}
