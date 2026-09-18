//! Mission 640 (Hack the Rings) — chains 1041-1046 in
//! `castle_cellblock_chains.sql`. Ring-switch Livewire hack, the
//! post-hack switch use that triggers the ring transporter, and the
//! teleport-in completion. See audit.md row 9 (C02) and
//! `work-packets.md`'s C02 scope.
//!
//! Flow: interact ring switch (step 2120 active) -> start Livewire
//! (chain 1041) -> victory directly invokes chain 1042 (advance to step
//! 2215, swap the interaction bit from "Livewire" to "RingNetwork" plus
//! a quest-highlight bit) -> interact ring switch again (step 2215
//! active) -> trigger the ring transporter to region 1 (chain 1043) ->
//! teleport-in to region 2 completes mission 640 while it's active
//! (chain 1044). Chains 1045/1046 restore the interaction bits on
//! relog per the Common Acceptance rule in work-packets.md ("(c)
//! asserts the relog-restore chain re-paints any interaction bit the
//! packet sets").

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1041 positive: interacting with the ring switch while step
/// 2120 is active starts a Livewire minigame whose victory re-dispatches
/// chain 1042.
#[tokio::test]
async fn chain_1041_starts_livewire_while_step_2120_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1041)
        .await
        .expect("DB query for chain 1041 must succeed")
        .expect("chain 1041 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("HackTheRings_Switch"),
    );
    ctx.set_param(
        "mission_640_step_2120_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let starts: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1041
                && matches!(
                    action,
                    Action::StartMinigame { minigame_type, on_victory_chains }
                    if minigame_type == "Livewire" && on_victory_chains == &vec![1042]
                )
        })
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "chain 1041 must resolve exactly one StartMinigame(Livewire, \
         on_victory_chains=[1042]) while step 2120 is active; got {:?}",
        resolved.actions,
    );
}

/// Chain 1041 negative: once step 2120 has already advanced past (the
/// player already hacked the switch and is on step 2215), re-interacting
/// with the switch must not start a second Livewire session.
#[tokio::test]
async fn chain_1041_does_not_fire_after_step_2120_completed() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1041)
        .await
        .expect("DB query for chain 1041 must succeed")
        .expect("chain 1041 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("HackTheRings_Switch"),
    );
    ctx.set_param(
        "mission_640_step_2120_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_640_step_2215_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1041_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1041)
        .count();
    assert_eq!(
        chain_1041_actions, 0,
        "chain 1041 must NOT fire once step 2120 has completed — it \
         would re-open Livewire after the player already hacked the \
         switch; got {chain_1041_actions} actions",
    );
}

/// Chain 1042 has no `content_triggers` row in the seed — it's invoked
/// directly by chain 1041's `on_victory_chains: [1042]` when the player
/// wins Livewire, not through `resolve_event`. `build_chains_from_rows`
/// gives triggerless chains a never-firing `OnCustomEvent` placeholder
/// (see `crates/content-engine/src/loader/mod.rs`), so this pins the
/// action list directly off the loaded `Chain` rather than firing a
/// synthetic event — same shape as `mission_641.rs`'s chain 1055 test.
#[tokio::test]
async fn chain_1042_livewire_victory_resolves_advance_and_icon_swap() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1042)
        .await
        .expect("DB query for chain 1042 must succeed")
        .expect("chain 1042 must exist in seeded content_chains");

    assert_eq!(
        chain.actions.len(),
        4,
        "chain 1042 must resolve exactly 4 actions (advance step + 3 \
         interaction-bit updates); got {:?}",
        chain.actions,
    );
    assert!(
        matches!(
            &chain.actions[0],
            Action::AdvanceStep {
                mission_id: 640,
                step_id: 2215
            }
        ),
        "chain 1042's first action must advance mission 640 to step \
         2215; got {:?}",
        chain.actions[0],
    );
    assert!(
        matches!(
            &chain.actions[1],
            Action::SetInteractionType { entity_tag, operation, mask: 256 }
            if entity_tag == "HackTheRings_Switch" && operation == "~"
        ),
        "chain 1042's second action must clear the Livewire interaction \
         bit (mask 256, op '~') on HackTheRings_Switch; got {:?}",
        chain.actions[1],
    );
    assert!(
        matches!(
            &chain.actions[2],
            Action::SetInteractionType { entity_tag, operation, mask: 32 }
            if entity_tag == "HackTheRings_Switch" && operation == "|"
        ),
        "chain 1042's third action must set the RingNetwork interaction \
         bit (mask 32, op '|') on HackTheRings_Switch; got {:?}",
        chain.actions[2],
    );
    assert!(
        matches!(
            &chain.actions[3],
            Action::SetInteractionType { entity_tag, operation, mask: 1073741824 }
            if entity_tag == "HackTheRings_Switch" && operation == "|"
        ),
        "chain 1042's fourth action must set the quest-highlight bit \
         (mask 2^30, op '|') on HackTheRings_Switch; got {:?}",
        chain.actions[3],
    );
}

