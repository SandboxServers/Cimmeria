//! Beta Site E1 mission XP rewards (UB-50a): one `grant_xp` chain per Beta
//! mission on `mission_completed`, chains 23480-23496. Seeded in
//! `db/resources/Content/Seed/beta_e1_rewards_chains.sql`. The amounts are the
//! XP column of `reward-proposal.md` (U-D07), and every mission's
//! `reward_xp` stays 0 so the seed cannot pay twice (D-UB16).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, engine_for, fire, summarize, summarized, whole_seed, with_mission,
    ALL_ARCHETYPES, ASGARD, FREE_JAFFA, HUMANS, REWARD_CHAINS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

/// The race gate a reward chain carries (`archetype` rows in the seed).
#[derive(Clone, Copy)]
enum Race {
    Human,
    FreeJaffa,
    Asgard,
}

impl Race {
    /// The archetypes this race's gate admits.
    fn archetypes(self) -> &'static [i64] {
        match self {
            Race::Human => &HUMANS[..],
            Race::FreeJaffa => &[FREE_JAFFA][..],
            Race::Asgard => &[ASGARD][..],
        }
    }
}

/// Mission, reward chain, race and XP for the 17 Beta missions, in the
/// proposal's table order (reward-proposal.md § 3).
const REWARDS: [(i64, i64, Race, u64); 17] = [
    (1423, 23480, Race::Human, 300),
    (381, 23481, Race::Human, 300),
    (473, 23482, Race::Human, 300),
    (478, 23483, Race::Human, 400),
    (1425, 23484, Race::Human, 400),
    (1426, 23485, Race::Human, 400),
    (1427, 23486, Race::Human, 750),
    (1500, 23487, Race::FreeJaffa, 250),
    (1502, 23488, Race::FreeJaffa, 300),
    (1503, 23489, Race::FreeJaffa, 350),
    (481, 23490, Race::Asgard, 300),
    (420, 23491, Race::Asgard, 300),
    (421, 23492, Race::Asgard, 300),
    (1147, 23493, Race::Asgard, 300),
    (443, 23494, Race::Asgard, 525),
    (477, 23495, Race::Asgard, 300),
    (479, 23496, Race::Asgard, 350),
];

/// A `mission_completed` of `mission` by a player of `archetype` (`None`
/// leaves it unset), with `mission` already `completed`.
fn finished(mission: i64, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(mission));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    with_mission(&mut ctx, mission as i32, "completed");
    ctx
}

/// The rendered `grant_xp(..)` actions in `resolved`, from every chain.
fn xp_grants(resolved: &ResolvedActions) -> Vec<String> {
    resolved
        .actions
        .iter()
        .map(|(_, action)| summarize(action))
        .filter(|s| s.starts_with("grant_xp("))
        .collect()
}

/// Revert: change one amount in the seed (chain 23486 to 700, say) and this
/// fails on that chain.
#[tokio::test]
async fn each_reward_chain_grants_its_xp_once() {
    let pool = require_db_or_skip!();
    for (mission, chain, race, xp) in REWARDS {
        let engine = engine_for(&pool, chain as i32).await;
        for &archetype in race.archetypes() {
            let resolved = fire(
                &engine,
                TriggerType::MissionCompleted,
                &finished(mission, Some(archetype)),
            );
            assert_eq!(
                summarized(&resolved, chain),
                vec![format!("grant_xp({xp})")],
                "mission {mission} by archetype {archetype} must grant {xp} XP once"
            );
            assert_no_deferred_actions(&resolved, chain);
        }
    }
}

/// Revert: drop one archetype row from a reward chain's seed (or widen its
/// gate) and this fails for that chain's outside archetype.
#[tokio::test]
async fn reward_chains_reject_other_races_and_missions() {
    let pool = require_db_or_skip!();
    for (i, (mission, chain, race, _)) in REWARDS.into_iter().enumerate() {
        let engine = engine_for(&pool, chain as i32).await;
        let outside = ALL_ARCHETYPES
            .into_iter()
            .filter(|a| !race.archetypes().contains(a))
            .map(Some)
            .chain(std::iter::once(None));
        for archetype in outside {
            let resolved = fire(
                &engine,
                TriggerType::MissionCompleted,
                &finished(mission, archetype),
            );
            assert!(
                summarized(&resolved, chain).is_empty(),
                "mission {mission} by archetype {archetype:?} must not grant XP"
            );
        }
        // Another Beta mission's completion, by a player of the right race.
        let (other, _, _, _) = REWARDS[(i + 1) % REWARDS.len()];
        let resolved = fire(
            &engine,
            TriggerType::MissionCompleted,
            &finished(other, Some(race.archetypes()[0])),
        );
        assert!(
            summarized(&resolved, chain).is_empty(),
            "mission {other} must not fire the reward chain for {mission}"
        );
    }
}

/// Revert: duplicate one reward chain under id 23497 (same mission, same
/// XP) and this fails on the count of `grant_xp` actions.
#[tokio::test]
async fn whole_seed_pays_each_mission_exactly_once() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for (mission, chain, race, xp) in REWARDS {
        let resolved = fire(
            &whole,
            TriggerType::MissionCompleted,
            &finished(mission, Some(race.archetypes()[0])),
        );
        assert_eq!(
            xp_grants(&resolved),
            vec![format!("grant_xp({xp})")],
            "mission {mission} must pay {xp} XP exactly once across the whole seed"
        );
        let rewarded: BTreeSet<i64> = chains_matched(&resolved)
            .into_iter()
            .filter(|id| REWARD_CHAINS.contains(id))
            .collect();
        assert_eq!(rewarded, BTreeSet::from([chain]));
    }
}

/// Revert: add a `grant_xp` action to any other chain in the Beta block
/// (23040, say) and this fails with the extra id.
#[tokio::test]
async fn only_reward_chains_grant_xp_in_the_beta_block() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT chain_id FROM resources.content_actions \
         WHERE action_type = 'grant_xp' AND chain_id BETWEEN 23000 AND 23499 \
         ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("resources.content_actions must be readable");
    let want: Vec<i32> = (23480..=23496).collect();
    assert_eq!(ids, want);
}

/// Revert: set `reward_xp` to 300 on mission 1423 in a scratch copy of the
/// seed and this fails (the double-pay guard, D-UB16).
#[tokio::test]
async fn beta_mission_reward_columns_stay_zero() {
    let pool = require_db_or_skip!();
    let ids: Vec<i64> = REWARDS.iter().map(|r| r.0).collect();
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT mission_id::bigint, reward_xp::bigint, reward_naq::bigint \
         FROM resources.missions WHERE mission_id = ANY($1::bigint[]) ORDER BY mission_id",
    )
    .bind(&ids)
    .fetch_all(&pool)
    .await
    .expect("resources.missions must be readable");
    assert_eq!(rows.len(), 17, "all 17 Beta missions must exist");
    for (mission, xp, naq) in rows {
        assert_eq!(
            (xp, naq),
            (0, 0),
            "mission {mission} must keep reward_xp and reward_naq at 0"
        );
    }
}
