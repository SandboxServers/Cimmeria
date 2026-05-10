//! Player-lifecycle event dispatchers: world entry and entity death.
//!
//! Both fire on whole-entity transitions rather than on a specific
//! interaction with a target — `PlayerLoaded` runs once per world enter
//! (after the BaseApp has restored the player's persisted state), and
//! `EntityDeath` runs when an NPC's `health.cur` crosses to zero. Sibling
//! to the more granular interaction/region/dialog/inventory dispatchers.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_mission_context;

/// Fire the `PlayerLoaded` event for a player entering a world.
pub async fn fire_player_loaded(
    entity_id: u32,
    player_id: i32,
    world_name: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));

    ctx.set_param("world_name".to_string(), serde_json::json!(world_name));

    // Populate mission/step/archetype context from entity state
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id, player_id, %world_name,
            actions = resolved.actions.len(),
            "fire_player_loaded: matched"
        );
    } else {
        tracing::debug!(entity_id, %world_name, "fire_player_loaded: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire the `EntityDeath` event when an NPC is killed.
///
/// This triggers content chains that track kill counts for mission progression
/// (e.g., chains 1085-1086: kill Hallway01_Guard → increment counter → complete mission 681).
pub async fn fire_entity_death(
    killer_entity_id: u32,
    player_id: i32,
    entity_tag: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx =
        ExecutionContext::new().with_source(cimmeria_common::EntityId(killer_entity_id as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(entity_tag));

    if let Some(entity) = space_mgr.get_entity(killer_entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: Some(cimmeria_common::EntityId(killer_entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            killer = killer_entity_id, player_id, %entity_tag,
            actions = resolved.actions.len(), "fire_entity_death: matched"
        );
    }
    executor::execute_actions(resolved, killer_entity_id, player_id, tx, space_mgr, engine).await;
}