/// Chain 1043 positive: interacting with the (post-hack) ring switch
/// while step 2215 is active triggers the ring transporter to region 1
/// and clears the quest-highlight bit.
#[tokio::test]
async fn chain_1043_triggers_transporter_region_1_while_step_2215_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1043)
        .await
        .expect("DB query for chain 1043 must succeed")
        .expect("chain 1043 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("HackTheRings_Switch"),
    );
    ctx.set_param(
        "mission_640_step_2215_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let transports: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1043 && matches!(action, Action::TriggerTransporter { region_id: 1 })
        })
        .collect();
    assert_eq!(
        transports.len(),
        1,
        "chain 1043 must resolve exactly one TriggerTransporter(region_id=1) \
         while step 2215 is active; got {:?}",
        resolved.actions,
    );

    let clears_highlight = resolved.actions.iter().any(|(id, action)| {
        *id == 1043
            && matches!(
                action,
                Action::SetInteractionType { entity_tag, operation, mask: 1073741824 }
                if entity_tag == "HackTheRings_Switch" && operation == "~"
            )
    });
    assert!(
        clears_highlight,
        "chain 1043 must clear the quest-highlight bit on \
         HackTheRings_Switch; got {:?}",
        resolved.actions,
    );
}

/// Chain 1043 negative: before the switch has been hacked (step 2120
/// still active, 2215 not yet reached), interacting with it must not
/// trigger the transporter — that would skip the Livewire minigame
/// entirely.
#[tokio::test]
async fn chain_1043_does_not_fire_before_step_2215_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1043)
        .await
        .expect("DB query for chain 1043 must succeed")
        .expect("chain 1043 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("HackTheRings_Switch"),
    );
    ctx.set_param(
        "mission_640_step_2120_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1043_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1043)
        .count();
    assert_eq!(
        chain_1043_actions, 0,
        "chain 1043 must NOT fire before step 2215 is active (the switch \
         hasn't been hacked yet); got {chain_1043_actions} actions",
    );
}

/// Chain 1044 positive: teleporting in to region 2 while mission 640 is
/// active completes it and enables the Preparation_ColMarsh interaction
/// bit.
#[tokio::test]
async fn chain_1044_completes_640_on_teleport_in_region_2_while_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1044)
        .await
        .expect("DB query for chain 1044 must succeed")
        .expect("chain 1044 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(2));
    ctx.set_param(
        "mission_640_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let completes = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1044 && matches!(action, Action::CompleteMission { mission_id: 640 })
        })
        .count();
    assert_eq!(
        completes, 1,
        "chain 1044 must resolve exactly one CompleteMission(640) on \
         teleport-in to region 2 while 640 is active; got {completes} \
         actions. Resolved: {:?}",
        resolved.actions,
    );

    let enables_marsh = resolved.actions.iter().any(|(id, action)| {
        *id == 1044
            && matches!(
                action,
                Action::SetInteractionType { entity_tag, operation, mask: 8388608 }
                if entity_tag == "Preparation_ColMarsh" && operation == "|"
            )
    });
    assert!(
        enables_marsh,
        "chain 1044 must set the Preparation_ColMarsh mission-available \
         bit (mask 2^23, op '|'); got {:?}",
        resolved.actions,
    );
}

/// Chain 1044 negative (before accept): teleporting in to region 2
/// before mission 640 has even been accepted must not complete it — a
/// player should never reach the ring room this way, but the gate must
/// hold regardless.
#[tokio::test]
async fn chain_1044_does_not_fire_before_640_accepted() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1044)
        .await
        .expect("DB query for chain 1044 must succeed")
        .expect("chain 1044 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(2));
    ctx.set_param(
        "mission_640_status".to_string(),
        serde_json::json!("not_active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let completes = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1044 && matches!(action, Action::CompleteMission { mission_id: 640 })
        })
        .count();
    assert_eq!(
        completes, 0,
        "chain 1044 must NOT complete 640 before it has been accepted; \
         got {completes} actions",
    );
}

