//! Inventory event dispatchers: `OnItemUse` (consume / use-on-self
//! actions) and `OnItemEquipped` (item arrived in the bandolier).
//!
//! `fire_item_use` first offers the use to the native consumable path
//! (`consumable_use`: items whose `items_event_sets` event-5 ability is a
//! heal or a stat buff, such as the Health Slappack); only an item that
//! path does not own reaches the chains. It also pulls stats into the chain
//! context so conditions like `StatBelowMax` can gate a chain on headroom.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::{
    populate_mission_context, populate_stats_context, populate_world_context,
};

/// Fire `OnItemUse` event when a player uses an inventory item.
///
/// A native consumable (see `consumable_use`) is handled there and fires no
/// chain: it is refused with feedback, or its unit is consumed and its
/// ability applied once the base confirms. Everything else runs chains as
/// before.
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
    if super::super::consumable_use::try_native_use(
        entity_id,
        player_id,
        instance_id,
        item_id,
        engine,
        tx,
        space_mgr,
    )
    .await
    {
        return;
    }

    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("item_id".to_string(), serde_json::json!(item_id));
    ctx.set_param("instance_id".to_string(), serde_json::json!(instance_id));

    populate_world_context(entity_id, space_mgr, &mut ctx);
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        // Stats are needed by `Condition::StatBelowMax`, so a heal chain
        // can gate on headroom instead of burning the stack at full HP.
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
        crate::cell::playtest_friction::item_use_no_chain(entity_id, item_id);
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

    populate_world_context(entity_id, space_mgr, &mut ctx);
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
