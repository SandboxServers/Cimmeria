//! Inventory event dispatchers: `OnItemUse` (consume / use-on-self
//! actions) and `OnItemEquipped` (item arrived in the bandolier).
//!
//! `fire_item_use` additionally pulls stats into the chain context so
//! conditions like `StatBelowMax` can fizzle the use cleanly when the
//! action would have no effect (e.g., a Health Slappack at full HP).

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::{populate_mission_context, populate_stats_context};

/// Fire `OnItemUse` event when a player uses an inventory item.
///
/// `item_id` is the item design id (type_id) — drives chain matching on
/// `item_use::<type_id>`. `instance_id` is the inventory row id the
/// player clicked — set into the context so `Action::RemoveItem` can
/// consume that exact stack instead of the player's first-by-type
/// instance (which is wrong when the player has multiple stacks of the
/// same item and clicks anything other than the first one).
pub async fn fire_item_use(
    entity_id: u32,
    player_id: i32,
    instance_id: i32,
    item_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("item_id".to_string(), serde_json::json!(item_id));
    ctx.set_param("instance_id".to_string(), serde_json::json!(instance_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        // Stats are needed by `Condition::StatBelowMax` so chains like
        // the Health Slappack (4001) can fizzle silently at full HP
        // instead of burning the stack.
        populate_stats_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemUse,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let matched = !resolved.actions.is_empty();
    if matched {
        tracing::info!(
            entity_id,
            player_id,
            item_id,
            actions = resolved.actions.len(),
            "fire_item_use: matched"
        );
    } else {
        tracing::debug!(entity_id, item_id, "fire_item_use: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire `OnItemEquipped` when an item lands in the bandolier (`container_id = 3`)
/// from another container. Drives chains keyed on `item_equipped::<type_id>` —
/// e.g., the mission 622 / 641 "equip the weapon you just picked up" steps.
pub async fn fire_item_equipped(
    entity_id: u32,
    player_id: i32,
    type_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("item_id".to_string(), serde_json::json!(type_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemEquipped,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            type_id,
            actions = resolved.actions.len(),
            "fire_item_equipped: matched"
        );
    } else {
        tracing::debug!(entity_id, type_id, "fire_item_equipped: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}
