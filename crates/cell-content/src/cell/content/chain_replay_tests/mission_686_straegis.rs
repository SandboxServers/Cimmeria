//! Mission 686 aftermath — Straegis attack scene (C08b). Pins chain 1161
//! (the Matinee/despawn/dialog sequence on `mission_completed 686`, with
//! its exact `delay_ms` staggering from C08a) and chain 1162 (the
//! player_loaded relog-restore gate that re-despawns Col Marsh so he
//! doesn't respawn from `resources.spawnlist` into a fresh
//! Castle_CellBlock instance after 686 has completed but before 687 is
//! accepted).

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1161: completing mission 686 must resolve exactly the three
/// actions in this packet's scope, each with its exact `delay_ms` —
/// `play_sequence 1751` and `destroy_entity Preparation_ColMarsh`
/// immediately (`delay_ms = 0`), `display_dialog 2516` 10.1s later
/// (`delay_ms = 10100`, right after the ~10.0096s Matinee finishes).
/// Order matters too: sort_order 0/1/2 must resolve in that order so a
/// regression that reordered the seed rows (e.g. showing the dialog
/// before the despawn) would trip this.
#[tokio::test]
async fn chain_1161_resolves_straegis_scene_with_exact_delays() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1161)
        .await
        .expect("DB query for chain 1161 must succeed")
        .expect("chain 1161 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(686));

    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_actions: Vec<(&Action, i32)> = resolved
        .actions
        .iter()
        .zip(resolved.action_delays.iter())
        .filter_map(|((id, action), delay)| {
            if *id == 1161 {
                Some((action, *delay))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        chain_actions.len(),
        3,
        "chain 1161 must resolve exactly 3 actions on mission_completed 686; got {:?}",
        resolved.actions,
    );

    match &chain_actions[0] {
        (Action::PlaySequence { sequence_id: 1751 }, 0) => {}
        other => panic!("action 0 must be PlaySequence(1751) with delay_ms=0; got {other:?}"),
    }
    match &chain_actions[1] {
        (Action::DestroyTaggedEntity { entity_tag }, 0) if entity_tag == "Preparation_ColMarsh" => {}
        other => panic!(
            "action 1 must be DestroyTaggedEntity(Preparation_ColMarsh) with delay_ms=0; got {other:?}"
        ),
    }
    match &chain_actions[2] {
        (Action::DisplayDialog { dialog_id: 2516 }, 10100) => {}
        other => panic!("action 2 must be DisplayDialog(2516) with delay_ms=10100; got {other:?}"),
    }
}

/// Chain 1161 negative: a different mission completing must not resolve
/// this chain — pins the `mission_completed` event_key match against
/// '686' specifically, not any mission.
#[tokio::test]
async fn chain_1161_does_not_resolve_on_other_mission_completion() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1161)
        .await
        .expect("DB query for chain 1161 must succeed")
        .expect("chain 1161 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(685));

    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1161)
        .count();
    assert_eq!(
        n, 0,
        "chain 1161 must NOT resolve on mission_completed 685; got {n} actions",
    );
}

/// Chain 1162: a player who relogs (or re-enters the zone, creating a
/// fresh instance) after 686 has completed but before 687 has been
/// accepted must re-despawn Marsh — the fresh instance would otherwise
/// respawn him from `resources.spawnlist` at his Preparation-room
/// position, undoing chain 1161's one-time despawn.
#[tokio::test]
async fn chain_1162_redespawns_marsh_when_686_done_and_687_not_started() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1162)
        .await
        .expect("DB query for chain 1162 must succeed")
        .expect("chain 1162 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_686_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("not_active"),
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
            *id == 1162
                && matches!(
                    action,
                    Action::DestroyTaggedEntity { entity_tag } if entity_tag == "Preparation_ColMarsh"
                )
        })
        .count();
    assert_eq!(
        n, 1,
        "chain 1162 must resolve DestroyTaggedEntity(Preparation_ColMarsh) on login \
         when 686 is completed and 687 is not_active; got {n}",
    );
}

/// Chain 1162 negative: once mission 687 has been accepted, Marsh must
/// NOT be re-despawned on login — a future escort packet (GC1, not yet
/// landed) owns what happens to him from that point on. Pins the
/// `mission_687_status eq not_active` gate.
#[tokio::test]
async fn chain_1162_does_not_redespawn_marsh_once_687_accepted() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1162)
        .await
        .expect("DB query for chain 1162 must succeed")
        .expect("chain 1162 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_686_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
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
        .filter(|(id, _)| *id == 1162)
        .count();
    assert_eq!(
        n, 0,
        "chain 1162 must NOT resolve once mission 687 is active; got {n} actions",
    );
}

/// Chain 1162 negative: before mission 686 has completed (e.g. still
/// mid-Hallway05), a login must NOT despawn Marsh either — he hasn't
/// been removed yet, so there's nothing to restore.
#[tokio::test]
async fn chain_1162_does_not_redespawn_marsh_before_686_completes() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1162)
        .await
        .expect("DB query for chain 1162 must succeed")
        .expect("chain 1162 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_686_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("not_active"),
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
        .filter(|(id, _)| *id == 1162)
        .count();
    assert_eq!(
        n, 0,
        "chain 1162 must NOT resolve before mission 686 completes; got {n} actions",
    );
}
