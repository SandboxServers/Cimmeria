//! Mission 688 — Secure the Armory. Pins the auto-accept on 687
//! complete, terminal-interact objective progression, ring-switch
//! mission completion + transporter trigger, and the login-restoration
//! chains. Together these exercise every chain in the 1105-1111 range
//! plus the new `objective_status` gating that depends on the
//! `mission_{m}_obj_{o}_status` population added in
//! `populate_mission_context`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1105: completing mission 687 must auto-accept mission 688.
/// This is the auto-progression that takes the player straight from
/// Aftermath into Secure the Armory without an NPC handoff.
#[tokio::test]
async fn chain_1105_auto_accepts_688_when_687_completes() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1105)
        .await
        .expect("DB query for chain 1105 must succeed")
        .expect("chain 1105 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(687));

    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let accept_count = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1105 && matches!(action, Action::AcceptMission { mission_id: 688 })
        })
        .count();
    assert_eq!(
        accept_count, 1,
        "chain 1105 must resolve exactly one AcceptMission(688) on \
         mission_completed 687; got {accept_count}",
    );
}

/// Chain 1106: mission 688 accepted → highlight `Cellblock_TerminalX`
/// so the player has a visual cue that the next objective is at the
/// armory terminal. Mirrors chain 1097 (687 accept → highlight
/// Cellblock_WoodenCrate).
#[tokio::test]
async fn chain_1106_highlights_armory_terminal_on_688_accept() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1106)
        .await
        .expect("DB query for chain 1106 must succeed")
        .expect("chain 1106 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(688));

    let event = TriggerEvent {
        trigger_type: TriggerType::MissionAccepted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let highlights: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, action)| {
            if *id != 1106 {
                return None;
            }
            match action {
                Action::SetInteractionType {
                    entity_tag,
                    operation,
                    mask,
                } if entity_tag == "Cellblock_TerminalX"
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
        highlights.len(),
        1,
        "chain 1106 must resolve one SetInteractionType(Cellblock_TerminalX, |, INT_MissionWorldObject); \
         got {} matches in {:?}",
        highlights.len(),
        resolved.actions,
    );
}

/// Chain 1107: interacting with the armory terminal while step 2356 is
/// active and obj 2734 is still active must complete obj 2734, clear the
/// terminal highlight, and highlight the ring switch. The
/// `objective_status` condition relies on the `mission_688_obj_2734_status`
/// context population added in `populate_mission_context` — without that
/// fix this chain would never fire.
#[tokio::test]
async fn chain_1107_completes_obj_2734_and_swaps_highlights() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1107)
        .await
        .expect("DB query for chain 1107 must succeed")
        .expect("chain 1107 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_TerminalX"),
    );
    ctx.set_param(
        "mission_688_step_2356_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_688_obj_2734_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == 1107 { Some(a) } else { None })
        .collect();

    let completed_obj = chain_actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::CompleteObjective {
                    mission_id: 688,
                    objective_id: 2734
                }
            )
        })
        .count();
    assert_eq!(
        completed_obj, 1,
        "chain 1107 must resolve CompleteObjective(688, 2734); got {completed_obj}",
    );

    let cleared_terminal = chain_actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::SetInteractionType { entity_tag, operation, mask: _ }
                    if entity_tag == "Cellblock_TerminalX" && operation == "~"
            )
        })
        .count();
    assert_eq!(
        cleared_terminal, 1,
        "chain 1107 must clear the terminal highlight; got {cleared_terminal}",
    );

    let highlighted_ring = chain_actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::SetInteractionType { entity_tag, operation, mask: _ }
                    if entity_tag == "Cellblock_ArmoryRingSwitch" && operation == "|"
            )
        })
        .count();
    assert_eq!(
        highlighted_ring, 1,
        "chain 1107 must highlight the ring switch; got {highlighted_ring}",
    );
}

/// Chain 1107 negative: the same interact_tag firing AFTER obj 2734 is
/// already complete must NOT re-complete the objective (would double-fire
/// the onObjectiveUpdate wire call). Pins the `objective_status eq active`
/// gate.
#[tokio::test]
async fn chain_1107_does_not_refire_when_obj_2734_already_completed() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1107)
        .await
        .expect("DB query for chain 1107 must succeed")
        .expect("chain 1107 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_TerminalX"),
    );
    ctx.set_param(
        "mission_688_step_2356_status".to_string(),
        serde_json::json!("active"),
    );
    // obj already completed
    ctx.set_param(
        "mission_688_obj_2734_status".to_string(),
        serde_json::json!("completed"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1107)
        .count();
    assert_eq!(
        n, 0,
        "chain 1107 must NOT match when obj 2734 already completed (gate is `eq active`); \
         got {n} actions",
    );
}

