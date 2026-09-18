//! Mission 703 "Payback" — chains 1271-1273 in
//! `db/resources/Content/Seed/castle_702_704_chains.sql` (packet CA06).
//!
//! RECONSTRUCTION, not a port: `Castle.py` never mentions 703. The
//! authoring decisions these guards pin:
//!
//! * Region entry advances 2403 → 2404, mirroring 702's chain 1261 on the
//!   same volume — both fire on one entry when the player holds both
//!   missions, which is the intended shape under D-CA05.
//! * Romney's death completes 703 and grants item 2135 ("Romney's NID
//!   Badge") as an EXPLICIT chain grant. There is no loot table and no
//!   `mission_reward_groups` row for 701-708, so this is the only
//!   mechanism available.
//! * Castle is an open world and region entry is client-hinted, so a
//!   player can reach Romney without ever crossing
//!   `Castle.InterrogationBlock`. Chain 1273 is the off-path death that
//!   advances and completes in one action list; without it the mission
//!   sticks on 2403 behind a corpse.
//! * 1272 and 1273 are mutually exclusive by construction. The engine
//!   snapshots the killer's mission context once in `fire_entity_death`
//!   and evaluates every chain against that one snapshot, so exactly one
//!   of the two can match a given kill — and the badge is granted once.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Compact label for an action so a drifted list fails on the sequence.
fn label(a: &Action) -> &'static str {
    match a {
        Action::AdvanceStep { .. } => "advance_step",
        Action::GrantItem { .. } => "add_item",
        Action::CompleteMission { .. } => "complete_mission",
        _ => "OTHER",
    }
}

/// Register the given seeded chains in one engine and resolve a synthetic
/// event against all of them. Returns `(chain_id, action)` pairs so a
/// caller can assert which chain claimed which action.
async fn resolve_chains(
    pool: &sqlx::PgPool,
    chain_ids: &[i32],
    trigger_type: TriggerType,
    params: &[(&str, serde_json::Value)],
) -> Vec<(i64, Action)> {
    let mut engine = ChainEngine::new();
    for &chain_id in chain_ids {
        let chain = load_single_chain_for_test(pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| {
                panic!(
                    "chain {chain_id} must exist in seeded content_chains — \
                     castle_702_704_chains.sql missing from db/database.sql?"
                )
            });
        engine.register_chain(chain);
    }

    let mut ctx = ExecutionContext::new();
    for (k, v) in params {
        ctx.set_param((*k).to_string(), v.clone());
    }
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx).actions
}

/// Same as [`resolve_chains`] for a single chain, dropping the chain id.
async fn resolve_chain(
    pool: &sqlx::PgPool,
    chain_id: i32,
    trigger_type: TriggerType,
    params: &[(&str, serde_json::Value)],
) -> Vec<Action> {
    resolve_chains(pool, &[chain_id], trigger_type, params)
        .await
        .into_iter()
        .map(|(_, a)| a)
        .collect()
}

/// The badge grant must be a single mission-container grant of item 2135.
fn assert_badge_grant(a: &Action, what: &str) {
    match a {
        Action::GrantItem {
            item_id,
            count,
            container_id,
        } => {
            assert_eq!(*item_id, 2135, "{what}: must grant Romney's NID Badge");
            assert_eq!(*count, 1, "{what}: exactly one badge");
            // `container: 0` is the "use the item's own container_sets[1]"
            // sentinel — `executor::inventory::grant` filters `c > 0`, so 0
            // falls through to the DB mapping (2, the mission container).
            // Mirrors chain 1003 in castle_cellblock_chains.sql.
            assert_eq!(
                *container_id,
                Some(0),
                "{what}: container must stay the 0 sentinel so the item's own \
                 container_sets mapping decides (mission container)",
            );
        }
        other => panic!("{what}: expected add_item, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1271 — enter Castle.InterrogationBlock on step 2403
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1271_region_entry_advances_to_the_kill_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1271,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_703_step_2403_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert_eq!(
        actions.len(),
        1,
        "chain 1271 must produce exactly one action; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::AdvanceStep {
                mission_id: 703,
                step_id: 2404
            }
        ),
        "chain 1271 must advance 703 to step 2404; got {:?}",
        actions[0],
    );
}

#[tokio::test]
async fn chain_1271_does_not_resolve_once_2404_is_active() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1271,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_703_step_2404_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1271 must be gated on step 2403; got {actions:?}",
    );
}

