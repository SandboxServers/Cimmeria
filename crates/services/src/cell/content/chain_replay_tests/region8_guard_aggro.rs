//! Chain 1008 (`castle_cellblock_chains.sql`) — Region8 guard-aggro trap.
//!
//! Restored from the purged auto-export (former chain 5002 in
//! `space_castle_cellblock_chains.sql`, deleted per audit.md defect B3 /
//! decision D-CB02). The auto-export's trigger key was
//! `Castle_Cellblock.Region8` (lowercase b); the point set's actual name
//! is `Castle_CellBlock.Region8` (capital B) — see
//! `db/resources/Events/Seed/point_sets.sql`, set_id 2039, the ONLY
//! Cellblock region with that spelling — so the auto-export's trigger
//! could never match a real region-entry event. These tests pin the
//! corrected key and the two failure modes the purge fixed.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Positive case: entering Region8 with the corrected, byte-exact key
/// (`Castle_CellBlock.Region8`, capital B) resolves `SetAggression`
/// (level 1) followed by `GenerateThreat` (1000), both targeting
/// `ArmYourself_NIDGuard` — the Python's `setAggression(1)` then
/// `threatGenerated(player, 1000)` order (Castle_CellBlock.py
/// n120_trigger_In), not the auto-export's dropped-aggression /
/// threat_level=5000 shape.
#[tokio::test]
async fn chain_1008_fires_aggression_then_threat_on_correct_key() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1008)
        .await
        .expect("DB query for chain 1008 must succeed")
        .expect("chain 1008 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_CellBlock.Region8"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1008_actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1008)
        .map(|(_, a)| a)
        .collect();

    assert_eq!(
        chain_1008_actions.len(),
        2,
        "chain 1008 must resolve exactly two actions (SetAggression then \
         GenerateThreat) on the corrected key; got {chain_1008_actions:?}",
    );
    assert!(
        matches!(
            chain_1008_actions[0],
            Action::SetAggression { entity_tag, level: 1 }
            if entity_tag == "ArmYourself_NIDGuard"
        ),
        "first action must be SetAggression(ArmYourself_NIDGuard, level=1) \
         — the Python calls setAggression BEFORE threatGenerated; got {:?}",
        chain_1008_actions[0],
    );
    assert!(
        matches!(
            chain_1008_actions[1],
            Action::GenerateThreat { entity_tag: Some(tag), threat_level: 1000 }
            if tag == "ArmYourself_NIDGuard"
        ),
        "second action must be GenerateThreat(ArmYourself_NIDGuard, \
         threat_level=1000) — the Python's threatGenerated(player, 1000), \
         not the auto-export's dropped-aggression / 5000 shape; got {:?}",
        chain_1008_actions[1],
    );
}

/// Negative case A: the OLD (buggy) auto-export key, lowercase b in
/// "Cellblock", must NOT match. This is the literal shape of defect B3 —
/// pinning it guards against someone "fixing" the key back to the wrong
/// casing by copy-pasting from the (now-deleted) auto-export file or from
/// any other Cellblock region key in this seed, all of which use the
/// lowercase form.
#[tokio::test]
async fn chain_1008_does_not_match_old_lowercase_key() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1008)
        .await
        .expect("DB query for chain 1008 must succeed")
        .expect("chain 1008 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region8"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1008_fired = resolved.actions.iter().any(|(id, _)| *id == 1008);
    assert!(
        !chain_1008_fired,
        "chain 1008 must NOT match 'Castle_Cellblock.Region8' (lowercase b) \
         — that's the auto-export's broken key, byte-different from the \
         point set's real name. Got actions: {:?}",
        resolved.actions,
    );
}

/// Negative case B: the chain must not fire before the player actually
/// reaches Region8. Simulated by firing a `RegionEnter` for an earlier
/// region in the same zone (Region2, which the player crosses well
/// before Region8) — the trigger's exact-string `region_key` match means
/// this can only resolve if the chain (wrongly) ignored its own key.
#[tokio::test]
async fn chain_1008_does_not_fire_for_an_earlier_region() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1008)
        .await
        .expect("DB query for chain 1008 must succeed")
        .expect("chain 1008 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region2"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1008_fired = resolved.actions.iter().any(|(id, _)| *id == 1008);
    assert!(
        !chain_1008_fired,
        "chain 1008 must NOT fire before Region8 is actually entered \
         (simulated here by a RegionEnter for Region2). Got actions: {:?}",
        resolved.actions,
    );
}
