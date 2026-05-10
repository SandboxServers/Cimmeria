//! Region- and teleport-event dispatchers.
//!
//! `fire_enter_region` / `fire_exit_region` are fired by the player movement
//! path when the client crosses a Kismet trigger volume. `fire_teleport_in`
//! is the post-warp arrival hook used by chain 1044 (and any future
//! cross-region completion hook). All three carry the region tag/id as the
//! match key so chain authoring stays declarative.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_mission_context;

/// Fire the `RegionEnter` event when the client enters a Kismet region.
///
/// `region_tag` is the DB `point_sets.name` value (e.g., "Castle_Cellblock.Region2")
/// which doubles as the content engine trigger key. The Python reference fires
/// `client_hinted_region::{tag}` — our content engine matches on `region_key`.
pub async fn fire_enter_region(
    entity_id: u32,
    player_id: i32,
    region_tag: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(region_tag));

    let world_name = space_mgr
        .get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Unknown".to_string());
    ctx.set_param("world_name".to_string(), serde_json::json!(&world_name));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(entity_id, player_id, %region_tag, actions = resolved.actions.len(), "fire_enter_region: matched");
    } else {
        tracing::debug!(entity_id, %region_tag, "fire_enter_region: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire the `RegionExit` event when the client exits a Kismet region.
///
/// See [`fire_enter_region`] for parameter documentation.
pub async fn fire_exit_region(
    entity_id: u32,
    player_id: i32,
    region_tag: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(region_tag));

    let world_name = space_mgr
        .get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Unknown".to_string());
    ctx.set_param("world_name".to_string(), serde_json::json!(&world_name));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionExit,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(entity_id, player_id, %region_tag, actions = resolved.actions.len(), "fire_exit_region: matched");
    } else {
        tracing::debug!(entity_id, %region_tag, "fire_exit_region: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire the `teleport_in` event when a player arrives via a ring transporter.
///
/// Chain 1044 (`teleport_in` event_key=`2`) hooks this to complete mission 640
/// when the player teleports into Castle_CellBlock ring 2. The chain loader
/// converts the SQL `event_key` string into a typed `region_id` field on the
/// trigger, so matching only needs the typed `region_id` param.
pub async fn fire_teleport_in(
    entity_id: u32,
    player_id: i32,
    region_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_id".to_string(), serde_json::json!(region_id));
    // The content engine's `teleport_in` trigger reads `region_id` as i64 (see
    // `Trigger::OnTeleportIn::matches` in crates/content-engine/src/triggers.rs).
    // No `event_key` params are needed — the loader already converts the SQL
    // event_key string into a typed `region_id` field on the trigger.

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            region_id,
            actions = resolved.actions.len(),
            "fire_teleport_in: matched"
        );
    } else {
        tracing::debug!(entity_id, region_id, "fire_teleport_in: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}