/// Chain 1108: killing the optional armory guard increments the
/// cosmetic counter while step 2356 is active.
#[tokio::test]
async fn chain_1108_increments_armory_kills_on_guard_death() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1108)
        .await
        .expect("DB query for chain 1108 must succeed")
        .expect("chain 1108 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_ArmoryGuard1"),
    );
    ctx.set_param(
        "mission_688_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_688_step_2356_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1108
                && matches!(
                    action,
                    Action::IncrementCounter { counter_name, amount }
                        if counter_name == "armory_kills" && *amount == 1
                )
        })
        .count();
    assert_eq!(n, 1, "chain 1108 must increment armory_kills by 1; got {n}",);
}

/// Chain 1109 — the load-bearing chain of mission 688. Interacting
/// with the ring switch while obj 2734 is complete must:
///   1. complete mission 688 (sort_order 0)
///   2. clear the ring highlight (sort_order 1)
///   3. trigger transporter region 33 (sort_order 2)
///
/// Order matters at execute-time: `complete_mission` flips the in-memory
/// `MissionInstance::status` to MISSION_COMPLETED before
/// `trigger_transporter` calls `handle_interact`, which then sees the
/// runtime `required_mission_id=688` gate satisfied. This test pins
/// the action set; the executor-level ordering is enforced by sort_order
/// and exercised by `ring_transport::tests`.
#[tokio::test]
async fn chain_1109_completes_688_and_triggers_transporter() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1109)
        .await
        .expect("DB query for chain 1109 must succeed")
        .expect("chain 1109 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_ArmoryRingSwitch"),
    );
    ctx.set_param(
        "mission_688_step_2356_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_688_obj_2734_status".to_string(),
        serde_json::json!("completed"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == 1109 { Some(a) } else { None })
        .collect();

    let completes = chain_actions
        .iter()
        .filter(|a| matches!(a, Action::CompleteMission { mission_id: 688 }))
        .count();
    assert_eq!(
        completes, 1,
        "chain 1109 must resolve CompleteMission(688); got {completes}",
    );

    let triggers = chain_actions
        .iter()
        .filter(|a| matches!(a, Action::TriggerTransporter { region_id: 33 }))
        .count();
    assert_eq!(
        triggers, 1,
        "chain 1109 must resolve TriggerTransporter(33); got {triggers}",
    );

    let cleared = chain_actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::SetInteractionType { entity_tag, operation, mask: _ }
                    if entity_tag == "Cellblock_ArmoryRingSwitch" && operation == "~"
            )
        })
        .count();
    assert_eq!(
        cleared, 1,
        "chain 1109 must clear the ring highlight; got {cleared}",
    );
}

/// Chain 1109 negative: ring interaction must NOT complete mission 688
/// while obj 2734 is still active. Without the `objective_status`
/// gate (and the population that backs it) a player could skip the
/// terminal step entirely. Both gates — chain-level (here) and
/// runtime-level (`required_mission_id=688` in the cell runtime) —
/// must hold for the design intent to be met.
#[tokio::test]
async fn chain_1109_does_not_complete_when_obj_2734_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1109)
        .await
        .expect("DB query for chain 1109 must succeed")
        .expect("chain 1109 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Cellblock_ArmoryRingSwitch"),
    );
    ctx.set_param(
        "mission_688_step_2356_status".to_string(),
        serde_json::json!("active"),
    );
    // obj still active — terminal not yet used.
    ctx.set_param(
        "mission_688_obj_2734_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1109)
        .count();
    assert_eq!(
        n, 0,
        "chain 1109 must NOT match when obj 2734 still active; got {n} actions",
    );
}

/// Chains 1110/1111 — login restoration. Re-applying the highlight on
/// `player_loaded Castle_CellBlock` is required because
/// `interaction_type` flags don't persist across server restarts.
/// These two chains pick which entity to highlight based on whether
/// obj 2734 has been completed yet.
#[tokio::test]
async fn chain_1110_restores_terminal_highlight_when_obj_2734_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1110)
        .await
        .expect("DB query for chain 1110 must succeed")
        .expect("chain 1110 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_688_step_2356_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_688_obj_2734_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1110
                && matches!(
                    action,
                    Action::SetInteractionType { entity_tag, operation, mask: _ }
                        if entity_tag == "Cellblock_TerminalX" && operation == "|"
                )
        })
        .count();
    assert_eq!(
        n, 1,
        "chain 1110 must restore the terminal highlight on login; got {n}",
    );
}

#[tokio::test]
async fn chain_1111_restores_ring_highlight_when_obj_2734_completed() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1111)
        .await
        .expect("DB query for chain 1111 must succeed")
        .expect("chain 1111 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_688_step_2356_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_688_obj_2734_status".to_string(),
        serde_json::json!("completed"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1111
                && matches!(
                    action,
                    Action::SetInteractionType { entity_tag, operation, mask: _ }
                        if entity_tag == "Cellblock_ArmoryRingSwitch" && operation == "|"
                )
        })
        .count();
    assert_eq!(
        n, 1,
        "chain 1111 must restore the ring switch highlight on login; got {n}",
    );
}