/// 702's chain 1261 shares this volume. A player who holds 702 but never
/// took 703 must not get 703's advance.
#[tokio::test]
async fn chain_1271_does_not_resolve_without_703() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1271,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_702_step_2402_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1271 must not fire for a player without 703; got {actions:?}",
    );
}

/// The seed comment on chain 1271 claims 702's chain 1261 and 703's 1271
/// both fire on a single Interrogation Block entry when the player holds
/// both missions, which is the intended shape under D-CA05 (703 is accepted
/// alongside 702). Every other test here registers one chain at a time and
/// so cannot see that; this one registers both and asserts they co-fire.
///
/// It is a real guard, not a restatement: the two chains share a region
/// trigger, and a future author who "de-duplicated" them onto one chain, or
/// who gated 1271 on 702's step by mistake, would leave a player holding
/// only 703 stuck on step 2403 forever.
#[tokio::test]
async fn chains_1261_and_1271_both_claim_one_interrogation_block_entry() {
    let pool = require_db_or_skip!();
    let actions = resolve_chains(
        &pool,
        &[1261, 1271],
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_702_step_2402_status", serde_json::json!("active")),
            ("mission_703_step_2403_status", serde_json::json!("active")),
        ],
    )
    .await;

    let claiming: Vec<i64> = {
        let mut ids: Vec<i64> = actions.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    assert_eq!(
        claiming,
        vec![1261_i64, 1271_i64],
        "a player holding both 702 and 703 on their travel steps must have \
         BOTH advance on one entry; got {actions:?}",
    );

    // Each advances its own mission, and neither touches the other's.
    assert!(
        actions.iter().any(|(id, a)| *id == 1261
            && matches!(
                a,
                Action::AdvanceStep {
                    mission_id: 702,
                    step_id: 2419
                }
            )),
        "chain 1261 must advance 702 to 2419; got {actions:?}",
    );
    assert!(
        actions.iter().any(|(id, a)| *id == 1271
            && matches!(
                a,
                Action::AdvanceStep {
                    mission_id: 703,
                    step_id: 2404
                }
            )),
        "chain 1271 must advance 703 to 2404; got {actions:?}",
    );
}

/// The other half of the same claim: holding only 703 must advance only
/// 703. This is what a player who took 703 without 702 (or who finished 702
/// on an earlier visit) sees.
#[tokio::test]
async fn only_703_advances_when_the_player_holds_only_703() {
    let pool = require_db_or_skip!();
    let actions = resolve_chains(
        &pool,
        &[1261, 1271],
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            (
                "mission_702_step_2402_status",
                serde_json::json!("completed"),
            ),
            ("mission_703_step_2403_status", serde_json::json!("active")),
        ],
    )
    .await;

    let claiming: Vec<i64> = {
        let mut ids: Vec<i64> = actions.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    assert_eq!(
        claiming,
        vec![1271_i64],
        "only 703's chain may fire once 702's travel step is done; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1272 — Romney dies on the kill step (the expected path)
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1272_romney_death_completes_703_and_grants_the_badge() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1272,
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Romney")),
            ("mission_703_step_2404_status", serde_json::json!("active")),
        ],
    )
    .await;

    // 2781 is 2404's only objective, so the mission completion completes
    // it. A hand-written `complete_objective` alongside would complete the
    // mission twice. Asserted BEFORE the action-list signature below, which
    // maps an unexpected action to "OTHER" and would otherwise trip first
    // and make this check unreachable.
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::CompleteObjective { .. })),
        "chain 1272 must not hand-complete objective 2781; got {actions:?}",
    );

    let signature: Vec<&str> = actions.iter().map(label).collect();
    assert_eq!(
        signature,
        vec!["add_item", "complete_mission"],
        "chain 1272 must grant before completing; got {actions:?}",
    );
    assert_badge_grant(&actions[0], "chain 1272");
    assert!(
        matches!(actions[1], Action::CompleteMission { mission_id: 703 }),
        "chain 1272 must complete 703; got {:?}",
        actions[1],
    );
}

/// Killing Romney with 703 already finished (or never taken) must grant
/// nothing — Romney respawns on the ordinary spawner timer for other
/// players, so every later death re-fires this trigger.
#[tokio::test]
async fn chain_1272_does_not_regrant_after_the_mission_is_done() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1272,
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Romney")),
            (
                "mission_703_step_2404_status",
                serde_json::json!("completed"),
            ),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1272 must be gated on step 2404 being active — otherwise every \
         later Romney kill re-grants the badge; got {actions:?}",
    );
}

