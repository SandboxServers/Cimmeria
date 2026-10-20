//! The Harset hand-off of mission 567 (CS-08b2): the state the Castle leg
//! leaves must satisfy chains 6503 and 6504. A child of
//! [`super`] so it shares that harness.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED};

use super::super::super::engine_loader::load_single_chain_for_test;
use super::super::super::mission_context::{populate_mission_context, populate_world_context};
use super::{
    accept, arrive, bound_at_copplemann, click, completed, cross, engine_with, log_in, mission_of,
    on_the_leg, staged, ALL_CHAINS, CASTLE, CMD_CENTER, ENTITY, HARSET_CHAINS, STEP_EXIT,
    STEP_FETCH, STEP_TURN_IN, TURN_IN_SET,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

/// Evaluate every condition row of `chain` against `ctx`; the labels of the
/// rows that fail.
fn failing_conditions(
    chain: &cimmeria_content_engine::chain::Chain,
    ctx: &ExecutionContext,
) -> Vec<String> {
    chain
        .conditions
        .iter()
        .filter(|c| !c.evaluate(ctx))
        .map(|c| format!("{c:?}"))
        .collect()
}

/// The context 6503 (`interact_tag`) and 6504 (`player_loaded`) judge: the
/// world and mission state the cell derives, plus both trigger params.
fn harset_ctx(mgr: &SpaceManager) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    populate_world_context(ENTITY, mgr, &mut ctx);
    populate_mission_context(mgr.get_entity(ENTITY).unwrap(), &mut ctx);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("CmdCenter_Copplemann"),
    );
    ctx.set_param("world_name".to_string(), serde_json::json!(CMD_CENTER));
    ctx
}

/// **The state the Castle leg leaves satisfies every condition of 6503 and
/// 6504, and 6503's removal matches 1397's grant.** The whole leg runs through
/// the real dispatchers (login, accept, cabinet, crossing); the player is
/// then hydrated in world 68. Asserted: one grant of 2698 in the whole leg,
/// 567 active at 4039 in world 68 with 2000 and 2012 done, the offer unbound,
/// every condition row of both chains true, 6504 binds exactly 2817, and 6503
/// shows 2044, removes 2698 x1 and completes 567. A control one step short
/// (2012) fails a condition of each chain. Revert proof: make 10241 advance
/// to 2012 instead of 4039 and the step assertion fails; change 6503's
/// `remove_item` qty or 1397's grant qty and the pair comparison fails.
#[tokio::test]
async fn live_db_mission_567_castle_meets_the_harset_contract() {
    let pool = require_db_or_skip!();
    let mut ids = ALL_CHAINS.to_vec();
    ids.extend(HARSET_CHAINS);
    let engine = engine_with(&pool, &ids).await;

    // The Castle leg, start to finish.
    let mut castle = staged(&pool, CASTLE, vec![completed(701), completed(703)]).await;
    let mut leg = log_in(&engine, &mut castle, CASTLE).await;
    leg.extend(accept(&engine, &mut castle).await);
    leg.extend(click(&engine, &mut castle, "Castle_FileCabinet").await.1);
    leg.extend(cross(&engine, &mut castle, "Harset").await);
    assert_eq!(
        leg,
        [
            "step 2000",
            "grant 2698 x1 container 2",
            "step 2012",
            "dialog 10733",
            "step 4039"
        ],
        "the leg grants once and ends at 4039"
    );
    assert!(
        bound_at_copplemann(&castle).is_empty(),
        "the offer set must be removed by the accept"
    );

    // Arrival in the Command Center (world 68).
    let mut harset = arrive(&pool, &castle, CMD_CENTER).await;
    assert_eq!(harset.world_id_for_world(CMD_CENTER), Some(68));
    let held = mission_of(&harset).expect("567 must come across");
    assert_eq!(held.status, MISSION_ACTIVE, "567 must be active");
    assert_eq!(held.current_step_id, Some(STEP_TURN_IN), "step 4039 active");
    assert!(
        held.completed_steps.contains(&STEP_FETCH) && held.completed_steps.contains(&STEP_EXIT),
        "steps 2000 and 2012 must be done; got {:?}",
        held.completed_steps
    );
    assert!(
        bound_at_copplemann(&harset).is_empty(),
        "no bind on arrival"
    );

    // Every condition row of both Harset chains holds.
    let c6503 = load_single_chain_for_test(&pool, 6503)
        .await
        .unwrap()
        .unwrap();
    let c6504 = load_single_chain_for_test(&pool, 6504)
        .await
        .unwrap()
        .unwrap();
    let ctx = harset_ctx(&harset);
    for chain in [&c6503, &c6504] {
        assert!(
            !chain.conditions.is_empty(),
            "chain {} has conditions",
            chain.id
        );
        let failing = failing_conditions(chain, &ctx);
        assert!(
            failing.is_empty(),
            "chain {} must accept the Castle leg's state; failing: {failing:?}",
            chain.id
        );
    }

    // The control: one step short, each chain refuses.
    let short = staged(&pool, CMD_CENTER, on_the_leg(STEP_EXIT, 2350)).await;
    for chain in [&c6503, &c6504] {
        assert!(
            !failing_conditions(chain, &harset_ctx(&short)).is_empty(),
            "chain {} must refuse a player still at 2012",
            chain.id
        );
    }

    // The removal matches the grant: same design id and quantity.
    let c1397 = load_single_chain_for_test(&pool, 1397)
        .await
        .unwrap()
        .unwrap();
    let granted: Vec<(i32, i32)> = c1397
        .actions
        .iter()
        .filter_map(|a| match a {
            Action::GrantItem { item_id, count, .. } => Some((*item_id, *count)),
            _ => None,
        })
        .collect();
    let removed: Vec<(i32, i32)> = c6503
        .actions
        .iter()
        .filter_map(|a| match a {
            Action::RemoveItem { item_id, count } => Some((*item_id, *count)),
            _ => None,
        })
        .collect();
    assert_eq!(granted, [(2698, 1)], "1397 grants exactly one 2698");
    assert_eq!(removed, granted, "6503 must remove what 1397 granted");

    // The hand-off behaves: bind on load, then turn in.
    log_in(&engine, &mut harset, CMD_CENTER).await;
    assert_eq!(
        bound_at_copplemann(&harset),
        vec![TURN_IN_SET],
        "6504 binds exactly the turn-in set, never the offer"
    );
    let (handled, out) = click(&engine, &mut harset, "CmdCenter_Copplemann").await;
    assert!(handled, "6503 must answer Copplemann");
    assert_eq!(out, ["dialog 2044", "remove 2698 x1"]);
    assert_eq!(
        mission_of(&harset).map(|m| m.status),
        Some(MISSION_COMPLETED),
        "6503 must complete 567"
    );
    assert!(
        bound_at_copplemann(&harset).is_empty(),
        "the turn-in bind is cleared"
    );
}
