//! Beta Site E1 Incendiary (UB-34, mission 443): the Mimir blurb bind (23405),
//! the offer (23406), the Accept and the Carter bind (23407), the Carter relog
//! restore (23408), Carter's hand-off (23409), the Satterfield restore (23410)
//! and hand-off (23411), the radio step (23412 region, 23413 fallback), the
//! suppressant manager (23414), the fire control hub (23415), the turn-in relog
//! restore (23416) and the Mimir turn-in (23417). Seeded in
//! `db/resources/Content/Seed/beta_e1_m443_chains.sql`.
//!
//! Omega UO-16 is HELD (HELD-B16), so no live Asgard reaches Carter through
//! Beta yet. These tests replay archetype 5 (D-UB13).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, ASGARD, BETA, HUMANS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

mod exclusivity;
mod hand_offs;

const BLURB: i64 = 23405;
const OFFER: i64 = 23406;
const ACCEPT: i64 = 23407;
const CARTER_RESTORE: i64 = 23408;
const CARTER: i64 = 23409;
const SATTERFIELD_RESTORE: i64 = 23410;
const SATTERFIELD: i64 = 23411;
const REGION: i64 = 23412;
const RADIO: i64 = 23413;
const MANAGER: i64 = 23414;
const HUB: i64 = 23415;
const TURN_IN_RESTORE: i64 = 23416;
const TURN_IN: i64 = 23417;
const M443_CHAINS: std::ops::RangeInclusive<i64> = 23405..=23417;
const MIMIR: &str = "BetaE1_Mimir";
const CARTER_TAG: &str = "BetaE1_Carter";
const SATTERFIELD_TAG: &str = "BetaE1_Satterfield";
const MANAGER_TAG: &str = "BetaE1_FireSuppressantManager";
const HUB_TAG: &str = "BetaE1_FireControlHub";
const FIRE_SITE: &str = "Beta_Site_Evo_1.FireSite";
const OFFER_DIALOG: i64 = 1918;
const TURN_IN_DIALOG: i64 = 1917;
/// The active steps of 443, in order, with the `interact_tag` that advances
/// or completes each one (S1484 is advanced by the region and the manager).
const STEPS: [(i32, &str); 6] = [
    (1482, CARTER_TAG),
    (1483, SATTERFIELD_TAG),
    (1484, MANAGER_TAG),
    (1485, MANAGER_TAG),
    (1486, HUB_TAG),
    (3455, MIMIR),
];

/// Where mission 443 stands for a replay: the offer is open, a step is
/// active, or the mission is done. Mission 1147 is always completed (the
/// prerequisite), except where a test overrides it.
#[derive(Clone, Copy, Debug)]
enum Stage {
    Offer,
    Step(i32),
    Done,
}

fn m443_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M443_CHAINS.contains(id))
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

/// Set mission 1147 completed and 443 at `stage`.
fn set_stage(ctx: &mut ExecutionContext, stage: Stage) {
    with_mission(ctx, 1147, "completed");
    match stage {
        Stage::Offer => with_mission(ctx, 443, "not_active"),
        Stage::Step(step) => {
            with_mission(ctx, 443, "active");
            with_step(ctx, 443, step, "active");
        }
        Stage::Done => with_mission(ctx, 443, "completed"),
    }
}

/// Fire `trigger` with `ctx` at `stage` and assert exactly `want` 443 chains match.
fn expect(
    engine: &cimmeria_content_engine::chain::ChainEngine,
    stage: Stage,
    trigger: TriggerType,
    mut ctx: ExecutionContext,
    want: &[i64],
) {
    set_stage(&mut ctx, stage);
    let resolved = fire(engine, trigger, &ctx);
    let want: BTreeSet<i64> = want.iter().copied().collect();
    assert_eq!(m443_matched(&resolved), want, "{stage:?}");
}

