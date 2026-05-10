//! Interaction event dispatchers: tag- and template-keyed `OnInteract`
//! hooks fired by the player's `useEntity` path.
//!
//! Both return `bool` so the caller in `cell_methods::player::interaction`
//! can decide whether to fall through to the default
//! (Python-`onUseEntity`-equivalent) handling when no chain matches.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_mission_context;

/// Fire `OnInteractTag` event when a player interacts with a tagged entity.
///
/// Returns `true` if a content chain handled the interaction (caller should
/// NOT fall through to the default `handle_interact()` logic).
pub async fn fire_interact_tag(
    entity_id: u32,
    player_id: i32,
    tag: &str,
    target_entity_id: u32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx.set_param(
        "target_entity_id".to_string(),
        serde_json::json!(target_entity_id),
    );

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: Some(cimmeria_common::EntityId(target_entity_id as i32)),
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let matched = !resolved.actions.is_empty();
    if matched {
        tracing::info!(entity_id, player_id, %tag, actions = resolved.actions.len(), "fire_interact_tag: matched");
        executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
    } else {
        tracing::debug!(entity_id, %tag, "fire_interact_tag: no chains matched");
    }
    matched
}

/// Fire `OnInteractTemplate` event when a player interacts with a templated entity.
///
/// Returns `true` if a content chain handled the interaction.
pub async fn fire_interact_template(
    entity_id: u32,
    player_id: i32,
    template_name: &str,
    target_entity_id: u32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param(
        "template_name".to_string(),
        serde_json::json!(template_name),
    );
    ctx.set_param(
        "target_entity_id".to_string(),
        serde_json::json!(target_entity_id),
    );

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTemplate,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: Some(cimmeria_common::EntityId(target_entity_id as i32)),
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let matched = !resolved.actions.is_empty();
    if matched {
        tracing::info!(entity_id, player_id, %template_name, actions = resolved.actions.len(), "fire_interact_template: matched");
        executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
    } else {
        tracing::debug!(entity_id, %template_name, "fire_interact_template: no chains matched");
    }
    matched
}
