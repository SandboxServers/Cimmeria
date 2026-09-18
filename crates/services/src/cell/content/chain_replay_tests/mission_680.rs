//! Mission 680 (Escape the Cellblock) — chains 1071-1074 in
//! `castle_cellblock_chains.sql`. The second ring switch (Preparation
//! room to topside), the teleport-in step advance, and the Region9
//! completion/accept pair. See audit.md row 15 (C02) and
//! `work-packets.md`'s C02 scope.
//!
//! Flow: interact the Preparation ring switch (step 2344 active) ->
//! trigger the ring transporter to region 2 (chain 1071) -> teleport-in
//! to region 3 advances mission 680 to step 2345 (chain 1072) ->
//! entering `Castle_Cellblock.Region9` completes mission 680 AND accepts
//! mission 681 in the SAME resolution (chain 1073).
//!
//! Chain 1073 fires on region **entry**, not exit — this is a
//! deliberate Cimmeria deviation from `EscapeTheCellblock.py` /
//! `MessHall.py`, which complete 680 on leaving Region9 rather than
//! entering it (audit.md row 15: "680 completes on Region9 **enter**, a
//! deliberate deviation"). This suite pins that exact shape; it does
//! NOT "fix" it to match the Python exit-based trigger.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1071 positive: interacting with the Preparation ring switch
/// while step 2344 is active triggers the ring transporter to region 2.
#[tokio::test]
async fn chain_1071_triggers_transporter_region_2_while_step_2344_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1071)
        .await
        .expect("DB query for chain 1071 must succeed")
        .expect("chain 1071 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Preparation_RingSwitch"),
    );
    ctx.set_param(
        "mission_680_step_2344_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let transports = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1071 && matches!(action, Action::TriggerTransporter { region_id: 2 })
        })
        .count();
    assert_eq!(
        transports, 1,
        "chain 1071 must resolve exactly one TriggerTransporter(region_id=2) \
         while step 2344 is active; got {transports} actions",
    );
}

/// Chain 1071 negative: before step 2344 is reached (still on an
/// earlier 680 step), interacting with the switch must not trigger the
/// transporter.
#[tokio::test]
async fn chain_1071_does_not_fire_before_step_2344_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1071)
        .await
        .expect("DB query for chain 1071 must succeed")
        .expect("chain 1071 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Preparation_RingSwitch"),
    );
    // 2344 defaults to "not_active" when unset — leave it unset to
    // simulate "haven't reached this step yet".

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1071_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1071)
        .count();
    assert_eq!(
        chain_1071_actions, 0,
        "chain 1071 must NOT fire before step 2344 is active; got \
         {chain_1071_actions} actions",
    );
}

/// Chain 1072 positive: teleporting in to region 3 advances mission 680
/// to step 2345 and clears the ring-switch interaction bits. Chain 1072
/// has no gating condition in the seed (any teleport-in to region 3
/// advances the step) — the trigger's `region_id` match is what scopes
/// it.
#[tokio::test]
async fn chain_1072_advances_to_step_2345_on_teleport_in_region_3() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1072)
        .await
        .expect("DB query for chain 1072 must succeed")
        .expect("chain 1072 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(3));

    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let advances = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1072
                && matches!(
                    action,
                    Action::AdvanceStep {
                        mission_id: 680,
                        step_id: 2345
                    }
                )
        })
        .count();
    assert_eq!(
        advances, 1,
        "chain 1072 must resolve exactly one AdvanceStep(680, 2345) on \
         teleport-in to region 3; got {advances} actions. Resolved: {:?}",
        resolved.actions,
    );

    let clears_switch_bit = resolved.actions.iter().any(|(id, action)| {
        *id == 1072
            && matches!(
                action,
                Action::SetInteractionType { entity_tag, operation, mask: 32 }
                if entity_tag == "Preparation_RingSwitch" && operation == "~"
            )
    });
    assert!(
        clears_switch_bit,
        "chain 1072 must clear the RingNetwork bit on \
         Preparation_RingSwitch; got {:?}",
        resolved.actions,
    );
}

/// Chain 1072 negative: teleporting in to a different region (e.g.
/// region 2, the ring switch's own landing spot) must not advance step
/// 2345 — the trigger's `region_id` filter is what scopes this to the
/// topside arrival.
#[tokio::test]
async fn chain_1072_does_not_fire_on_teleport_in_to_a_different_region() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1072)
        .await
        .expect("DB query for chain 1072 must succeed")
        .expect("chain 1072 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(2));

    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1072_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1072)
        .count();
    assert_eq!(
        chain_1072_actions, 0,
        "chain 1072 must NOT fire on teleport-in to a region other than \
         3; got {chain_1072_actions} actions",
    );
}