/// Chain 1044 negative (already completed): teleporting in to region 2
/// again after 640 is already completed must not re-fire (no double
/// completion, no re-enabling of an already-enabled Marsh bit).
#[tokio::test]
async fn chain_1044_does_not_fire_after_640_already_completed() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1044)
        .await
        .expect("DB query for chain 1044 must succeed")
        .expect("chain 1044 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(2));
    ctx.set_param(
        "mission_640_status".to_string(),
        serde_json::json!("completed"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let completes = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1044 && matches!(action, Action::CompleteMission { mission_id: 640 })
        })
        .count();
    assert_eq!(
        completes, 0,
        "chain 1044 must NOT re-fire once 640 is already completed; got \
         {completes} actions",
    );
}

/// Chain 1045 positive (relog restore): a player who logs in mid-mission
/// with step 2120 still active must have the Livewire interaction bit
/// re-painted — interaction flags are not persisted on the entity across
/// server restarts.
#[tokio::test]
async fn chain_1045_restores_livewire_bit_on_relog_while_step_2120_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1045)
        .await
        .expect("DB query for chain 1045 must succeed")
        .expect("chain 1045 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_640_step_2120_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let restores: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1045
                && matches!(
                    action,
                    Action::SetInteractionType { entity_tag, operation, mask: 256 }
                    if entity_tag == "HackTheRings_Switch" && operation == "|"
                )
        })
        .collect();
    assert_eq!(
        restores.len(),
        1,
        "chain 1045 must restore exactly one Livewire interaction bit on \
         relog while step 2120 is active; got {:?}",
        resolved.actions,
    );
}

/// Chain 1045 negative: a player who has already advanced past step
/// 2120 (into 2215) must not have the stale Livewire bit re-painted on
/// relog.
#[tokio::test]
async fn chain_1045_does_not_fire_once_step_2120_advanced_past() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1045)
        .await
        .expect("DB query for chain 1045 must succeed")
        .expect("chain 1045 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_640_step_2120_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_640_step_2215_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1045_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1045)
        .count();
    assert_eq!(
        chain_1045_actions, 0,
        "chain 1045 must NOT fire once step 2120 has advanced past; got \
         {chain_1045_actions} actions",
    );
}

/// Chain 1046 positive (relog restore): a player who logs in with step
/// 2215 active must have both the RingNetwork bit and the quest
/// highlight bit re-painted.
#[tokio::test]
async fn chain_1046_restores_ringnetwork_and_highlight_on_relog_while_step_2215_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1046)
        .await
        .expect("DB query for chain 1046 must succeed")
        .expect("chain 1046 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_640_step_2215_status".to_string(),
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
        *id == 1046
            && matches!(
                action,
                Action::SetInteractionType { entity_tag, operation, mask: 32 }
                if entity_tag == "HackTheRings_Switch" && operation == "|"
            )
    });
    assert!(
        restores_ring_network,
        "chain 1046 must restore the RingNetwork bit on relog; got {:?}",
        resolved.actions,
    );

    let restores_highlight = resolved.actions.iter().any(|(id, action)| {
        *id == 1046
            && matches!(
                action,
                Action::SetInteractionType { entity_tag, operation, mask: 1073741824 }
                if entity_tag == "HackTheRings_Switch" && operation == "|"
            )
    });
    assert!(
        restores_highlight,
        "chain 1046 must restore the quest-highlight bit on relog; got {:?}",
        resolved.actions,
    );
}

/// Chain 1046 negative: a player still on step 2120 (hasn't hacked the
/// switch yet) must not have the post-hack RingNetwork bits painted on
/// relog.
#[tokio::test]
async fn chain_1046_does_not_fire_while_step_2120_still_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1046)
        .await
        .expect("DB query for chain 1046 must succeed")
        .expect("chain 1046 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_640_step_2120_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1046_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1046)
        .count();
    assert_eq!(
        chain_1046_actions, 0,
        "chain 1046 must NOT fire while step 2120 is still active (the \
         switch hasn't been hacked); got {chain_1046_actions} actions",
    );
}