/// A different NPC dying must not satisfy the tag filter.
#[tokio::test]
async fn chain_1272_does_not_resolve_for_another_tag() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1272,
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Muelbach")),
            ("mission_703_step_2404_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1272's entity_dead_tag key must be Castle_Romney; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1273 — Romney dies before the region was ever entered
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1273_off_path_death_advances_completes_and_grants() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1273,
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Romney")),
            ("mission_703_step_2403_status", serde_json::json!("active")),
        ],
    )
    .await;

    let signature: Vec<&str> = actions.iter().map(label).collect();
    assert_eq!(
        signature,
        vec!["advance_step", "add_item", "complete_mission"],
        "chain 1273 must advance 2403 → 2404 before completing, so objective \
         2780 is force-completed by the step transition rather than orphaned. \
         Got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::AdvanceStep {
                mission_id: 703,
                step_id: 2404
            }
        ),
        "chain 1273 must advance to 2404 first; got {:?}",
        actions[0],
    );
    assert_badge_grant(&actions[1], "chain 1273");
    assert!(
        matches!(actions[2], Action::CompleteMission { mission_id: 703 }),
        "chain 1273 must complete 703; got {:?}",
        actions[2],
    );
}

#[tokio::test]
async fn chain_1273_does_not_resolve_on_the_kill_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1273,
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Romney")),
            ("mission_703_step_2404_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1273 must be gated on step 2403 — 1272 owns the 2404 case; \
         got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// 1272 / 1273 mutual exclusion — the badge is granted exactly once
// ──────────────────────────────────────────────────────────────────────

/// Both death chains registered together, one kill. Exactly one chain may
/// claim it, and exactly one badge may be granted. If a future edit
/// loosened either gate (e.g. swapped a `step_status` for a
/// `mission_status 703 eq active`), both would match the same snapshot and
/// the player would get two badges and a doubled completion.
#[tokio::test]
async fn only_one_death_chain_claims_a_kill_on_the_expected_path() {
    let pool = require_db_or_skip!();
    let actions = resolve_chains(
        &pool,
        &[1272, 1273],
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Romney")),
            (
                "mission_703_step_2403_status",
                serde_json::json!("completed"),
            ),
            ("mission_703_step_2404_status", serde_json::json!("active")),
        ],
    )
    .await;

    // The grant count is asserted first: it is the invariant that actually
    // matters, and `claiming` below would trip on a double-fire before this
    // line ever ran.
    let grants = actions
        .iter()
        .filter(|(_, a)| matches!(a, Action::GrantItem { item_id: 2135, .. }))
        .count();
    assert_eq!(
        grants, 1,
        "exactly one badge per kill; got {grants} grants in {actions:?}",
    );

    let claiming: Vec<i64> = {
        let mut ids: Vec<i64> = actions.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    assert_eq!(
        claiming,
        vec![1272],
        "on the expected path only chain 1272 may fire; got {actions:?}",
    );
}

#[tokio::test]
async fn only_one_death_chain_claims_a_kill_on_the_off_path() {
    let pool = require_db_or_skip!();
    let actions = resolve_chains(
        &pool,
        &[1272, 1273],
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Romney")),
            ("mission_703_step_2403_status", serde_json::json!("active")),
        ],
    )
    .await;

    let grants = actions
        .iter()
        .filter(|(_, a)| matches!(a, Action::GrantItem { item_id: 2135, .. }))
        .count();
    assert_eq!(
        grants, 1,
        "exactly one badge per kill; got {grants} grants in {actions:?}",
    );

    let claiming: Vec<i64> = {
        let mut ids: Vec<i64> = actions.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    assert_eq!(
        claiming,
        vec![1273],
        "off the expected path only chain 1273 may fire; got {actions:?}",
    );
}

/// A player with 703 finished who kills a respawned Romney gets nothing
/// from either chain. This is the regression that matters most in a
/// persistent shared world: Romney is a normal spawner NPC and every
/// player in the zone can kill him repeatedly.
#[tokio::test]
async fn neither_death_chain_fires_after_703_is_complete() {
    let pool = require_db_or_skip!();
    let actions = resolve_chains(
        &pool,
        &[1272, 1273],
        TriggerType::EntityDeath,
        &[
            ("entity_tag", serde_json::json!("Castle_Romney")),
            ("mission_703_status", serde_json::json!("completed")),
            (
                "mission_703_step_2403_status",
                serde_json::json!("completed"),
            ),
            (
                "mission_703_step_2404_status",
                serde_json::json!("completed"),
            ),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "a repeat kill after 703 is done must grant nothing; got {actions:?}",
    );
}
