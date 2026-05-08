//! Mission 687 — Aftermath. Crate-highlight, archetype-gated reward
//! kits, and the barracks-kill counter completion threshold.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1097: when mission 687 is accepted (Region6 entry triggers
/// chain 1084, which calls accept_mission, which now fires the new
/// `mission_accepted` follow-up event), chain 1097 matches and
/// resolves a `SetInteractionType` action that highlights the
/// Cellblock_WoodenCrate as a quest world object. Without this, the
/// player has no visual cue that the crate is interactable.
#[tokio::test]
async fn chain_1097_highlights_wooden_crate_when_mission_687_accepted() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1097)
        .await
        .expect("DB query for chain 1097 must succeed")
        .expect("chain 1097 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(687));

    let event = TriggerEvent {
        trigger_type: TriggerType::MissionAccepted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let crate_highlights: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, action)| {
            if *id != 1097 {
                return None;
            }
            match action {
                Action::SetInteractionType {
                    entity_tag,
                    operation,
                    mask,
                } if entity_tag == "Cellblock_WoodenCrate"
                    && operation == "|"
                    && *mask == cimmeria_entity::interaction_flags::INT_MISSION_WORLD_OBJECT =>
                {
                    Some(action)
                }
                _ => None,
            }
        })
        .collect();
    assert_eq!(
        crate_highlights.len(),
        1,
        "chain 1097 must resolve exactly one SetInteractionType(crate, |, INT_MissionWorldObject) \
         action when mission 687 is accepted; got {} matches in {:?}",
        crate_highlights.len(),
        resolved.actions,
    );
}

/// Chain 1098: Tau'ri / non-Jaffa player searches the crate while
/// step 2354 is active → grants the 5-piece Covert Stealth set + Combat
/// Knife (6 items) to the backpack and advances to step 2355. The 5
/// armor pieces and the knife are the load-bearing payload — drift
/// (e.g. someone changes container_id away from 1, or drops one of
/// the items) shows up here.
#[tokio::test]
async fn chain_1098_grants_stealth_set_and_advances_step_for_non_jaffa() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1098)
        .await
        .expect("DB query for chain 1098 must succeed")
        .expect("chain 1098 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_WoodenCrate"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_687_step_2354_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(1)); // Soldier

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let granted: Vec<i32> = resolved
        .actions
        .iter()
        .filter_map(|(id, action)| match action {
            Action::GrantItem {
                item_id,
                count,
                container_id,
            } if *id == 1098 && *count == 1 && *container_id == Some(1) => Some(*item_id),
            _ => None,
        })
        .collect();
    // Pin the exact set, not just the count — a future seed change
    // that drops the Combat Knife or adds a stray item would slip
    // past a count-only assertion. Order matches the action
    // sort_order in the seed (helmet, vest, pants, gloves, boots, knife).
    assert_eq!(
        granted,
        vec![3347, 3359, 3372, 3387, 3401, 3325],
        "chain 1098 must grant the 5 stealth pieces + Combat Knife to the \
         backpack (container 1) in the documented order; got {granted:?}"
    );

    let advance_step_2355 = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1098
                && matches!(
                    action,
                    Action::AdvanceStep {
                        mission_id: 687,
                        step_id: 2355
                    }
                )
        })
        .count();
    assert_eq!(
        advance_step_2355, 1,
        "chain 1098 must advance to step 2355 after the rewards are granted"
    );
}

/// Chain 1098 negative: a Jaffa player searching the crate must NOT
/// match the non-Jaffa branch. Without the `archetype neq 8`
/// condition, both archetype branches would fire and the player
/// would receive both kits.
#[tokio::test]
async fn chain_1098_does_not_match_jaffa_archetype() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1098)
        .await
        .expect("DB query for chain 1098 must succeed")
        .expect("chain 1098 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_WoodenCrate"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_687_step_2354_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(8)); // Jaffa

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1098_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1098)
        .count();
    assert_eq!(
        chain_1098_actions, 0,
        "chain 1098 must NOT match for archetype 8 (Jaffa) — the archetype \
         neq 8 condition is what prevents both reward branches firing for \
         the same player; got {chain_1098_actions} actions",
    );
}