/// Chain 1073 positive: entering `Castle_Cellblock.Region9` while
/// mission 681 has not yet been accepted completes mission 680 AND
/// accepts mission 681 in the same resolution. This is the deliberate
/// Cimmeria deviation from the Python (which completes 680 on Region9
/// *exit*) — pinned here exactly as shipped, per audit.md row 15 and
/// the C02 packet instruction not to "fix" it toward the Python shape.
#[tokio::test]
async fn chain_1073_completes_680_and_accepts_681_on_region9_entry() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1073)
        .await
        .expect("DB query for chain 1073 must succeed")
        .expect("chain 1073 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region9"),
    );
    ctx.set_param(
        "mission_681_status".to_string(),
        serde_json::json!("not_active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let completes_680 = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1073 && matches!(action, Action::CompleteMission { mission_id: 680 })
        })
        .count();
    assert_eq!(
        completes_680, 1,
        "chain 1073 must resolve exactly one CompleteMission(680) on \
         Region9 ENTRY (not exit — deliberate Cimmeria deviation, see \
         audit.md row 15); got {completes_680} actions. Resolved: {:?}",
        resolved.actions,
    );

    let accepts_681 = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1073 && matches!(action, Action::AcceptMission { mission_id: 681 })
        })
        .count();
    assert_eq!(
        accepts_681, 1,
        "chain 1073 must resolve exactly one AcceptMission(681) in the \
         SAME resolution as completing 680; got {accepts_681} actions. \
         Resolved: {:?}",
        resolved.actions,
    );

    // Ordering matters for the "one resolution" claim: complete_mission
    // must precede accept_mission (declaration order in the seed,
    // sort_order 0 then 1).
    let complete_idx = resolved
        .actions
        .iter()
        .position(|(id, a)| *id == 1073 && matches!(a, Action::CompleteMission { mission_id: 680 }))
        .expect("CompleteMission(680) must be present");
    let accept_idx = resolved
        .actions
        .iter()
        .position(|(id, a)| *id == 1073 && matches!(a, Action::AcceptMission { mission_id: 681 }))
        .expect("AcceptMission(681) must be present");
    assert!(
        complete_idx < accept_idx,
        "chain 1073 must resolve CompleteMission(680) before \
         AcceptMission(681) (seed sort_order 0 then 1); got indices \
         {complete_idx} and {accept_idx} in {:?}",
        resolved.actions,
    );
}

/// Chain 1073 negative: re-entering Region9 after 681 has already been
/// accepted (e.g. the player backtracks) must not re-complete 680 or
/// re-accept 681.
#[tokio::test]
async fn chain_1073_does_not_fire_once_681_already_accepted() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1073)
        .await
        .expect("DB query for chain 1073 must succeed")
        .expect("chain 1073 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region9"),
    );
    ctx.set_param(
        "mission_681_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1073_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1073)
        .count();
    assert_eq!(
        chain_1073_actions, 0,
        "chain 1073 must NOT fire once mission 681 has already been \
         accepted; got {chain_1073_actions} actions",
    );
}

/// Chain 1074 positive (relog restore): a player who logs in with step
/// 2344 still active must have both the RingNetwork bit and the quest
/// highlight bit re-painted on the Preparation ring switch.
#[tokio::test]
async fn chain_1074_restores_ringswitch_bits_on_relog_while_step_2344_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1074)
        .await
        .expect("DB query for chain 1074 must succeed")
        .expect("chain 1074 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_680_step_2344_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let restores_ring_network = resolved.actions.iter().any(|(id, action)| {
        *id == 1074
            && matches!(
                action,
                Action::SetInteractionType { entity_tag, operation, mask: 32 }
                if entity_tag == "Preparation_RingSwitch" && operation == "|"
            )
    });
    assert!(
        restores_ring_network,
        "chain 1074 must restore the RingNetwork bit on relog; got {:?}",
        resolved.actions,
    );

    let restores_highlight = resolved.actions.iter().any(|(id, action)| {
        *id == 1074
            && matches!(
                action,
                Action::SetInteractionType { entity_tag, operation, mask: 1073741824 }
                if entity_tag == "Preparation_RingSwitch" && operation == "|"
            )
    });
    assert!(
        restores_highlight,
        "chain 1074 must restore the quest-highlight bit on relog; got {:?}",
        resolved.actions,
    );
}

/// Chain 1074 negative: a player who has already advanced past step
/// 2344 (into 2345, topside) must not have the stale ring-switch bits
/// re-painted on relog.
#[tokio::test]
async fn chain_1074_does_not_fire_once_step_2344_advanced_past() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1074)
        .await
        .expect("DB query for chain 1074 must succeed")
        .expect("chain 1074 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_680_step_2344_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_680_step_2345_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1074_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1074)
        .count();
    assert_eq!(
        chain_1074_actions, 0,
        "chain 1074 must NOT fire once step 2344 has advanced past; got \
         {chain_1074_actions} actions",
    );
}