/// Revert: drop 23405's `mission_completed 1147` trigger row. Then the blurb
/// is missing after 1147 completes in the same session.
#[tokio::test]
async fn blurb_binds_on_login_and_on_1147_completion() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut on_login = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut on_login, Stage::Offer);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &on_login);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(4581, slot=2305)".to_string()],
    );
    assert_no_deferred_actions(&resolved, BLURB);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([BLURB]));
    let mut on_completion = completed(1147, Some(ASGARD));
    set_stage(&mut on_completion, Stage::Offer);
    let resolved = fire(&engine, TriggerType::MissionCompleted, &on_completion);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(4581, slot=2305)".to_string()],
    );
    assert_eq!(m443_matched(&resolved), BTreeSet::from([BLURB]));
    let mut other_world = login_ctx("Castle", Some(ASGARD));
    set_stage(&mut other_world, Stage::Offer);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &other_world);
    assert!(m443_matched(&resolved).is_empty());
}

/// Revert: drop the `archetype eq 5` row from 23405, or its `443 not_active`
/// gate. Then a Human, an unstamped login, or a started 443 gets the blurb.
#[tokio::test]
async fn blurb_bind_ignores_other_races_and_states() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut on_login = login_ctx(BETA, archetype);
        set_stage(&mut on_login, Stage::Offer);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &on_login);
        assert!(m443_matched(&resolved).is_empty(), "{archetype:?}");
        let mut on_completion = completed(1147, archetype);
        set_stage(&mut on_completion, Stage::Offer);
        let resolved = fire(&engine, TriggerType::MissionCompleted, &on_completion);
        assert!(m443_matched(&resolved).is_empty(), "{archetype:?}");
    }
    let mut started = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut started, Stage::Step(1482));
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &started);
    assert!(summarized(&resolved, BLURB).is_empty());
    let mut done = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut done, Stage::Done);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &done);
    assert!(summarized(&resolved, BLURB).is_empty());
    let mut unfinished = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut unfinished, Stage::Offer);
    with_mission(&mut unfinished, 1147, "active");
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &unfinished);
    assert!(summarized(&resolved, BLURB).is_empty());
}

/// Revert: drop 23406's `1147 completed` gate or its `display_dialog`. Then
/// the offer shows before 1147 is done, or not at all.
#[tokio::test]
async fn offer_shows_to_asgard_once_1147_is_done() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_stage(&mut ctx, Stage::Offer);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, OFFER),
        vec!["display_dialog(1918)".to_string()],
    );
    assert_no_deferred_actions(&resolved, OFFER);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([OFFER]));
    for archetype in non_asgard() {
        expect(
            &engine,
            Stage::Offer,
            TriggerType::InteractTag,
            click(MIMIR, archetype),
            &[],
        );
    }
    let mut unfinished = click(MIMIR, Some(ASGARD));
    set_stage(&mut unfinished, Stage::Offer);
    with_mission(&mut unfinished, 1147, "active");
    let resolved = fire(&engine, TriggerType::InteractTag, &unfinished);
    assert!(m443_matched(&resolved).is_empty());
    expect(
        &engine,
        Stage::Step(1482),
        TriggerType::InteractTag,
        click(MIMIR, Some(ASGARD)),
        &[],
    );
}

/// Revert: drop 23407's `remove_dialog_set` or its Carter `add_dialog_set`.
/// Then the blurb stays bound on Mimir, or Carter is not bound at S1482.
#[tokio::test]
async fn accept_starts_443_and_binds_carter() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = choice(OFFER_DIALOG, Some(ASGARD));
    set_stage(&mut ctx, Stage::Offer);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_eq!(
        summarized(&resolved, ACCEPT),
        vec![
            "accept_mission(443)".to_string(),
            "remove_dialog_set(4581, slot=2305)".to_string(),
            "add_dialog_set(3728, slot=2300)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, ACCEPT);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([ACCEPT]));
    for archetype in non_asgard() {
        expect(
            &engine,
            Stage::Offer,
            TriggerType::DialogChoice,
            choice(OFFER_DIALOG, archetype),
            &[],
        );
    }
    expect(
        &engine,
        Stage::Offer,
        TriggerType::DialogChoice,
        choice(TURN_IN_DIALOG, Some(ASGARD)),
        &[],
    );
    expect(
        &engine,
        Stage::Step(1482),
        TriggerType::DialogChoice,
        choice(OFFER_DIALOG, Some(ASGARD)),
        &[],
    );
}