/// Chain 1099: Jaffa-branch sibling of 1098. Grants the Armored Prison
/// Jacket + Serpent Staff to the backpack and advances to step 2355.
#[tokio::test]
async fn chain_1099_grants_jaffa_kit_and_advances_step_for_jaffa() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1099)
        .await
        .expect("DB query for chain 1099 must succeed")
        .expect("chain 1099 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_WoodenCrate"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_687_step_2354_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(8)); // Jaffa

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let granted: Vec<i32> = resolved
        .actions
        .iter()
        .filter_map(|(id, action)| match action {
            Action::GrantItem {
                item_id,
                count,
                container_id,
            } if *id == 1099 && *count == 1 && *container_id == Some(1) => Some(*item_id),
            _ => None,
        })
        .collect();
    assert_eq!(
        granted,
        vec![3482, 2797],
        "chain 1099 must grant Armored Prison Jacket (3482) + Serpent Staff (2797) \
         to the backpack; got {granted:?}"
    );

    let advance_step_2355 = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1099
                && matches!(
                    action,
                    Action::AdvanceStep {
                        mission_id: 687,
                        step_id: 2355
                    }
                )
        })
        .count();
    assert_eq!(
        advance_step_2355, 1,
        "chain 1099 must advance to step 2355 after the rewards are granted"
    );
}

/// Chain 1103 positive: the player has already killed two barracks
/// guards (counter at 2), kills the third — counter is still at 2 at
/// the moment chain conditions are evaluated (the increment chain
/// 1100/1101/1102 mutates state during *execute*, not *resolve*, see
/// the ordering note at chain 1087). With the counter at 2 and the
/// `gte 2` threshold, chain 1103 must resolve `complete_mission 687`.
///
/// Loaded with `load_chain_expansions_for_test` because 1103 has three
/// trigger rows (one per guard tag) — a single-expansion load would
/// silently miss the OR shape and only test the Guard1 path.
#[tokio::test]
async fn chain_1103_completes_687_on_third_barracks_kill() {
    use super::super::engine_loader::load_chain_expansions_for_test;
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let expansions = load_chain_expansions_for_test(&pool, 1103)
        .await
        .expect("DB query for chain 1103 must succeed");
    assert_eq!(
        expansions.len(),
        3,
        "chain 1103 must materialize one in-memory Chain per Barracks_Guard \
         trigger row (3 total); got {}",
        expansions.len(),
    );

    let mut engine = ChainEngine::new();
    for chain in expansions {
        engine.register_chain(chain);
    }

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Barracks_Guard3"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_687_step_2355_status".to_string(),
        serde_json::json!("active"),
    );
    // Counter at 2 simulates "two prior kills already incremented the
    // counter; the third kill is the one being resolved now".
    ctx.set_param("counter_barracks_kills".to_string(), serde_json::json!(2));

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let complete_actions = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1103 && matches!(action, Action::CompleteMission { mission_id: 687 })
        })
        .count();
    assert_eq!(
        complete_actions, 1,
        "chain 1103 must resolve CompleteMission(687) when counter_barracks_kills >= 2 \
         (target - 1 threshold) on the third guard kill; got {complete_actions} actions \
         from chain 1103. Resolved actions: {:?}",
        resolved.actions,
    );
}

/// Chain 1103 negative: with `counter_barracks_kills = 0` (the player
/// just killed the first guard — counter not yet incremented at
/// resolve time), the `gte 2` condition fails and the completion
/// chain must NOT fire. This is the load-bearing pin against a
/// regression that lowers the threshold below 2 (which would
/// complete the mission on the first kill).
#[tokio::test]
async fn chain_1103_does_not_complete_687_on_first_barracks_kill() {
    use super::super::engine_loader::load_chain_expansions_for_test;
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let expansions = load_chain_expansions_for_test(&pool, 1103)
        .await
        .expect("DB query for chain 1103 must succeed");

    let mut engine = ChainEngine::new();
    for chain in expansions {
        engine.register_chain(chain);
    }

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Barracks_Guard1"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_687_step_2355_status".to_string(),
        serde_json::json!("active"),
    );
    // First kill: counter has not been incremented yet at this point
    // (resolve runs before execute), so the condition sees 0.
    ctx.set_param("counter_barracks_kills".to_string(), serde_json::json!(0));

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let complete_actions = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1103 && matches!(action, Action::CompleteMission { mission_id: 687 })
        })
        .count();
    assert_eq!(
        complete_actions, 0,
        "chain 1103 must NOT resolve CompleteMission on the first guard kill \
         (counter at 0 < gte 2 threshold); got {complete_actions} actions",
    );
}
