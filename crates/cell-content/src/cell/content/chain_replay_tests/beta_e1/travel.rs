//! Beta Site E1 travel way back (UB-01b): a Union player who gates to Beta
//! learns the Omega Site address (stargate 5) on `player_loaded`, so the
//! DHD there can dial home. Chains 23001 (Human), 23002 (Asgard) and 23003
//! (Free Jaffa), seeded in `db/resources/Content/Seed/beta_e1_travel_chains.sql`.
//! Goa'uld (6) and Jaffa (8) are Praxis and must learn nothing.

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, engine_for, fire, login_ctx, summarized, whole_seed, ASGARD, BETA, FREE_JAFFA,
    GOAULD, HUMANS, JAFFA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::triggers::TriggerType;

const HUMAN_CHAIN: i64 = 23001;
const ASGARD_CHAIN: i64 = 23002;
const FREE_JAFFA_CHAIN: i64 = 23003;

/// The travel block's chains that `resolved` matched, for the exclusivity checks.
fn travel_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    let travel = [HUMAN_CHAIN, ASGARD_CHAIN, FREE_JAFFA_CHAIN];
    chains_matched(resolved)
        .into_iter()
        .filter(|id| travel.contains(id))
        .collect()
}

/// Revert: delete chain 23001's rows, or drop one of its archetype rows.
/// Any Human archetype must then fail to resolve the grant.
#[tokio::test]
async fn human_arrival_learns_omega() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23001).await;
    for archetype in HUMANS {
        let resolved = fire(
            &engine,
            TriggerType::PlayerLoaded,
            &login_ctx(BETA, Some(archetype)),
        );
        assert_eq!(
            summarized(&resolved, HUMAN_CHAIN),
            vec!["grant_stargate_address(5)".to_string()],
            "Human archetype {archetype} must learn Omega Site on Beta",
        );
        assert_no_deferred_actions(&resolved, HUMAN_CHAIN);
    }
}

/// Revert: delete chain 23002's rows (the Asgard grant then has no chain
/// and `engine_for` panics), or change its archetype away from 5.
#[tokio::test]
async fn asgard_arrival_learns_omega() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23002).await;
    let resolved = fire(
        &engine,
        TriggerType::PlayerLoaded,
        &login_ctx(BETA, Some(ASGARD)),
    );
    assert_eq!(
        summarized(&resolved, ASGARD_CHAIN),
        vec!["grant_stargate_address(5)".to_string()],
        "Asgard must learn Omega Site on Beta",
    );
    assert_no_deferred_actions(&resolved, ASGARD_CHAIN);
}

/// Revert: delete chain 23003's rows, or change its archetype away from 7.
#[tokio::test]
async fn free_jaffa_arrival_learns_omega() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23003).await;
    let resolved = fire(
        &engine,
        TriggerType::PlayerLoaded,
        &login_ctx(BETA, Some(FREE_JAFFA)),
    );
    assert_eq!(
        summarized(&resolved, FREE_JAFFA_CHAIN),
        vec!["grant_stargate_address(5)".to_string()],
        "Free Jaffa must learn Omega Site on Beta",
    );
    assert_no_deferred_actions(&resolved, FREE_JAFFA_CHAIN);
}

/// Revert: widen any travel chain's archetype gate to cover Praxis (6 or 8),
/// or drop the gate. The whole seed must then match a travel chain for a
/// Goa'uld, a Jaffa, the Any archetype (0) or a missing one.
#[tokio::test]
async fn praxis_and_unknown_arrivals_learn_nothing() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let cases = [Some(0), Some(GOAULD), Some(JAFFA), None];
    for archetype in cases {
        let resolved = fire(
            &engine,
            TriggerType::PlayerLoaded,
            &login_ctx(BETA, archetype),
        );
        let matched = travel_matched(&resolved);
        assert!(
            matched.is_empty(),
            "archetype {archetype:?} must match no travel chain, matched {matched:?}",
        );
    }
}

/// Revert: a second race gate that overlaps another race (Human `lte 5`,
/// or Asgard `gte 5`), or a travel chain that drops its gate. Each Union
/// race must match exactly its own chain, never two.
#[tokio::test]
async fn each_union_race_matches_exactly_one_travel_chain() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let races = [
        (1, HUMAN_CHAIN),
        (2, HUMAN_CHAIN),
        (3, HUMAN_CHAIN),
        (4, HUMAN_CHAIN),
        (ASGARD, ASGARD_CHAIN),
        (FREE_JAFFA, FREE_JAFFA_CHAIN),
    ];
    for (archetype, own_chain) in races {
        let resolved = fire(
            &engine,
            TriggerType::PlayerLoaded,
            &login_ctx(BETA, Some(archetype)),
        );
        let matched = travel_matched(&resolved);
        assert_eq!(
            matched,
            BTreeSet::from([own_chain]),
            "archetype {archetype} must match exactly chain {own_chain}",
        );
    }
}

/// Revert: change a travel trigger's `event_key` away from `Beta_Site_Evo_1`,
/// or match every world. A login on another world (Omega Site, world 18)
/// must then grant nothing.
#[tokio::test]
async fn other_worlds_do_not_grant() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23001).await;
    let resolved = fire(
        &engine,
        TriggerType::PlayerLoaded,
        &login_ctx("Omega_Site", Some(1)),
    );
    assert!(
        summarized(&resolved, HUMAN_CHAIN).is_empty(),
        "a Human login on Omega_Site must not grant anything on Beta's chain",
    );
}
