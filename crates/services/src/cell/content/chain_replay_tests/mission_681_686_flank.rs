//! Flanking objectives 2725 (mission 681, Mess Hall) and 2731 (mission
//! 686, Hallway05) — Castle Cellblock C06.
//!
//! Chains 1141/1142 complete the flank objective on a `player_flanked_npc`
//! event for an "NID Guard" while the mission is active. Per D-CB05 the
//! flank objective is tracked but does NOT gate: the kill-counter chains
//! (1087/1094) still complete the mission on their own, and the flank
//! chains never touch mission state beyond the single objective.
//!
//! The guards worth pinning here:
//!   - the chain resolves ONLY for the player-perspective trigger. The
//!     NPC-perspective `npc_flanked` event runs its actions on the NPC with
//!     player id 0, so a cross-match would call `complete_objective` on the
//!     wrong entity;
//!   - the resolved action list is exactly one `CompleteObjective` — no
//!     `CompleteMission`/`AdvanceStep`, because
//!     `cell::missions::complete_objective` completes the whole mission
//!     when every required objective is done (steps 2348/2353 each carry a
//!     kill objective that this path never completes);
//!   - self-completion guard: the AI re-fires the flank event on every
//!     cover release, so an already-completed objective must not re-fire.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_chain_expansions_for_test;
use crate::test_support::require_db_or_skip;

/// Registers EVERY trigger expansion of each chain: the loader emits one
/// `Chain` per `content_triggers` row, and chain 1087 fires on either
/// MessHall guard (two trigger rows) — `load_single_chain_for_test` would
/// silently register only the Guard1 expansion.
async fn engine_with(pool: &sqlx::PgPool, chain_ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &id in chain_ids {
        let expansions = load_chain_expansions_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"));
        assert!(
            !expansions.is_empty(),
            "chain {id} must exist in seeded content_chains"
        );
        for chain in expansions {
            engine.register_chain(chain);
        }
    }
    engine
}

/// A flank event as `fire_player_flanked_npc` / `fire_npc_flanked` build
/// it: `npc_template` param plus the mission/objective status keys
/// `populate_mission_context` derives from the player's saved missions.
fn flank_event(
    trigger_type: TriggerType,
    template: &str,
    mission_id: i32,
    mission_status: &str,
    objective: Option<(i32, &str)>,
) -> (ExecutionContext, TriggerEvent) {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("npc_template".to_string(), serde_json::json!(template));
    ctx.set_param(
        format!("mission_{mission_id}_status"),
        serde_json::json!(mission_status),
    );
    if let Some((objective_id, status)) = objective {
        ctx.set_param(
            format!("mission_{mission_id}_obj_{objective_id}_status"),
            serde_json::json!(status),
        );
    }
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    (ctx, event)
}

fn actions_of(
    engine: &ChainEngine,
    chain_id: i64,
    ctx: &ExecutionContext,
    event: &TriggerEvent,
) -> Vec<Action> {
    engine
        .resolve_event(event, ctx)
        .actions
        .into_iter()
        .filter_map(|(id, a)| (id == chain_id).then_some(a))
        .collect()
}

// ── Chain 1141: Mess Hall flank → objective 2725 ────────────────────────

#[tokio::test]
async fn chain_1141_completes_flank_objective_2725_and_nothing_else() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1141]).await;

    let (ctx, event) = flank_event(
        TriggerType::PlayerFlankedNpc,
        "NID Guard",
        681,
        "active",
        Some((2725, "active")),
    );
    let actions = actions_of(&engine, 1141, &ctx, &event);

    assert_eq!(
        actions.len(),
        1,
        "chain 1141 must resolve exactly one action; got {actions:?}"
    );
    assert!(
        matches!(
            actions[0],
            Action::CompleteObjective {
                mission_id: 681,
                objective_id: 2725
            }
        ),
        "chain 1141 must complete flank objective 2725 and nothing else (no \
         CompleteMission/AdvanceStep — the kill objective 2724 is still open); got {:?}",
        actions[0]
    );
}

#[tokio::test]
async fn chain_1141_does_not_fire_when_mission_681_not_active() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1141]).await;

    for status in ["not_active", "completed"] {
        let (ctx, event) = flank_event(
            TriggerType::PlayerFlankedNpc,
            "NID Guard",
            681,
            status,
            Some((2725, "active")),
        );
        let actions = actions_of(&engine, 1141, &ctx, &event);
        assert!(
            actions.is_empty(),
            "chain 1141 must not fire while mission 681 is {status}; got {actions:?}"
        );
    }
}

#[tokio::test]
async fn chain_1141_does_not_refire_once_2725_already_completed() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1141]).await;

    let (ctx, event) = flank_event(
        TriggerType::PlayerFlankedNpc,
        "NID Guard",
        681,
        "active",
        Some((2725, "completed")),
    );
    let actions = actions_of(&engine, 1141, &ctx, &event);
    assert!(
        actions.is_empty(),
        "the AI re-fires the flank event on every cover release; a completed \
         2725 must not resolve again; got {actions:?}"
    );
}

