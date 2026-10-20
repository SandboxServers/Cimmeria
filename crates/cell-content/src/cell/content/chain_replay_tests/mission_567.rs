//! Mission 567 — Romney's Files, **step 4039 only** (packet H30,
//! `db/resources/Content/Seed/harset_opcore_chains.sql` chains 6503-6504).
//!
//! Both chains and the Castle leg ship **enabled** (FZ-12, decision D-FZ1).
//! Step 4039 needs item 2698 "Romney's Files", which Castle chain 1397 grants
//! (`castle_567_chains.sql`); Harset 6503 takes it back. Disable both legs
//! together or not at all.
//!
//! This file pins the flags (`live_db_chains_6503_and_6504_ship_enabled`,
//! `live_db_castle_567_chains_ship_enabled`) and proves the turn-in resolves
//! under its real context: a player on step 4039 in the Command Center gets
//! the four turn-in actions from 6503 and the indicator rebind from 6504.
//! Loading is asserted separately from resolving so "row missing" and "row
//! present but skipped" have different failure messages (TESTING.md type 6).

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// `resources.worlds.world_id` for `Harset_CmdCenter`, where Copplemann
/// (template 48, tag `CmdCenter_Copplemann`) stands.
const CMD_CENTER: i32 = 68;

/// The context that satisfies chain 6503: standing in the Command Center,
/// 567 active, step 4039 current.
fn copplemann_ctx() -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("CmdCenter_Copplemann"),
    );
    ctx.set_param(
        "mission_567_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_567_step_4039_status".to_string(),
        serde_json::json!("active"),
    );
    ctx
}

/// Both chains are present and ship enabled. Pinning `enabled == true` is
/// what keeps the Castle and Harset legs together: disabling one alone
/// strands a player on step 4039 holding item 2698, so this assertion fails
/// and names the chain.
#[tokio::test]
async fn live_db_chains_6503_and_6504_ship_enabled() {
    let pool = require_db_or_skip!();

    for chain_id in [6503, 6504] {
        let chain = load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

        assert!(
            chain.enabled,
            "chain {chain_id} must ship enabled with the Castle 567 leg (FZ-12); disable both legs together or not at all"
        );
    }
}

/// The nine Castle-leg chains of 567 (`castle_567_chains.sql`).
const CASTLE_567_CHAINS: [i32; 9] = [1394, 1395, 1396, 1397, 1400, 10240, 10241, 10242, 10243];

/// **The Castle leg of 567 ships enabled, as the seed is shipped: all nine
/// chains.** Loads each chain with no override, so this is the only test that
/// sees every shipped flag; the replay tests load them as shipped too.
/// Revert proof: set any Castle chain back to `false` in the seed and this
/// test fails naming that chain.
#[tokio::test]
async fn live_db_castle_567_chains_ship_enabled() {
    let pool = require_db_or_skip!();

    for chain_id in CASTLE_567_CHAINS {
        let chain = load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
        assert!(
            chain.enabled,
            "Castle 567 chain {chain_id} must ship enabled with Harset 6503/6504 (FZ-12); \
             disable both legs together or not at all"
        );
    }
}

/// Chain 6503 resolves the turn-in under the context that satisfies all three
/// of its conditions: Command Center, 567 active, step 4039 current.
#[tokio::test]
async fn live_db_chain_6503_resolves_the_turn_in() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6503)
        .await
        .expect("DB query for chain 6503 must succeed")
        .expect("chain 6503 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let ctx = copplemann_ctx();
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);

    assert_eq!(
        resolved.actions.len(),
        4,
        "chain 6503 must resolve its four turn-in actions for a player on step 4039; got {:?}",
        resolved.actions
    );
}

/// Chain 6504 (the relog restore) rebinds Copplemann's turn-in indicator when
/// a player on step 4039 enters the Command Center.
#[tokio::test]
async fn live_db_chain_6504_rebinds_the_turn_in() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6504)
        .await
        .expect("DB query for chain 6504 must succeed")
        .expect("chain 6504 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Harset_CmdCenter"),
    );
    ctx.set_param(
        "mission_567_step_4039_status".to_string(),
        serde_json::json!("active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    assert_eq!(
        engine.resolve_event(&event, &ctx).actions.len(),
        1,
        "chain 6504 must rebind Copplemann's turn-in on Command Center entry at step 4039"
    );
}

/// Chain 6503 is **correctly authored**: the four actions are exactly what
/// the turn-in needs, in order.
///
/// Reading the action list off the loaded chain (rather than resolving) pins
/// each action's type and ids, so a dialog id drifting or the `remove_item`
/// going missing fails here with the action named, not as a bare count
/// mismatch in `live_db_chain_6503_resolves_the_turn_in`.
#[tokio::test]
async fn live_db_chain_6503_is_authored() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6503)
        .await
        .expect("DB query for chain 6503 must succeed")
        .expect("chain 6503 must exist in seeded content_chains");

    assert_eq!(
        chain.actions.len(),
        4,
        "chain 6503 must carry exactly four actions; got {:?}",
        chain.actions
    );
    assert!(
        matches!(chain.actions[0], Action::DisplayDialog { dialog_id: 2044 }),
        "action 0 must play Copplemann's blurb 2044 (\"These files are going to help us \
         out a lot. Nice work.\", dsm 2817, speaker 968); got {:?}",
        chain.actions[0]
    );
    assert!(
        matches!(
            chain.actions[1],
            Action::RemoveItem {
                item_id: 2698,
                count: 1
            }
        ),
        "action 1 must consume exactly one Romney's Files (2698); got {:?}",
        chain.actions[1]
    );
    assert!(
        matches!(
            chain.actions[2],
            Action::RemoveDialogSet {
                dialog_set_id: 2817,
                slot: 48
            }
        ),
        "action 2 must clear the dsm 2817 bind from template slot 48 (Copplemann); got {:?}",
        chain.actions[2]
    );
    assert!(
        matches!(
            chain.actions[3],
            Action::CompleteMission { mission_id: 567 }
        ),
        "action 3 must COMPLETE 567 — step 4039 is its terminal step, so this also \
         closes objective 4652; got {:?}",
        chain.actions[3]
    );
}

/// The restore chain's single action is the matching bind. Same
/// rot-protection rationale as the test above.
#[tokio::test]
async fn live_db_chain_6504_is_authored_ready_too() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6504)
        .await
        .expect("DB query for chain 6504 must succeed")
        .expect("chain 6504 must exist in seeded content_chains");

    assert_eq!(chain.actions.len(), 1, "chain 6504 binds one dialog set");
    assert!(
        matches!(
            chain.actions[0],
            Action::AddDialogSet {
                dialog_set_id: 2817,
                slot: 48,
                mission_id: Some(567),
            }
        ),
        "chain 6504 must re-bind dsm 2817 to template slot 48 (Copplemann) for mission \
         567; got {:?}",
        chain.actions[0]
    );
}