#[tokio::test]
async fn chain_1141_does_not_fire_for_a_non_guard_template() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1141]).await;

    let (ctx, event) = flank_event(
        TriggerType::PlayerFlankedNpc,
        "Jaffa Warrior",
        681,
        "active",
        Some((2725, "active")),
    );
    let actions = actions_of(&engine, 1141, &ctx, &event);
    assert!(
        actions.is_empty(),
        "only 'NID Guard' flanks count toward 2725; got {actions:?}"
    );
}

/// Bug-shape guard for the whole reason `player_flanked_npc` exists: the
/// NPC-perspective `npc_flanked` event executes against the NPC with
/// player id 0, so the flank chains must not resolve on it.
#[tokio::test]
async fn chain_1141_does_not_resolve_on_the_npc_perspective_flank_event() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1141, 1142]).await;

    let (ctx, event) = flank_event(
        TriggerType::NpcFlanked,
        "NID Guard",
        681,
        "active",
        Some((2725, "active")),
    );
    assert!(
        actions_of(&engine, 1141, &ctx, &event).is_empty(),
        "chain 1141 must bind player_flanked_npc, not npc_flanked"
    );
    assert!(
        actions_of(&engine, 1142, &ctx, &event).is_empty(),
        "chain 1142 must bind player_flanked_npc, not npc_flanked"
    );
}

#[tokio::test]
async fn chain_1141_ignores_a_flank_while_only_mission_686_is_active() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1141]).await;

    // Mission 686 active, 681 absent from the context entirely.
    let (ctx, event) = flank_event(
        TriggerType::PlayerFlankedNpc,
        "NID Guard",
        686,
        "active",
        Some((2731, "active")),
    );
    assert!(
        actions_of(&engine, 1141, &ctx, &event).is_empty(),
        "a Hallway05 flank must not complete the Mess Hall objective"
    );
}

// ── Chain 1142: Hallway05 flank → objective 2731 ────────────────────────

#[tokio::test]
async fn chain_1142_completes_flank_objective_2731_and_nothing_else() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1142]).await;

    let (ctx, event) = flank_event(
        TriggerType::PlayerFlankedNpc,
        "NID Guard",
        686,
        "active",
        Some((2731, "active")),
    );
    let actions = actions_of(&engine, 1142, &ctx, &event);

    assert_eq!(
        actions.len(),
        1,
        "chain 1142 must resolve exactly one action; got {actions:?}"
    );
    assert!(
        matches!(
            actions[0],
            Action::CompleteObjective {
                mission_id: 686,
                objective_id: 2731
            }
        ),
        "chain 1142 must complete flank objective 2731 and nothing else (686's \
         completion drives the Straegis scene — only the kill counter may end it); got {:?}",
        actions[0]
    );
}

#[tokio::test]
async fn chain_1142_does_not_fire_when_mission_686_not_active() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1142]).await;

    for status in ["not_active", "completed"] {
        let (ctx, event) = flank_event(
            TriggerType::PlayerFlankedNpc,
            "NID Guard",
            686,
            status,
            Some((2731, "active")),
        );
        let actions = actions_of(&engine, 1142, &ctx, &event);
        assert!(
            actions.is_empty(),
            "chain 1142 must not fire while mission 686 is {status}; got {actions:?}"
        );
    }
}

#[tokio::test]
async fn chain_1142_does_not_refire_once_2731_already_completed() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1142]).await;

    let (ctx, event) = flank_event(
        TriggerType::PlayerFlankedNpc,
        "NID Guard",
        686,
        "active",
        Some((2731, "completed")),
    );
    assert!(
        actions_of(&engine, 1142, &ctx, &event).is_empty(),
        "a completed 2731 must not resolve again"
    );
}

// ── D-CB05: the flank objective never gates the kill completion ─────────

/// Kill-without-flank AND flank-then-kill: the Mess Hall completion chain
/// (1087) resolves `CompleteMission(681)` + `AcceptMission(682)` on the
/// second guard death whether or not 2725 was ever completed, and with the
/// flank chain loaded alongside it never adds an objective action of its
/// own to a death event.
#[tokio::test]
async fn kill_completes_681_with_or_without_the_flank_objective() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1085, 1086, 1087, 1141]).await;

    for flank_status in ["active", "completed"] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "entity_tag".to_string(),
            serde_json::json!("MessHall_Guard2"),
        );
        ctx.set_param(
            "mission_681_status".to_string(),
            serde_json::json!("active"),
        );
        ctx.set_param(
            "mission_681_obj_2725_status".to_string(),
            serde_json::json!(flank_status),
        );
        // Pre-increment counter value: the first guard already died.
        ctx.set_param("counter_messhall_kills".to_string(), serde_json::json!(1));
        let event = TriggerEvent {
            trigger_type: TriggerType::EntityDeath,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let actions = actions_of(&engine, 1087, &ctx, &event);

        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::CompleteMission { mission_id: 681 })),
            "with 2725 {flank_status}, the kill counter must still complete \
             681 (D-CB05: track, don't gate); got {actions:?}"
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::AcceptMission { mission_id: 682 })),
            "with 2725 {flank_status}, completing 681 must still accept 682; got {actions:?}"
        );
        assert!(
            actions_of(&engine, 1141, &ctx, &event).is_empty(),
            "the flank chain must never resolve on a death event"
        );
    }
}
